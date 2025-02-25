use axum::{extract::{Query, State}, http::HeaderMap, response::{IntoResponse, Response}, routing::get, Router};
use reqwest::{header::USER_AGENT, StatusCode};
use ring::digest::{self, digest};
use tracing::{error, info, instrument};

use crate::{auth::{has_joined, Userinfo}, utils::rand, AppState};
use super::types::auth::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}

async fn id(
    // First stage of authentication
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id =
        faster_hex::hex_string(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0..20]);
    let state = state.user_manager;
    state.pending_insert(server_id.clone(), query.username);
    server_id
}

#[instrument(skip_all)]
async fn verify(
    // Second stage of authentication
    Query(query): Query<Verify>,
    header: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let server_id = query.id.clone();
    let nickname = state.user_manager.pending_remove(&server_id).unwrap().1; // TODO: Add error check
    let userinfo = match has_joined(
        state.config.read().await.auth_providers.clone(),
        &server_id,
        &nickname
    ).await {
        Ok(d) => d,
        Err(_e) => {
            // error!("[Authentication] {e}"); // In auth error log already defined
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal verify error".to_string()).into_response();
        },
    };
    if let Some((uuid, auth_provider)) = userinfo {
        let umanager = state.user_manager;
        if umanager.is_banned(&uuid) {
            info!("{nickname} tried to log in, but was banned");
            return (StatusCode::BAD_REQUEST, "You're banned!".to_string()).into_response();
        }
        let mut userinfo = Userinfo {
            nickname,
            uuid,
            token: Some(server_id.clone()),
            auth_provider,
            ..Default::default()
        };
        if let Some(agent) = header.get(USER_AGENT) {
            if let Ok(agent) = agent.to_str() {
                userinfo.version = agent.to_string();
            }
        }
        info!("{} logged in using {} with {}", userinfo.nickname, userinfo.auth_provider.name, userinfo.version);

        match umanager.insert(uuid, server_id.clone(), userinfo.clone()) {
            Ok(_) => {},
            Err(_) => {
                umanager.remove(&uuid);
                if umanager.insert(uuid, server_id.clone(), userinfo).is_err() {
                    error!("Old token error after attempting to remove it! Unexpected behavior!");
                    return (StatusCode::BAD_REQUEST, "second session detected".to_string()).into_response();
                };
            }
        }
        (StatusCode::OK, server_id.to_string()).into_response()
    } else {
        info!("failed to verify {nickname}");
        (StatusCode::BAD_REQUEST, "failed to verify".to_string()).into_response()
    }
}