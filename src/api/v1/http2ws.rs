use axum::extract::{Query, State};
use tracing::{debug, trace, warn};

use crate::{api::errors::{error_and_log, internal_and_log}, auth::Token, ApiResult, AppState};
use super::types::UserUuid;

pub(super) async fn verify(
    Token(token): Token,
    State(state): State<AppState>,
) -> ApiResult<&'static str> {
    state.config.read().await.clone()
        .verify_token(&token)?;
    Ok("ok")
}

pub(super) async fn raw(
    Token(token): Token,
    Query(query): Query<UserUuid>,
    State(state): State<AppState>,
    body: String,
) -> ApiResult<&'static str> {
    trace!(body = body);
    state.config.read().await.clone().verify_token(&token)?;
    let payload = hex::decode(body).map_err(|err| { warn!("not raw data"); error_and_log(err, crate::ApiError::NotAcceptable) })?;
    debug!("{:?}", payload);

    match query.uuid {
        Some(uuid) => {
            // for only one
            let tx = state.session.get(&uuid).ok_or_else(|| { warn!("unknown uuid"); crate::ApiError::NotFound })?;
            tx.value().send(crate::api::figura::SessionMessage::Ping(payload)).await.map_err(internal_and_log)?;
            Ok("ok")
        },
        None => {
            // for all
            warn!("uuid doesnt defined");
            Err(crate::ApiError::NotFound)
        },
    }
}

pub(super) async fn sub_raw(
    Token(token): Token,
    Query(query): Query<UserUuid>,
    State(state): State<AppState>,
    body: String,
) -> ApiResult<&'static str> {
    trace!(body = body);
    state.config.read().await.clone().verify_token(&token)?;
    let payload = hex::decode(body).map_err(|err| { warn!("not raw data"); error_and_log(err, crate::ApiError::NotAcceptable) })?;
    debug!("{:?}", payload);
    
    match query.uuid {
        Some(uuid) => {
            // for only one
            let tx = state.subscribes.get(&uuid).ok_or_else(|| { warn!("unknown uuid"); crate::ApiError::NotFound })?;
            tx.value().send(payload).map_err(internal_and_log)?;
            Ok("ok")
        },
        None => {
            warn!("uuid doesnt defined");
            Err(crate::ApiError::NotFound)
        },
    }
}