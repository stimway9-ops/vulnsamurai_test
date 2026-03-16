pub mod auth;
pub mod health;
pub mod scans;
pub mod vulns;

use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

// Shared API error type
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Unauthorized,
    BadRequest(String),
    InternalError(String),
    Conflict(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(m)      => (StatusCode::NOT_FOUND, m.clone()),
            ApiError::Unauthorized     => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            ApiError::BadRequest(m)    => (StatusCode::BAD_REQUEST, m.clone()),
            ApiError::InternalError(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            ApiError::Conflict(m)      => (StatusCode::CONFLICT, m.clone()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!("Internal error: {e:#}");
        ApiError::InternalError("Internal server error".into())
    }
}

impl From<mongodb::error::Error> for ApiError {
    fn from(e: mongodb::error::Error) -> Self {
        tracing::error!("MongoDB error: {e:#}");
        ApiError::InternalError("Database error".into())
    }
}
