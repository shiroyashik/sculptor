use std::{io::Read, path::PathBuf};

use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub motd: String,
    pub limitations: Limitations,
    pub advanced_users: Table,
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
