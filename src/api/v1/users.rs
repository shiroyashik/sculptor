use axum::{
    extract::{Path, State},
    Json
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{auth::{Token, Userinfo}, ApiResult, AppState};

pub(super) async fn create_user(
    Token(token): Token,
    State(state): State<AppState>,
    Json(json): Json<Userinfo>
) -> ApiResult<&'static str> {
    state.config.read().await.clone().verify_token(&token)?;

    debug!("Creating new user: {json:?}");
    
    state.user_manager.insert_user(json.uuid, json);
    Ok("ok")
}

pub(super) async fn ban(
    Token(token): Token,
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>
) -> ApiResult<&'static str> {
    state.config.read().await.clone().verify_token(&token)?;

    info!("Trying ban user: {uuid}");
    
    state.user_manager.ban(&Userinfo { uuid: uuid, banned: true, ..Default::default() });
    Ok("ok")
}

pub(super) async fn unban(
    Token(token): Token,
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>
) -> ApiResult<&'static str> {
    state.config.read().await.clone().verify_token(&token)?;

    info!("Trying unban user: {uuid}");
    
    state.user_manager.unban(&uuid);
    Ok("ok")
}