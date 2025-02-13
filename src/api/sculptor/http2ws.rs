use std::collections::HashMap;

use axum::extract::{Query, State};
use tracing::instrument;
use uuid::Uuid;

use crate::{api::errors::{error_and_log, internal_and_log}, auth::Token, ApiResult, AppState};

/*
    FIXME: need to refactor
*/

pub(super) async fn verify(
    Token(token): Token,
    State(state): State<AppState>,
) -> ApiResult<&'static str> {
    state.config.read().await.clone()
        .verify_token(&token)?;
    Ok("ok")
}

#[instrument(skip(token, state, body))]
pub(super) async fn raw(
    Token(token): Token,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
    body: String,
) -> ApiResult<&'static str> {
    tracing::trace!(body = body);
    state.config.read().await.clone().verify_token(&token)?;
    let mut payload = vec![0; body.len() / 2];
    faster_hex::hex_decode(body.as_bytes(), &mut payload).map_err(|err| { tracing::warn!("not raw data"); error_and_log(err, crate::ApiError::NotAcceptable) })?;

    if query.contains_key("uuid") == query.contains_key("all") {
        tracing::warn!("invalid query params");
        return Err(crate::ApiError::BadRequest);
    }

    if let Some(uuid) = query.get("uuid") {
        // for one
        let uuid = Uuid::parse_str(uuid).map_err(|err| { tracing::warn!("invalid uuid"); error_and_log(err, crate::ApiError::BadRequest) })?;
        let tx = state.session.get(&uuid).ok_or_else(|| { tracing::warn!("unknown uuid"); crate::ApiError::NotFound })?;
        tx.value().send(crate::api::figura::SessionMessage::Ping(payload)).await.map_err(internal_and_log)?;
        Ok("ok")
    } else if query.contains_key("all") {
        // for all
        for tx in state.session.iter() {
            if let Err(e) = tx.value().send(crate::api::figura::SessionMessage::Ping(payload.clone())).await {
                tracing::debug!(error = ?e , "error while sending to session");
            }
        };
        Ok("ok")
    } else {
        tracing::error!("unreachable code!");
        Err(crate::ApiError::Internal)
    }
}

#[instrument(skip(token, state, body))]
pub(super) async fn sub_raw(
    Token(token): Token,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
    body: String,
) -> ApiResult<&'static str> {
    tracing::trace!(body = body);
    state.config.read().await.clone().verify_token(&token)?;
    let mut payload = vec![0; body.len() / 2];
    faster_hex::hex_decode(body.as_bytes(), &mut payload).map_err(|err| { tracing::warn!("not raw data"); error_and_log(err, crate::ApiError::NotAcceptable) })?;

    if let Some(uuid) = query.get("uuid") {
        let uuid = Uuid::parse_str(uuid).map_err(|err| { tracing::warn!("invalid uuid"); error_and_log(err, crate::ApiError::BadRequest) })?;
        let tx = state.subscribes.get(&uuid).ok_or_else(|| { tracing::warn!("unknown uuid"); crate::ApiError::NotFound })?;
        tx.value().send(payload).map_err(internal_and_log)?;
        Ok("ok")
    } else {
        tracing::warn!("uuid doesnt defined");
        Err(crate::ApiError::NotFound)
    }
}