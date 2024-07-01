use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json
};
use tracing::debug;

use crate::{auth::{Token, Userinfo}, AppState};

pub(super) async fn create_user(
    Token(token): Token,
    State(state): State<AppState>,
    Json(json): Json<Userinfo>
) -> Response {
    debug!("Json: {json:?}");
    match state.config.lock().await.clone().verify_token(&token) {
        Ok(_) => {},
        Err(e) => return e,
    }
    
    state.user_manager.insert_user(json.uuid, json);
    (StatusCode::OK, "ok".to_string()).into_response()
}