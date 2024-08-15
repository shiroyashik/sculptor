use axum::{
    extract::State,
    Json
};
use tracing::debug;

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