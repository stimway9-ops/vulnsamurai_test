use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde_json::json;

use crate::{db::AppState, models::user::Claims};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    ExpiredToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token expired"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

// ── Extractor ─────────────────────────────────────────────────────────────────

pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::MissingToken)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        let key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;
                match e.kind() {
                    ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                    _ => AuthError::InvalidToken,
                }
            })?;

        Ok(AuthUser(token_data.claims))
    }
}

// ── Token generation helper ───────────────────────────────────────────────────

pub fn generate_token(claims: &Claims, secret: &str) -> anyhow::Result<String> {
    let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    Ok(jsonwebtoken::encode(&jsonwebtoken::Header::default(), claims, &key)?)
}
