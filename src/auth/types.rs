use std::str::FromStr;

use serde::Deserialize;
use uuid::Uuid;
use anyhow::anyhow;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Userinfo {
    pub username: String,
    pub uuid: Uuid,
    pub auth_system: AuthSystem,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    pub(super) fn get_url(&self) -> String {
        match self {
            AuthSystem::Internal => panic!("Can't get internal URL!"),
            AuthSystem::ElyBy => String::from("http://minecraft.ely.by/session/hasJoined"),
            AuthSystem::Mojang => String::from("https://sessionserver.mojang.com/session/minecraft/hasJoined"),
        }
    }
}

