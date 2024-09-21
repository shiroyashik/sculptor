use std::{fs::File, io::Read, path::{Path, PathBuf}};

use base64::prelude::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use ring::digest::{self, digest};
use tokio::io::AsyncReadExt;
use tracing::{error, info};
use uuid::Uuid;
use chrono::prelude::*;

use crate::{auth::{UManager, Userinfo}, state::{AdvancedUsers, BannedPlayer}};

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

pub fn update_advanced_users(value: &std::collections::HashMap<Uuid, AdvancedUsers>, umanager: &UManager) {
    let users: Vec<(Uuid, Userinfo)> = value
        .iter()
        .map( |(uuid, userdata)| {
            (
            uuid.clone(),
            Userinfo { 
                uuid: uuid.clone(),
                username: userdata.username.clone(),
                banned: userdata.banned,
                ..Default::default()
            }
        )})
        .collect();

    for (uuid, userinfo) in users {
        umanager.insert_user(uuid, userinfo.clone());
        if userinfo.banned {
            umanager.ban(&userinfo)
        }
    }
}

pub async fn update_bans_from_minecraft(folder: PathBuf, umanager: std::sync::Arc<UManager>) {
    let path = folder.join("banned-players.json");
    let mut file = tokio::fs::File::open(path.clone()).await.expect("Access denied or banned-players.json doesn't exists!");
    let mut data = String::new();
    // vars end

    // initialize
    file.read_to_string(&mut data).await.expect("cant read banned-players.json");
    let mut old_bans: Vec<BannedPlayer> = serde_json::from_str(&data).expect("cant parse banned-players.json");
    
    if !old_bans.is_empty() {
        let names: Vec<String> = old_bans.iter().map(|user| user.name.clone()).collect();
        info!("Banned players: {}", names.join(", "));
    }

    for player in &old_bans {
        umanager.ban(&player.clone().into());
    }

    // old_bans
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        let mut file = tokio::fs::File::open(path.clone()).await.expect("Access denied or file doesn't exists!");
        let mut data = String::new();
        file.read_to_string(&mut data).await.expect("cant read banned-players.json");
        let new_bans: Vec<BannedPlayer> = if let Ok(res) = serde_json::from_str(&data) { res } else {
            error!("Error occured while parsing a banned-players.json");
            continue;
        };

        if new_bans != old_bans {
            info!("Minecraft ban list modification detected!");
            let unban: Vec<&BannedPlayer> = old_bans.iter().filter(|user| !new_bans.contains(user)).collect();
            let mut unban_names = unban.iter().map(|user| user.name.clone()).collect::<Vec<String>>().join(", ");
            if !unban.is_empty() {
                for player in unban {
                    umanager.unban(&player.uuid);
                }
            } else { unban_names = String::from("-")};
            let ban: Vec<&BannedPlayer> = new_bans.iter().filter(|user| !old_bans.contains(user)).collect();
            let mut ban_names = ban.iter().map(|user| user.name.clone()).collect::<Vec<String>>().join(", ");
            if !ban.is_empty() {
                for player in ban {
                    umanager.ban(&player.clone().into());
                }
            } else { ban_names = String::from("-")};
            info!("List of changes:\n    Banned: {ban_names}\n    Unbanned: {unban_names}");
            // Write new to old for next iteration
            old_bans = new_bans;
        }
    }
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

pub fn get_log_file(folder: &str) -> String {
    let local_date = Local::now().format("%Y-%m-%d");
    let mut index: u16 = 0;
    loop {
        let file_name = format!("{local_date}.{:04}.log", index);
        let file_path = Path::new(folder).join(&file_name);
        if !Path::new(&file_path).exists() {
            return file_name;
        }
        index += 1;
    }
}

pub fn get_limit_as_bytes(limit: usize) -> usize {
    1024 + limit * 1024 // Adding additional 1 KB just for fun :)
}