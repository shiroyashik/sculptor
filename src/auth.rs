use axum::{async_trait, debug_handler, extract::{FromRequestParts, Query, State}, http::{request::Parts, StatusCode}, response::{IntoResponse, Response}, routing::get, Router};
use log::debug;
use serde::Deserialize;
use ring::digest::{self, digest};
use crate::utils::*;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/id", get(id))
        .route("/verify", get(verify))
}


// Веб функции
#[derive(Deserialize)]
struct Id {username: String}

#[debug_handler]
async fn id( // 1 этап аутентификации
    Query(query): Query<Id>,
    State(state): State<AppState>,
) -> String {
    let server_id = bytes_into_string(&digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &rand()).as_ref()[0 .. 20]);
    let state = state.pending.lock().await;
    state.insert(server_id.clone(), query.username);
    server_id
}

#[derive(Deserialize)]
struct Verify {id: String}

#[debug_handler]
async fn verify( // 2 этап аутентификации
    Query(query): Query<Verify>,
    State(state): State<AppState>,
) -> String {
    let server_id = query.id.clone();
    let username = state.pending.lock().await.remove(&server_id).unwrap().1;
    if let Some(uuid) = elyby_api::has_joined(&server_id, &username).await.unwrap() {
        let authenticated = state.authenticated.lock().await;
        let link = state.authenticated_link.lock().await;
        authenticated.insert(server_id.clone(), crate::Userinfo { username, uuid });
        link.insert(uuid, crate::AuthenticatedLink(server_id.clone()));
        return format!("{server_id}")
    } else {
        return String::from("failed to verify")
    }
}

pub async fn status(
    Token(token): Token,
    State(state): State<AppState>,
) -> Response {
    match token {
        Some(token) => {
            if state.authenticated.lock().await.contains_key(&token) {
                // format!("ok") // 200
                (StatusCode::OK, format!("ok")).into_response()
            } else {
                // format!("unauthorized") // 401
                (StatusCode::UNAUTHORIZED, format!("unauthorized")).into_response()
            }
        },
        None => {
            // format!("bad request") // 400
            (StatusCode::BAD_REQUEST, format!("bad request")).into_response()
        },
    }
}
// Конец веб функций


// Это экстрактор достающий из Заголовка зовущегося токен, соответственно ТОКЕН.
#[derive(PartialEq, Debug)]
pub struct Token(pub Option<String>);

#[async_trait]
impl<S> FromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("token")
            .and_then(|value| value.to_str().ok());
        debug!("[Extractor Token] Данные: {token:?}");
        match token {
            Some(token) => Ok(Self(Some(token.to_string()))),
            None => Ok(Self(None)),
        }
    }
}
// Конец экстрактора