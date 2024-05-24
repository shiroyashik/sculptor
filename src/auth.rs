use anyhow::anyhow;
use axum::{async_trait, debug_handler, extract::{FromRequestParts, Query, State}, http::{request::Parts, StatusCode}, response::{IntoResponse, Response}, routing::get, Router};
use log::{debug, info, trace};
use serde::Deserialize;
use ring::digest::{self, digest};
use uuid::Uuid;
use crate::utils::*;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}


// Web
#[derive(Deserialize)]
struct Id {username: String}

#[debug_handler]
async fn id( // First stage of authentication
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id = bytes_into_string(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0 .. 20]);
    let state = state.pending;
    state.insert(server_id.clone(), query.username);
    server_id
}

#[derive(Deserialize)]
struct Verify {id: String}

#[debug_handler]
async fn verify( // Second stage of authentication
    Query(query): Query<Verify>,
    State(state): State<AppState>,
) -> String {
    let server_id = query.id.clone();
    let username = state.pending.remove(&server_id).unwrap().1;
    if let Some((uuid, auth_system)) = has_joined(&server_id, &username).await.unwrap() {
        info!("[Authorization] {username} logged in using {auth_system:?}");
        let authenticated = state.authenticated;
        // let link = state.authenticated_link.lock().await; // // Реализация поиска пользователя в HashMap по UUID
        authenticated.insert(uuid, server_id.clone(), crate::Userinfo { username, uuid, auth_system });
        // link.insert(uuid, crate::AuthenticatedLink(server_id.clone())); // Реализация поиска пользователя в HashMap по UUID
        return format!("{server_id}")
    } else {
        return String::from("failed to verify")
    }
}

pub async fn status(
    Token(token): Token,
    State(state): State<AppState>,
) -> Response {
    match token {
        Some(token) => {
            if state.authenticated.contains_token(&token) {
                // format!("ok") // 200
                (StatusCode::OK, format!("ok")).into_response()
            } else {
                // format!("unauthorized") // 401
                (StatusCode::UNAUTHORIZED, format!("unauthorized")).into_response()
            }
        },
        None => {
            // format!("bad request") // 400
            (StatusCode::BAD_REQUEST, format!("bad request")).into_response()
        },
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
        trace!("[Extractor Token] Data: {token:?}");
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

pub async fn has_joined(server_id: &str, username: &str) -> anyhow::Result<Option<(Uuid, AuthSystem)>> {
    let client = reqwest::Client::new();
    tokio::select! {
        Ok(Some(res)) = async {
            let res = client.clone().get(
                format!("http://minecraft.ely.by/session/hasJoined?serverId={server_id}&username={username}")).send().await?;
            debug!("{res:?}");
            match res.status().as_u16() {
                200 => {
                    let json = serde_json::from_str::<serde_json::Value>(&res.text().await?)?;
                    let uuid = Uuid::parse_str(json["id"].as_str().unwrap())?;
                    Ok(Some((uuid, AuthSystem::ElyBy)))
                },
                401 => Ok(None),
                _ => Err(anyhow::anyhow!("Unknown code: {}", res.status().as_u16()))
            }
        } => {Ok(Some(res))}
        Ok(Some(res)) = async {
            let res = client.clone().get(
                format!("https://sessionserver.mojang.com/session/minecraft/hasJoined?serverId={server_id}&username={username}")).send().await?;
            debug!("{res:?}");
            match res.status().as_u16() {
                200 => {
                    let json = serde_json::from_str::<serde_json::Value>(&res.text().await?)?;
                    let uuid = Uuid::parse_str(json["id"].as_str().unwrap())?;
                    Ok(Some((uuid, AuthSystem::Mojang)))
                },
                204 => Ok(None),
                _ => Err(anyhow::anyhow!("Unknown code: {}", res.status().as_u16()))
            }
        } => {Ok(Some(res))}
        else => {Err(anyhow!("Something went wrong in external apis request process"))}
    }
}
// End of work with external APIs