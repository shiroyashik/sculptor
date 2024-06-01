use std::{io::Read, path::PathBuf};

use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub motd: String,
    pub advanced_users: Table,
}

impl Config {
    pub fn parse(path: PathBuf) -> Self {
        let mut file = std::fs::File::open(path).expect("Access denied or file doesn't exists!");
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        toml::from_str(&data).unwrap()
    }
}
