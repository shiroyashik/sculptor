use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router
};
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use crate::{auth::Token, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify", get(verify))
        .route("/raw", post(raw))
        .route("/sub/raw", post(sub_raw))
}

#[derive(Deserialize)]
struct UserUuid {
    uuid: Option<Uuid>,
}

async fn verify(
    Token(token): Token,
    State(state): State<AppState>,
) -> Response {
    match token {
        Some(t) => {
            if !state.config.lock().await.verify_token(&t) {
                return (StatusCode::UNAUTHORIZED, "wrong token".to_string()).into_response()
            }
        },
        None => return (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response(),
    }
    (StatusCode::OK, "ok".to_string()).into_response()
}

async fn raw(
    Token(token): Token,
    Query(query): Query<UserUuid>,
    State(state): State<AppState>,
    body: String,
) -> Response {
    debug!(body = body);
    match token {
        Some(t) => {
            if !state.config.lock().await.verify_token(&t) {
                return (StatusCode::UNAUTHORIZED, "wrong token".to_string()).into_response()
            }
        },
        None => return (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response(),
    }
    let payload = match hex::decode(body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_ACCEPTABLE, "not raw data".to_string()).into_response(),
    };
    debug!("{:?}", payload);

    match query.uuid {
        Some(uuid) => {
            // for only one
            let tx = match state.session.get(&uuid) {
                Some(d) => d,
                None => return (StatusCode::NOT_FOUND, "unknown uuid".to_string()).into_response(),
            };
            match tx.value().send(payload).await {
                Ok(_) => return (StatusCode::OK, "ok".to_string()).into_response(),
                Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "cant send".to_string()).into_response(),
            };
        },
        None => {
            // for all
            return (StatusCode::NOT_FOUND, "uuid doesnt defined".to_string()).into_response();
        },
    }
}

async fn sub_raw(
    Token(token): Token,
    Query(query): Query<UserUuid>,
    State(state): State<AppState>,
    body: String,
) -> Response {
    debug!(body = body);
    match token {
        Some(t) => {
            if !state.config.lock().await.verify_token(&t) {
                return (StatusCode::UNAUTHORIZED, "wrong token".to_string()).into_response()
            }
        },
        None => return (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response(),
    }
    let payload = match hex::decode(body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_ACCEPTABLE, "not raw data".to_string()).into_response(),
    };
    debug!("{:?}", payload);

    
    match query.uuid {
        Some(uuid) => {
            // for only one
            let tx = match state.broadcasts.get(&uuid) {
                Some(d) => d,
                None => return (StatusCode::NOT_FOUND, "unknown uuid".to_string()).into_response(),
            };
            match tx.value().send(payload) {
                Ok(_) => return (StatusCode::OK, "ok".to_string()).into_response(),
                Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "cant send".to_string()).into_response(),
            };
        },
        None => {
            return (StatusCode::NOT_FOUND, "uuid doesnt defined".to_string()).into_response();
        },
    }
}