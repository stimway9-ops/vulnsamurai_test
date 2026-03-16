use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::post,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use bson::doc;
use chrono::Utc;
use validator::Validate;

use crate::{
    db::AppState,
    middleware::auth::generate_token,
    models::user::{AuthResponse, Claims, LoginRequest, RegisterRequest, User, UserResponse, UserRole},
};
use super::ApiError;

const USERS_COLL: &str = "users";
const TOKEN_TTL_SECS: usize = 60 * 60 * 24; // 24 h

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login",    post(login))
}

// POST /api/auth/register
async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    body.validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let coll = state.db.collection::<User>(USERS_COLL);

    // Check duplicate email
    if coll.find_one(doc! { "email": &body.email }, None).await?.is_some() {
        return Err(ApiError::Conflict("Email already registered".into()));
    }

    let password_hash = hash(&body.password, DEFAULT_COST)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let now = Utc::now();
    let user = User {
        id: None,
        username: body.username.clone(),
        email: body.email.clone(),
        password_hash,
        role: UserRole::Analyst,
        created_at: now,
    };

    let insert = coll.insert_one(&user, None).await?;
    let oid = insert.inserted_id.as_object_id().unwrap();

    let claims = build_claims(oid.to_hex(), &body.username, &UserRole::Analyst);
    let token = generate_token(&claims, &state.jwt_secret)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let mut returned = user;
    returned.id = Some(oid);

    Ok((StatusCode::CREATED, Json(AuthResponse {
        token,
        user: returned.into(),
    })))
}

// POST /api/auth/login
async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let coll = state.db.collection::<User>(USERS_COLL);

    let user = coll
        .find_one(doc! { "email": &body.email }, None)
        .await?
        .ok_or_else(|| ApiError::Unauthorized)?;

    let valid = verify(&body.password, &user.password_hash)
        .map_err(|_| ApiError::Unauthorized)?;

    if !valid {
        return Err(ApiError::Unauthorized);
    }

    let oid = user.id.unwrap();
    let claims = build_claims(oid.to_hex(), &user.username, &user.role);
    let token = generate_token(&claims, &state.jwt_secret)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}

fn build_claims(sub: String, username: &str, role: &UserRole) -> Claims {
    let now = chrono::Utc::now().timestamp() as usize;
    Claims {
        sub,
        username: username.to_string(),
        role: role.clone(),
        iat: now,
        exp: now + TOKEN_TTL_SECS,
    }
}
