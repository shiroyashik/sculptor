use anyhow_http::{http_error_ret, response::Result};
use axum::{body::Bytes, debug_handler, extract::{Path, State}, Json};
use serde_json::{json, Value};
use tokio::{fs, io::{AsyncReadExt, BufWriter, self}};
use uuid::Uuid;

use crate::{utils::{calculate_file_sha256, format_uuid}, auth::Token, AppState};

#[debug_handler]
pub async fn user_info(
    Path(uuid): Path<Uuid>,
    State(_state): State<AppState>, // FIXME: Variable doesn't using!
) -> Json<Value> {
    log::info!("Получение информации для {}",uuid);

    let formatted_uuid = format_uuid(uuid);

    let avatar_file = format!("avatars/{}.moon", formatted_uuid);

    let mut user_info_response = json!({
        "uuid": &formatted_uuid,
        "rank": "default",
        "equipped": [],
        "lastUsed": "2024-05-11T22:20:48.884Z",
        "equippedBadges": {
            "special": [1,1,1,1,1,1],
            "pride": [0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
        },
        "version": "0.1.4+1.20.1",
        "banned": false
    });

    if fs::metadata(&avatar_file).await.is_ok() {
        if let Some(equipped) = user_info_response.get_mut("equipped").and_then(Value::as_array_mut){
            match calculate_file_sha256(&avatar_file){
                Ok(hash) => {
                    equipped.push(json!({
                        "id": "avatar",
                        "owner": &formatted_uuid,
                        "hash": hash
                    }))
                }
                Err(_e) => {}
            }

            
        }
    }
    Json(user_info_response)
}

#[debug_handler]
pub async fn download_avatar(
    Path(uuid): Path<Uuid>,
) -> Result<Vec<u8>> {
    let uuid = format_uuid(uuid);
    log::info!("Запрашиваем аватар: {}", uuid);
    let mut file = if let Ok(file) = fs::File::open(format!("avatars/{}.moon", uuid)).await {
        file
    } else {
        http_error_ret!(NOT_FOUND, "Ошибка! Данный аватар не существует!");
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

#[debug_handler]
pub async fn upload_avatar(
    Token(token): Token,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<String>  {
    
    let request_data = body;

    let token = match token {
        Some(t) => t,
        None => http_error_ret!(UNAUTHORIZED, "Ошибка аутентификации!"),
    };
    let userinfos = state.authenticated.lock().await;

    if let Some(user_info) = userinfos.get(token.as_str()) {
        log::info!("{} ({}) пытается загрузить аватар",user_info.uuid,user_info.username);
        let avatar_file = format!("avatars/{}.moon",user_info.uuid);
        let mut file = BufWriter::new(fs::File::create(&avatar_file).await?);
        io::copy(&mut request_data.as_ref(), &mut file).await?;
    }
    Ok(format!("ok"))
}

pub async fn equip_avatar() -> String {
    format!("ok")
}

pub async fn delete_avatar(
    Token(token): Token,
    State(state): State<AppState>,
) -> Result<String> {
    let token = match token {
        Some(t) => t,
        None => http_error_ret!(UNAUTHORIZED, "Ошибка аутентификации!"),
    };
    let userinfos = state.authenticated.lock().await;
    if let Some(user_info) = userinfos.get(token.as_str()) {
        log::info!("{} ({}) пытается удалить аватар",user_info.uuid,user_info.username);
        let avatar_file = format!("avatars/{}.moon",user_info.uuid);
        fs::remove_file(avatar_file).await?;
    }
    // let avatar_file = format!("avatars/{}.moon",user_info.uuid);
    Ok(format!("ok"))
}