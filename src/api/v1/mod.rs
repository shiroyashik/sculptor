use axum::{routing::{get, post}, Router};
use crate::AppState;

mod http2ws;
mod users;
mod auth;
mod types;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify", get(http2ws::verify))
        .route("/raw", post(http2ws::raw))
        .route("/sub/raw", post(http2ws::sub_raw))
        .route("/auth/", get(auth::status))
        .route("/users/create", post(users::create_user))
}