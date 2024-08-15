use axum::{http::StatusCode, response::{IntoResponse, Response}};
use thiserror::Error;
use tracing::{error, warn};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("bad request")]
    BadRequest, // 400
    #[error("unauthorized")]
    Unauthorized, // 401
    #[error("not found")]
    NotFound, // 404
    #[error("not acceptable")]
    NotAcceptable, // 406
    #[error("internal server error")]
    Internal, // 500
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::BadRequest => (StatusCode::BAD_REQUEST, "bad request").into_response(),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized").into_response(),
            ApiError::NotAcceptable=> (StatusCode::NOT_ACCEPTABLE, "not acceptable").into_response(),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response(),
        }
    }
}

pub fn internal_and_log<E: std::fmt::Display>(err: E) -> ApiError { // NOTE: Realize it like a macros?
    error!("Internal error: {}", err);
    ApiError::Internal
}

pub fn error_and_log<E: std::fmt::Display>(err: E, error_type: ApiError) -> ApiError {
    warn!("{error_type:?}: {}", err);
    error_type
}