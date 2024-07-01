use axum::{extract::State, http::StatusCode, response::{IntoResponse, Response}};

use crate::{auth::Token, AppState};

pub async fn status(
    Token(token): Token,
    State(state): State<AppState>
) -> Response {
    match token {
        Some(token) => {
            if state.user_manager.is_authenticated(&token) {
                (StatusCode::OK, "ok".to_string()).into_response()
            } else {

                (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response()
            }
        }
        None => {
            (StatusCode::BAD_REQUEST, "bad request".to_string()).into_response()
        }
    }
}