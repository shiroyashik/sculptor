use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}};
use tokio::{fs, io::{self, BufWriter}};
use uuid::Uuid;

use crate::{api::figura::profile::send_event, auth::Token, AppState};

pub async fn upload_avatar(
    Path(uuid): Path<Uuid>,
    Token(token): Token,
    State(state): State<AppState>,
    body: Bytes,
) -> Response {
    let request_data = body;

    match state.config.lock().await.clone().verify_token(&token) {
        Ok(_) => {},
        Err(err) => return err,
    };

    tracing::info!(
        "trying to upload the avatar for {}",
        uuid,
    );

    let avatar_file = format!("avatars/{}.moon", &uuid);
    let mut file = BufWriter::new(fs::File::create(&avatar_file).await.unwrap());
    io::copy(&mut request_data.as_ref(), &mut file).await.unwrap();
    send_event(&state.broadcasts, &uuid);

    (StatusCode::OK, "ok".to_string()).into_response()
}

pub async fn delete_avatar(
    Path(uuid): Path<Uuid>,
    Token(token): Token,
    State(state): State<AppState>
) -> Response {
    match state.config.lock().await.clone().verify_token(&token) {
        Ok(_) => {},
        Err(err) => return err,
    };

    tracing::info!(
        "trying to delete the avatar for {}",
        uuid,
    );

    let avatar_file = format!("avatars/{}.moon", &uuid);
    match fs::remove_file(avatar_file).await {
        Ok(_) => {},
        Err(_) => return (StatusCode::NOT_FOUND, "avatar doesn't exist".to_string()).into_response()
    };
    send_event(&state.broadcasts, &uuid);

    (StatusCode::OK, "ok".to_string()).into_response()
}