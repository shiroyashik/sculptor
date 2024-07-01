use axum::{debug_handler, extract::{Query, State}, response::{IntoResponse, Response}, routing::get, Router};
use reqwest::StatusCode;
use ring::digest::{self, digest};
use tracing::{error, info};

use crate::{auth::{has_joined, Userinfo}, utils::rand, AppState};
use super::types::auth::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}

#[debug_handler]
async fn id(
    // First stage of authentication
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id =
        hex::encode(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0..20]);
    let state = state.user_manager;
    state.pending_insert(server_id.clone(), query.username);
    server_id
}

#[debug_handler]
async fn verify(
    // Second stage of authentication
    Query(query): Query<Verify>,
    State(state): State<AppState>,
) -> Response {
    let server_id = query.id.clone();
    let username = state.user_manager.pending_remove(&server_id).unwrap().1; // TODO: Add error check
    let userinfo = match has_joined(&server_id, &username).await {
        Ok(d) => d,
        Err(e) => {
            error!("[Authentication] {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal verify error".to_string()).into_response();
        },
    };
    if let Some((uuid, auth_system)) = userinfo {
        info!("[Authentication] {username} logged in using {auth_system:?}");
        let authenticated = state.user_manager;
        authenticated.insert(
            uuid,
            server_id.clone(),
            Userinfo {
                username,
                uuid,
                auth_system,
                token: Some(server_id.clone()),
            },
        );
        (StatusCode::OK, server_id.to_string()).into_response()
    } else {
        info!("[Authentication] failed to verify {username}");
        (StatusCode::BAD_REQUEST, "failed to verify".to_string()).into_response()
    }
}