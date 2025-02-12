use std::sync::Arc;

use anyhow::{anyhow, Context};
use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts, State}, http::{request::Parts, StatusCode}
};
use dashmap::DashMap;
use thiserror::Error;
use tracing::{debug, error, instrument, trace, warn};
use uuid::Uuid;

use crate::{ApiError, ApiResult, AppState, TIMEOUT, USER_AGENT};

use super::types::*;

// It's an extractor that pulls a token from the Header.
#[derive(PartialEq, Debug)]
pub struct Token(pub String);

impl Token {
    pub async fn _check_auth(self, state: &AppState) -> ApiResult<()> {
        if let Some(user) = state.user_manager.get(&self.0) {
            if !user.banned {
                Ok(())
            } else {
                Err(ApiError::Unauthorized)
            }
        } else {
            Err(ApiError::Unauthorized)
        }
    }
}

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
            Some(token) => Ok(Self(token.to_string())),
            None => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

impl<S> OptionalFromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = StatusCode; // Not required

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        let token = parts
            .headers
            .get("token")
            .and_then(|value| value.to_str().ok());
        trace!(token = ?token);
        Ok(token.map(|t| Self(t.to_string())))
    }
}
// End Extractor

// Work with external APIs
/// Get UUID from JSON response
fn get_id_json(json: &serde_json::Value) -> Result<Uuid, uuid::Error> {
    trace!("json: {json:#?}"); // For debugging, we'll get to this later!
    let uuid = Uuid::parse_str(json.get("id").unwrap().as_str().unwrap())?;
    Ok(uuid)
}

#[derive(Debug, Error)]
enum FetchError {
    #[error("invalid response code (expected 200), found {0}.\n    Response: {1:#?}")]
    WrongResponse(u16, Result<String, reqwest::Error>),
    #[error(transparent)]
    SendError(#[from] reqwest::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),

}

async fn fetch_json(
    auth_provider: &AuthProvider,
    server_id: &str,
    username: &str,
) -> Result<(Uuid, AuthProvider), FetchError> {
    let client = reqwest::Client::builder().timeout(TIMEOUT).user_agent(USER_AGENT).build().unwrap();
    let url = auth_provider.url.clone();

    let res = client
        .get(url)
        .query(&[("serverId", server_id), ("username", username)])
        .send()
        .await?;
    trace!("{res:?}");
    match res.status().as_u16() {
        200 => {
            let json = serde_json::from_str::<serde_json::Value>(&res.text().await?).with_context(|| "Cant deserialize".to_string())?;
            let uuid = get_id_json(&json).with_context(|| "Cant get UUID".to_string())?;
            Ok((uuid, auth_provider.clone()))
        }
        _ => Err(FetchError::WrongResponse(res.status().as_u16(), res.text().await)),
    }
}

pub async fn has_joined(
    AuthProviders(authproviders): AuthProviders,
    server_id: &str,
    username: &str,
) -> anyhow::Result<Option<(Uuid, AuthProvider)>> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    for provider in &authproviders {
        tokio::spawn(fetch_and_send(
            provider.clone(),
            server_id.to_string(),
            username.to_string(),
            tx.clone()
        ));
    } 
    let mut errors = Vec::new(); // Counting fetches what returns errors
    let mut misses = Vec::new(); // Counting non OK results
    let mut prov_count: usize = authproviders.len();
    while prov_count > 0 {
        if let Some(fetch_res) = rx.recv().await {
            match fetch_res {
                Ok(data) => return Ok(Some(data)),
                Err(err) => {
                    match err {
                        FetchError::WrongResponse(code, data) => misses.push((code, data)),
                        FetchError::SendError(err) => errors.push(err.to_string()),
                        FetchError::Other(err) => errors.push(err.to_string()),
                    }
                },
            }
        } else {
            error!("Unexpected behavior!");
            return Err(anyhow!("Something went wrong..."))
        }
        prov_count -= 1;
    }

    // Choosing what error return

    // Returns if some internals errors occured
    if !errors.is_empty() {
        error!("Something wrong with your authentification providers!\nMisses: {misses:?}\nErrors: {errors:?}");
        Err(anyhow::anyhow!("{:?}", errors))
        
    } else {
        // Returning if user can't be authenticated
        debug!("Misses: {misses:?}");
        Ok(None)
    }
}

async fn fetch_and_send(
    provider: AuthProvider,
    server_id: String,
    username: String,
    tx: tokio::sync::mpsc::Sender<Result<(Uuid, AuthProvider), FetchError>>
) {
    let _ = tx.send(fetch_json(&provider, &server_id, &username).await)
        .await.map_err( |err| trace!("fetch_and_send error [note: ok res returned and mpsc clossed]: {err:?}"));
}

