use std::sync::Arc;

use anyhow_http::{http_error_ret, response::Result};
use axum::{
    body::Bytes, extract::{Path, State}, response::IntoResponse, Json, http::header,
};
use dashmap::DashMap;
use reqwest::StatusCode;
use tracing::debug;
use serde_json::{json, Value};
use tokio::{
    fs,
    io::{self, AsyncReadExt, BufWriter}, sync::broadcast::Sender,
};
use uuid::Uuid;

use crate::{
    auth::Token,
    utils::{calculate_file_sha256, format_uuid, get_correct_array},
    AppState,
};
use super::types::S2CMessage;

pub async fn user_info(
    Path(uuid): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("Receiving profile information for {}", uuid);

    let formatted_uuid = format_uuid(&uuid);

    let avatar_file = format!("avatars/{}.moon", formatted_uuid);

    let auth_system = match state.user_manager.get_by_uuid(&uuid) {
        Some(d) => d.auth_system.to_string(),
        None => return     (
            StatusCode::NO_CONTENT,
            [(header::CONTENT_TYPE, "text/plain")],
            "err".to_string()
        )// (StatusCode::NO_CONTENT, "not sculptor user".to_string()), //(StatusCode::NOT_FOUND, "not found".to_string()).into_response(),
    };

    let mut user_info_response = json!({
        "uuid": &formatted_uuid,
        "rank": "default",
        "equipped": [],
        "lastUsed": "2024-05-11T22:20:48.884Z",
        "equippedBadges": {
            "special": [0,0,0,0,0,0],
            "pride": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
        },
        "version": "0.1.4+1.20.1",
        "banned": false,
        "authSystem": auth_system
    });

    if let Some(settings) = state.config.lock().await.advanced_users.get(&formatted_uuid) {
        let pride = get_correct_array(settings.get("pride").unwrap());
        let special = get_correct_array(settings.get("special").unwrap());
        let badges = user_info_response
            .get_mut("equippedBadges")
            .and_then(Value::as_object_mut)
            .unwrap();
        badges.append(
            json!({
                "special": special,
                "pride": pride
            })
            .as_object_mut()
            .unwrap(),
        )
    }

    if fs::metadata(&avatar_file).await.is_ok() {
        if let Some(equipped) = user_info_response
            .get_mut("equipped")
            .and_then(Value::as_array_mut)
        {
            match calculate_file_sha256(&avatar_file) {
                Ok(hash) => equipped.push(json!({
                    "id": "avatar",
                    "owner": &formatted_uuid,
                    "hash": hash
                })),
                Err(_e) => {}
            }
        }
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        user_info_response.to_string()
    )
}

pub async fn download_avatar(Path(uuid): Path<Uuid>) -> Result<Vec<u8>> {
    let uuid = format_uuid(&uuid);
    tracing::info!("Requesting an avatar: {}", uuid);
    let mut file = if let Ok(file) = fs::File::open(format!("avatars/{}.moon", uuid)).await {
        file
    } else {
        http_error_ret!(NOT_FOUND, "Error! This avatar does not exist!");
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    //match Body::from_file("avatars/74cf2ba3-f346-4dfe-b3b5-f453b9f5cc5e.moon").await  {
    // match Body::from_file(format!("avatars/{}.moon",uuid)).await  {
    //     Ok(body) => Ok(Response::builder(StatusCode::Ok).body(body).build()),
    //     Err(e) => Err(e.into()),
    // }
    Ok(buffer)
}

pub async fn upload_avatar(
    Token(token): Token,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<String> {
    let request_data = body;

    let token = match token {
        Some(t) => t,
        None => http_error_ret!(UNAUTHORIZED, "Authentication error!"),
    };

    if let Some(user_info) = state.user_manager.get(&token) {
        tracing::info!(
            "{} ({}) trying to upload an avatar",
            user_info.uuid,
            user_info.username
        );
        let avatar_file = format!("avatars/{}.moon", user_info.uuid);
        let mut file = BufWriter::new(fs::File::create(&avatar_file).await?);
        io::copy(&mut request_data.as_ref(), &mut file).await?;
    }
    Ok("ok".to_string())
}

pub async fn equip_avatar(Token(token): Token, State(state): State<AppState>) -> String {
    debug!("[API] S2C : Equip");
    let uuid = state.user_manager.get(&token.unwrap()).unwrap().uuid;
    send_event(&state.broadcasts, &uuid);
    "ok".to_string()
}

pub async fn delete_avatar(Token(token): Token, State(state): State<AppState>) -> Result<String> {
    let token = match token {
        Some(t) => t,
        None => http_error_ret!(UNAUTHORIZED, "Authentication error!"),
    };
    if let Some(user_info) = state.user_manager.get(&token) {
        tracing::info!(
            "{} ({}) is trying to delete the avatar",
            user_info.uuid,
            user_info.username
        );
        let avatar_file = format!("avatars/{}.moon", user_info.uuid);
        fs::remove_file(avatar_file).await?;
        send_event(&state.broadcasts, &user_info.uuid);
    }
    // let avatar_file = format!("avatars/{}.moon",user_info.uuid);
    Ok("ok".to_string())
}

pub fn send_event(broadcasts: &Arc<DashMap<Uuid, Sender<Vec<u8>>>>, uuid: &Uuid) {
    if let Some(broadcast) = broadcasts.get(&uuid) {
        if broadcast.send(S2CMessage::Event(*uuid).to_vec()).is_err() {
            debug!("[WebSocket] Failed to send Event! There is no one to send. UUID: {uuid}")
        };
    } else {
        debug!("[WebSocket] Failed to send Event! Can't find UUID: {uuid}")
    };
}