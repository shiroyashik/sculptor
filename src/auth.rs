use crate::utils::*;
use anyhow::anyhow;
use axum::{
    async_trait, debug_handler,
    extract::{FromRequestParts, Query, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use ring::digest::{self, digest};
use serde::Deserialize;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}

// Web
#[derive(Deserialize)]
struct Id {
    username: String,
}

#[debug_handler]
async fn id(
    // First stage of authentication
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id =
        bytes_into_string(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0..20]);
    let state = state.pending;
    state.insert(server_id.clone(), query.username);
    server_id
}

#[derive(Deserialize)]
struct Verify {
    id: String,
}

#[debug_handler]
async fn verify(
    // Second stage of authentication
    Query(query): Query<Verify>,
    State(state): State<AppState>,
) -> String {
    let server_id = query.id.clone();
    let username = state.pending.remove(&server_id).unwrap().1; // TODO: Add 
    if let Some((uuid, auth_system)) = has_joined(&server_id, &username).await.unwrap() {
        info!("[Authorization] {username} logged in using {auth_system:?}");
        let authenticated = state.authenticated;
        authenticated.insert(
            uuid,
            server_id.clone(),
            crate::Userinfo {
                username,
                uuid,
                auth_system,
            },
        );
        server_id.to_string()
    } else {
        String::from("failed to verify")
    }
}

pub async fn status(Token(token): Token, State(state): State<AppState>) -> Response {
    match token {
        Some(token) => {
            if state.authenticated.contains_token(&token) {
                // format!("ok") // 200
                (StatusCode::OK, "ok".to_string()).into_response()
            } else {
                // format!("unauthorized") // 401
                (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response()
            }
        }
        None => {
            // format!("bad request") // 400
            (StatusCode::BAD_REQUEST, "bad request".to_string()).into_response()
        }
    }
}
// Web End

// It's an extractor that pulls a token from the Header.
#[derive(PartialEq, Debug)]
pub struct Token(pub Option<String>);

#[async_trait]
impl<S> FromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("token")
            .and_then(|value| value.to_str().ok());
        trace!(token = ?token);
        match token {
            Some(token) => Ok(Self(Some(token.to_string()))),
            None => Ok(Self(None)),
        }
    }
}
// End Extractor

// Work with external APIs

#[derive(Debug, Clone)]
pub enum AuthSystem {
    ElyBy,
    Mojang,
}

impl ToString for AuthSystem {
    fn to_string(&self) -> String {
        match self {
            AuthSystem::ElyBy => String::from("elyby"),
            AuthSystem::Mojang => String::from("mojang"),
        }
    }
}

/// Get UUID from JSON response
#[inline]
fn get_id_json(json: &serde_json::Value) -> anyhow::Result<Uuid> {
    trace!("json: {json:#?}"); // For debugging, we'll get to this later!
    let uuid = Uuid::parse_str(json.get("id").unwrap().as_str().unwrap())?;
    Ok(uuid)
}

#[inline]
async fn fetch_json(
    url: &str,
    server_id: &str,
    username: &str,
) -> anyhow::Result<Option<(Uuid, AuthSystem)>> {
    let client = reqwest::Client::new();
    let auth_system = if url.contains("https://sessionserver.mojang.com") {
        AuthSystem::Mojang
    } else if url.contains("http://minecraft.ely.by") {
        AuthSystem::ElyBy
    } else {
        return Err(anyhow!("Unknown auth system"));
    };

    let res = client
        .get(url)
        .query(&[("serverId", server_id), ("username", username)])
        .send()
        .await?;
    debug!("{res:?}");
    match res.status().as_u16() {
        200 => {
            let json = serde_json::from_str::<serde_json::Value>(&res.text().await?)?;
            let uuid = get_id_json(&json)?;
            Ok(Some((uuid, auth_system)))
        }
        401 => Ok(None),
        _ => Err(anyhow!("Unknown code: {}", res.status().as_u16())),
    }
}

pub async fn has_joined(
    server_id: &str,
    username: &str,
) -> anyhow::Result<Option<(Uuid, AuthSystem)>> {
    tokio::select! {
        Ok(Some(res)) = fetch_json("http://minecraft.ely.by/session/hasJoined", server_id, username) => {Ok(Some(res))},
        Ok(Some(res)) = fetch_json("https://sessionserver.mojang.com/session/minecraft/hasJoined", server_id, username) => {Ok(Some(res))},
        else => {Err(anyhow!("Something went wrong in external apis request process"))}
    }
}
// End of work with external APIs
