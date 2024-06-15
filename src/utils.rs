use std::{fs::File, io::Read, str::FromStr};

use base64::prelude::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use ring::digest::{self, digest};
use uuid::Uuid;

use crate::auth::{AuthSystem, Userinfo};

// Core functions
pub fn rand() -> [u8; 50] {
    let mut rng = thread_rng();
    let distr = rand::distributions::Uniform::new_inclusive(0, 255);
    let mut nums: [u8; 50] = [0u8; 50];
    for x in &mut nums {
        *x = rng.sample(distr);
    }
    nums
}
// End of Core functions

pub fn _generate_hex_string(length: usize) -> String {
    // FIXME: Variable doesn't using!
    let rng = thread_rng();
    let random_bytes: Vec<u8> = rng.sample_iter(&Alphanumeric).take(length / 2).collect();

    hex::encode(random_bytes)
}

pub fn get_correct_array(value: &toml::Value) -> Vec<u8> {
    // let res: Vec<u8>;
    value
        .as_array()
        .unwrap()
        .iter()
        .map(move |x| x.as_integer().unwrap() as u8)
        .collect()
}
pub fn collect_advanced_users(value: &toml::Table) -> Vec<(Uuid, Userinfo)> {
    value
        .iter()
        .map( |(uuid, userdata)| {
            let auth_system = AuthSystem::from_str(userdata.as_table().unwrap().get("authSystem").expect("Can't find authSystem in advancedUser!").as_str().unwrap()).unwrap();
            let username = userdata.as_table().unwrap().get("username").expect("Can't find username in advancedUser!").as_str().unwrap().to_string();
            (
            Uuid::parse_str(uuid).unwrap(),
            Userinfo { username,
                uuid: Uuid::parse_str(uuid).unwrap(),
                auth_system,
                token: None
            }
        )})
        .collect()
}

pub fn format_uuid(uuid: &Uuid) -> String {
    // let uuid = Uuid::parse_str(&uuid)?; TODO: Вероятно format_uuid стоит убрать
    // .map_err(|_| tide::Error::from_str(StatusCode::InternalServerError, "Failed to parse UUID"))?;
    uuid.as_hyphenated().to_string()
}

pub fn calculate_file_sha256(file_path: &str) -> Result<String, std::io::Error> {
    // Read the file content
    let mut file = File::open(file_path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    // Convert the content to base64
    let base64_content = BASE64_STANDARD.encode(&content);

    // Calculate the SHA-256 hash of the base64 string
    let binding = digest(&digest::SHA256, base64_content.as_bytes());
    let hash = binding.as_ref();

    // Convert the hash to a hexadecimal string
    let hex_hash = hex::encode(hash);

    Ok(hex_hash)
}
