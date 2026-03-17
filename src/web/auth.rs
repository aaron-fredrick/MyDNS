use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::records::findUserHash;
use crate::state::AppState;
use crate::web::error::ApiError;

// ── JWT claims ────────────────────────────────────────────────────────────────

/// Claims embedded in every issued JWT.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (username).
    pub sub: String,
    /// Issued-at epoch seconds.
    pub iat: u64,
    /// Expiry epoch seconds (1 hour from issuance).
    pub exp: u64,
}

// ── login handler ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

/// `POST /api/v1/auth/login`
#[allow(non_snake_case)]
pub async fn login(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let hash = findUserHash(&state.db, &body.username)
        .await
        .context("DB query failed")?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".into()))?;

    verify_password(&body.password, &hash)?;

    let token = issue_token(&body.username, &state.config.read().await.jwt_secret)
        .context("Token issuance failed")?;

    tracing::info!(username = %body.username, "Admin login successful");
    let _ = state
        .log_tx
        .send(format!("[AUTH] Login: user={}", body.username));

    Ok(Json(LoginResponse { token }))
}

// ── JWT extractor ─────────────────────────────────────────────────────────────

/// Axum extractor that validates a Bearer token and surfaces [`Claims`].
#[allow(dead_code)]
pub struct JwtClaims(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for JwtClaims {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": e.to_string()})),
            )
        })?;

        let secret = state.config.read().await.jwt_secret.clone();
        let claims = validate_token(token, &secret).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": e.to_string()})),
            )
        })?;

        Ok(JwtClaims(claims))
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn hashPassword(password: &str) -> anyhow::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Argon2 hashing error: {}", e))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<(), ApiError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|_| ApiError::Unauthorized("Invalid credentials".into()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| ApiError::Unauthorized("Invalid credentials".into()))
}

fn issue_token(username: &str, secret: &str) -> anyhow::Result<String> {
    let now = epoch_now();
    let claims = Claims {
        sub: username.to_string(),
        iat: now,
        exp: now + 3600,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("JWT encode failed")
}

fn validate_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .context("JWT validation failed")?;
    Ok(data.claims)
}

fn extract_bearer<'a>(parts: &'a Parts) -> anyhow::Result<&'a str> {
    parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .context("Missing or malformed Authorization header")
}

fn epoch_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
