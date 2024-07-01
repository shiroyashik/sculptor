use std::{io::Read, path::PathBuf};

use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub token: Option<String>,
    pub motd: String,
    pub limitations: Limitations,
    pub advanced_users: Table,
}

impl Config {
    pub fn verify_token(&self, suspicious: &Option<String>) -> Result<axum::response::Response, axum::response::Response> {
        use axum::{http::StatusCode, response::IntoResponse};
        match &self.token {
            Some(token) => {
                match suspicious {
                    Some(suspicious) => {
                        if token == suspicious {
                            return Ok((StatusCode::OK, "ok".to_string()).into_response())
                        } else {
                            return Err((StatusCode::UNAUTHORIZED, "wrong token".to_string()).into_response())
                        }
                    },
                    None => return Err((StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response())
                }
            },
            None => return Err((StatusCode::LOCKED, "token doesnt defined".to_string()).into_response()),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Limitations {
    pub max_avatar_size: u64,
    pub max_avatars: u64,
}

impl Config {
    pub fn parse(path: PathBuf) -> Self {
        let mut file = std::fs::File::open(path).expect("Access denied or file doesn't exists!");
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        toml::from_str(&data).unwrap()
    }
}
