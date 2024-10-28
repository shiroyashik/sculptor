use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Userinfo {
    pub uuid: Uuid,
    pub username: String,
    pub rank: String,
    pub last_used: String,
    pub auth_provider: AuthProvider,
    pub token: Option<String>,
    pub version: String,
    pub banned: bool
    
}

impl Default for Userinfo {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            username: Default::default(),
            rank: "default".to_string(),
            last_used: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            auth_provider: Default::default(),
            token: Default::default(),
            version: "0.1.4+1.20.1".to_string(),
            banned: false
        }
    }
}

// new part

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProvider {
    pub name: String,
    pub url: String,
}

impl Default for AuthProvider {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            url: Default::default()
        }
    }
}

impl AuthProvider {
    pub fn is_empty(&self) -> bool {
        self.name == "Unknown"
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProviders(pub Vec<AuthProvider>);

pub fn default_authproviders() -> AuthProviders {
    AuthProviders(vec![
        AuthProvider { name: "Mojang".to_string(), url: "https://sessionserver.mojang.com/session/minecraft/hasJoined".to_string() },
        AuthProvider { name: "ElyBy".to_string(), url: "http://minecraft.ely.by/session/hasJoined".to_string() }
        ])
}