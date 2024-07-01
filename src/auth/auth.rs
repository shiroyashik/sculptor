use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    async_trait, extract::FromRequestParts, http::{request::Parts, StatusCode}
};
use dashmap::DashMap;
use tracing::{debug, trace};
use uuid::Uuid;

use super::types::*;

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
    /// Authenticated users TODO: Change name to sessions
    authenticated: Arc<DashMap<String, Uuid>>, // <SHA1 serverId, Userinfo> NOTE: In the future, try it in a separate LockRw branch
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
    pub fn _is_authenticated(&self, token: &String) -> bool {
        self.authenticated.contains_key(token)
    }
    pub fn _is_registered(&self, uuid: &Uuid) -> bool {
        self.registered.contains_key(uuid)
    }
    pub fn remove(&self, uuid: &Uuid) {
        let token = self.registered.remove(uuid).unwrap().1.token.unwrap();
        self.authenticated.remove(&token);
    }
}
// End of User manager