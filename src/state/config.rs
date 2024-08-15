use std::{collections::HashMap, io::Read, path::PathBuf};

use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::auth::{default_authproviders, AuthProviders, Userinfo};

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub token: Option<String>,
    pub motd: CMotd,
    #[serde(default = "default_authproviders")]
    pub auth_providers: AuthProviders,
    pub limitations: Limitations,
    #[serde(default)]
    pub mc_folder: PathBuf,
    #[serde(default)]
    pub advanced_users: HashMap<Uuid, AdvancedUsers>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CMotd {
    pub display_server_info: bool,
    pub custom_text: String,
    #[serde(rename = "sInfoUptime")]
    pub text_uptime: String,
    #[serde(rename = "sInfoAuthClients")]
    pub text_authclients: String,
    #[serde(rename = "sInfoDrawIndent")]
    pub draw_indent: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Limitations {
    pub max_avatar_size: u64,
    pub max_avatars: u64,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedUsers {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub banned: bool,
    #[serde(default)]
    pub special: [u8;6],
    #[serde(default)]
    pub pride: [u8;25],
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BannedPlayer {
    pub uuid: Uuid,
    pub name: String,
}

impl Into<Userinfo> for BannedPlayer {
    fn into(self) -> Userinfo {
        Userinfo {
            uuid: self.uuid,
            username: self.name,
            banned: true,
            ..Default::default()
        }
    }
}

impl Config {
    pub fn parse(path: PathBuf) -> Self {
        let mut file = std::fs::File::open(path).expect("Access denied or file doesn't exists!");
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        toml::from_str(&data).unwrap()
    }

    pub fn verify_token(&self, suspicious: &str) -> crate::ApiResult<()> {
        use crate::ApiError;
        match &self.token {
            Some(token) => {
                if token == suspicious {
                    debug!("Admin token passed!");
                    Ok(())
                } else {
                    warn!("Unknown tryed to use admin functions, but use wrong token!");
                    Err(ApiError::Unauthorized)
                }
            },
            None => {
                warn!("Unknown tryed to use admin functions, but token is not defined!");
                Err(ApiError::BadRequest)
            },
        }
    }
}