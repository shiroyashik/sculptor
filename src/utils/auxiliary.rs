use std::{fs::File, io::Read, path::{Path, PathBuf}, sync::Arc};

use notify::{Event, Watcher};
use tokio::{io::AsyncReadExt, sync::RwLock};
use base64::prelude::*;
use rand::{thread_rng, Rng};
use ring::digest::{self, digest};
use uuid::Uuid;
use chrono::prelude::*;

use crate::{auth::Userinfo, state::{BannedPlayer, Config}, UManager};

pub fn rand() -> [u8; 50] {
    let mut rng = thread_rng();
    let distr = rand::distributions::Uniform::new_inclusive(0, 255);
    let mut nums: [u8; 50] = [0u8; 50];
    for x in &mut nums {
        *x = rng.sample(distr);
    }
    nums
}

pub async fn update_advanced_users(
    path: PathBuf,
    umanager: Arc<UManager>,
    sessions: Arc<dashmap::DashMap<Uuid, tokio::sync::mpsc::Sender<crate::api::figura::SessionMessage>>>,
    config: Arc<RwLock<Config>>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(1);
    tx.send(Ok(notify::Event::default())).await.unwrap();
    let mut watcher = notify::PollWatcher::new(
        move |res| {
            tx.blocking_send(res).unwrap();
        },
        notify::Config::default(),
    ).unwrap();
    watcher.watch(&path, notify::RecursiveMode::NonRecursive).unwrap();

    let mut first_time = true;
    while rx.recv().await.is_some() {
        let new_config = Config::parse(path.clone());
        let mut config = config.write().await;

        if new_config != *config || first_time {
            if !first_time { tracing::info!("Server configuration modification detected!") }
            first_time = false;
            *config = new_config;
            let users: Vec<(Uuid, Userinfo)> = config.advanced_users
                .iter()
                .map( |(uuid, userdata)| {
                    (
                    *uuid,
                    Userinfo { 
                        uuid: *uuid,
                        nickname: userdata.username.clone(),
                        banned: userdata.banned,
                        ..Default::default()
                    }
                )})
                .collect();
        
            for (uuid, userinfo) in users {
                umanager.insert_user(uuid, userinfo.clone());
                if userinfo.banned {
                    umanager.ban(&userinfo);
                    if let Some(tx) = sessions.get(&uuid) {let _ = tx.send(crate::api::figura::SessionMessage::Banned).await;}
                } else {
                    umanager.unban(&uuid);
                }
            }
        }
    }
}

pub async fn update_bans_from_minecraft(
    folder: PathBuf,
    umanager: Arc<UManager>,
    sessions: Arc<dashmap::DashMap<Uuid, tokio::sync::mpsc::Sender<crate::api::figura::SessionMessage>>>
) {
    let path = folder.join("banned-players.json");
    let mut file = tokio::fs::File::open(path.clone()).await.expect("Access denied or banned-players.json doesn't exists!");
    let mut data = String::new();
    // vars end

    // initialize
    file.read_to_string(&mut data).await.expect("cant read banned-players.json");
    let mut old_bans: Vec<BannedPlayer> = serde_json::from_str(&data).expect("cant parse banned-players.json");

    if !old_bans.is_empty() {
        let names: Vec<String> = old_bans.iter().map(|user| user.name.clone()).collect();
        tracing::info!("Banned players: {}", names.join(", "));
    }

    for player in &old_bans {
        umanager.ban(&player.clone().into());
        if let Some(tx) = sessions.get(&player.uuid) {let _ = tx.send(crate::api::figura::SessionMessage::Banned).await;}
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(1);
    let mut watcher = notify::PollWatcher::new(
        move |res| {
            tx.blocking_send(res).unwrap();
        },
        notify::Config::default(),
    ).unwrap();
    watcher.watch(&path, notify::RecursiveMode::NonRecursive).unwrap();

    // old_bans
    while rx.recv().await.is_some() {
        let mut file = tokio::fs::File::open(path.clone()).await.expect("Access denied or file doesn't exists!");
        let mut data = String::new();
        file.read_to_string(&mut data).await.expect("cant read banned-players.json");
        let new_bans: Vec<BannedPlayer> = if let Ok(res) = serde_json::from_str(&data) { res } else {
            tracing::error!("Error occured while parsing a banned-players.json");
            continue;
        };

        if new_bans != old_bans {
            tracing::info!("Minecraft ban list modification detected!");
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
                    if let Some(tx) = sessions.get(&player.uuid) {let _ = tx.send(crate::api::figura::SessionMessage::Banned).await;}
                }
            } else { ban_names = String::from("-")};
            tracing::info!("List of changes:\n    Banned: {ban_names}\n    Unbanned: {unban_names}");
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
    let hex_hash = faster_hex::hex_string(hash);

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