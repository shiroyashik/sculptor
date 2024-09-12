use axum::{extract::DefaultBodyLimit, routing::{delete, get, post, put}, Router};
use crate::AppState;

mod http2ws;
mod users;
mod types;
mod avatars;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify", get(http2ws::verify))
        .route("/raw", post(http2ws::raw))
        .route("/sub/raw", post(http2ws::sub_raw))
        .route("/user/list", get(users::list))
        .route("/user/sessions", get(users::list_sessions))
        .route("/user/create", post(users::create_user))
        .route("/user/:uuid/ban", post(users::ban))
        .route("/user/:uuid/unban", post(users::unban))
        .route("/avatar/:uuid", put(avatars::upload_avatar).layer(DefaultBodyLimit::disable()))
        .route("/avatar/:uuid", delete(avatars::delete_avatar))
}