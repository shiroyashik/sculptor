use std::{str::FromStr, sync::Arc};

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
use dashmap::DashMap;
use ring::digest::{self, digest};
use serde::Deserialize;
use tracing::{debug, error, info, trace};
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
        hex::encode(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0..20]);
    let state = state.user_manager;
    state.pending_insert(server_id.clone(), query.username);
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
) -> Response {
    let server_id = query.id.clone();
    let username = state.user_manager.pending_remove(&server_id).unwrap().1; // TODO: Add error check
    let userinfo = match has_joined(&server_id, &username).await {
        Ok(d) => d,
        Err(e) => {
            error!("[Authentication] {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal verify error".to_string()).into_response();
        },
    };
    if let Some((uuid, auth_system)) = userinfo {
        info!("[Authentication] {username} logged in using {auth_system:?}");
        let authenticated = state.user_manager;
        authenticated.insert(
            uuid,
            server_id.clone(),
            Userinfo {
                username,
                uuid,
                auth_system,
                token: Some(server_id.clone()),
            },
        );
        (StatusCode::OK, server_id.to_string()).into_response()
    } else {
        info!("[Authentication] failed to verify {username}");
        (StatusCode::BAD_REQUEST, "failed to verify".to_string()).into_response()
    }
}

pub async fn status(Token(token): Token, State(state): State<AppState>) -> Response {
    match token {
        Some(token) => {
            if state.user_manager.is_authenticated(&token) {
                (StatusCode::OK, "ok".to_string()).into_response()
            } else {

                (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response()
            }
        }
        None => {
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AuthSystem {
    Internal,
    ElyBy,
    Mojang,
}

impl ToString for AuthSystem {
    fn to_string(&self) -> String {
        match self {
            AuthSystem::Internal => String::from("internal"),
            AuthSystem::ElyBy => String::from("elyby"),
            AuthSystem::Mojang => String::from("mojang"),
        }
    }
}

impl FromStr for AuthSystem {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "internal" => Ok(Self::Internal),
            "elyby" => Ok(Self::ElyBy),
            "mojang" => Ok(Self::Mojang),
            _ => Err(anyhow!("No auth system called: {s}"))
        }
    }
}

impl AuthSystem {
    fn get_url(&self) -> String {
        match self {
            AuthSystem::Internal => panic!("Can't get internal URL!"),
            AuthSystem::ElyBy => String::from("http://minecraft.ely.by/session/hasJoined"),
            AuthSystem::Mojang => String::from("https://sessionserver.mojang.com/session/minecraft/hasJoined"),
        }
    }
}

// Work with external APIs
/// Get UUID from JSON response
#[inline]
fn get_id_json(json: &serde_json::Value) -> anyhow::Result<Uuid> {
    trace!("json: {json:#?}"); // For debugging, we'll get to this later!
    let uuid = Uuid::parse_str(json.get("id").unwrap().as_str().unwrap())?;
    Ok(uuid)
}

#[inline]
async fn fetch_json(
    auth_system: AuthSystem,
    server_id: &str,
    username: &str,
) -> anyhow::Result<Option<(Uuid, AuthSystem)>> {
    let client = reqwest::Client::new();
    let url = auth_system.get_url();

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
        401 => Ok(None), // Ely.By None
        204 => Ok(None), // Mojang None
        _ => Err(anyhow!("Unknown code: {}", res.status().as_u16())),
    }
}

pub async fn has_joined(
    server_id: &str,
    username: &str,
) -> anyhow::Result<Option<(Uuid, AuthSystem)>> {
    let (elyby, mojang) = (
        fetch_json(AuthSystem::ElyBy,server_id, username).await?,
        fetch_json(AuthSystem::Mojang, server_id, username).await?
    );

    if elyby.is_none() && mojang.is_none() {
        Ok(None)
    } else if mojang.is_some() {
        Ok(mojang)
    } else if elyby.is_some() {
        Ok(elyby)
    } else {
        panic!("Impossible error!")
    }
}
// End of work with external APIs

// User manager
#[derive(Debug, Clone)]
pub struct UManager {
    /// Users with incomplete authentication
    pending: Arc<DashMap<String, String>>, // <SHA1 serverId, USERNAME> TODO: Add automatic purge
    /// Authenticated users
    authenticated: Arc<DashMap<String, Uuid>>, // <SHA1 serverId, Userinfo> NOTE: In the future, try it in a separate LockRw branch
    /// Registered users
    registered: Arc<DashMap<Uuid, Userinfo>>,
}

#[derive(Debug, Clone)]
pub struct Userinfo {
    pub username: String,
    pub uuid: Uuid,
    pub auth_system: AuthSystem,
    pub token: Option<String>,
}

impl UManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            registered: Arc::new(DashMap::new()),
            authenticated: Arc::new(DashMap::new()),
        }
    }
    pub fn pending_insert(&self, server_id: String, username: String) {
        self.pending.insert(server_id, username);
    }
    pub fn pending_remove(&self, server_id: &str) -> std::option::Option<(std::string::String, std::string::String)> {
        self.pending.remove(server_id)
    }
    pub fn insert(&self, uuid: Uuid, token: String, userinfo: Userinfo) -> Option<Userinfo> {
        self.authenticated.insert(token, uuid);
        self.registered.insert(uuid, userinfo)
    }
    pub fn insert_user(&self, uuid: Uuid, userinfo: Userinfo) -> Option<Userinfo> {
        self.registered.insert(uuid, userinfo)
    }
    pub fn get(
        &self,
        token: &String,
    ) -> Option<dashmap::mapref::one::Ref<'_, Uuid, Userinfo>> {
        let uuid = self.authenticated.get(token)?;
        self.registered.get(uuid.value())
    }
    pub fn get_by_uuid(
        &self,
        uuid: &Uuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, Uuid, Userinfo>> {
        self.registered.get(uuid)
    }
    pub fn is_authenticated(&self, token: &String) -> bool {
        self.authenticated.contains_key(token)
    }
    pub fn is_registered(&self, uuid: &Uuid) -> bool {
        self.registered.contains_key(uuid)
    }
    pub fn remove(&self, uuid: &Uuid) {
        let token = self.registered.remove(uuid).unwrap().1.token.unwrap();
        self.authenticated.remove(&token);
    }
}
// End of User manager