// User manager
#[derive(Debug, Clone)]
pub struct UManager {
    /// Users with incomplete authentication
    pending: Arc<DashMap<String, String>>, // <SHA1 serverId, USERNAME> TODO: Add automatic purge
    /// Authenticated users TODO: Change name to sessions
    authenticated: Arc<DashMap<String, Uuid>>, // <SHA1 serverId, Userinfo>
    /// Registered users
    registered: Arc<DashMap<Uuid, Userinfo>>,
}

impl UManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            registered: Arc::new(DashMap::new()),
            authenticated: Arc::new(DashMap::new()),
        }
    }
    pub fn get_all_registered(&self) -> DashMap<Uuid, Userinfo> {
        self.registered.as_ref().clone()
    }
    pub fn get_all_authenticated(&self) -> DashMap<String, Uuid> {
        self.authenticated.as_ref().clone()
    }
    pub fn pending_insert(&self, server_id: String, username: String) {
        self.pending.insert(server_id, username);
    }
    pub fn pending_remove(&self, server_id: &str) -> Option<(String, String)> {
        self.pending.remove(server_id)
    }
    pub fn insert(&self, uuid: Uuid, token: String, userinfo: Userinfo) -> Result<(), ()> {
        // Check for the presence of an active session.
        if let Some(userinfo) = self.registered.get(&uuid) {
            if let Some(token) = &userinfo.token {
                if self.authenticated.contains_key(token) {
                    warn!("Rejected attempt to create a second session for the same user!");
                    return Err(())
                }
                debug!("`{}` already have token in registered profile (old token already removed from 'authenticated')", userinfo.nickname);
            }
        }

        // Adding a user
        self.authenticated.insert(token, uuid);
        self.insert_user(uuid, userinfo);
        Ok(())
    }
    pub fn insert_user(&self, uuid: Uuid, userinfo: Userinfo) {
        // self.registered.insert(uuid, userinfo)
        let usercopy = userinfo.clone();
        self.registered.entry(uuid)
            .and_modify(|exist| {
                if !userinfo.nickname.is_empty() { exist.nickname = userinfo.nickname };
                if !userinfo.auth_provider.is_empty() { exist.auth_provider = userinfo.auth_provider };
                if userinfo.rank != Userinfo::default().rank { exist.rank = userinfo.rank };
                if userinfo.token.is_some() { exist.token = userinfo.token };
                if userinfo.version != Userinfo::default().version { exist.version = userinfo.version };
            }).or_insert(usercopy);
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
    pub fn ban(&self, banned_user: &Userinfo) {
        self.registered.entry(banned_user.uuid)
            .and_modify(|exist| {
                exist.banned = true;
            }).or_insert(banned_user.clone());
    }
    pub fn unban(&self, uuid: &Uuid) {
        if let Some(mut user) = self.registered.get_mut(uuid) {
            user.banned = false;
        };
    }
    pub fn _is_authenticated(&self, token: &String) -> bool {
        self.authenticated.contains_key(token)
    }
    pub fn _is_registered(&self, uuid: &Uuid) -> bool {
        self.registered.contains_key(uuid)
    }
    pub fn is_banned(&self, uuid: &Uuid) -> bool {
        if let Some(user) = self.registered.get(uuid) { user.banned } else { false }
    }
    pub fn count_authenticated(&self) -> usize {
        self.authenticated.len()
    }
    pub fn remove(&self, uuid: &Uuid) {
        let token = self.registered.get(uuid).unwrap().token.clone().unwrap();
        self.authenticated.remove(&token);
    }
}
// End of User manager

#[axum::debug_handler]
#[instrument(skip_all)]
pub async fn check_auth(
    token: Option<Token>,
    State(state): State<AppState>,
) -> ApiResult<&'static str> {
    match token {
        Some(token) => {
            match state.user_manager.get(&token.0) {
                Some(user) => {
                    if user.banned {
                        debug!(nickname = user.nickname, status = "banned", "Token owner is banned");
                        Err(ApiError::Unauthorized)
                    } else {
                        debug!(nickname = user.nickname, status = "ok", "Token verified successfully");
                        Ok("ok")
                    }
                }
                None => {
                    debug!(token = token.0, status = "invalid", "Invalid token");
                    Err(ApiError::Unauthorized)
                }
            }
        }
        None => {
            debug!(status = "not provided", "Token not provided");
            Err(ApiError::BadRequest)
        }
    }
}