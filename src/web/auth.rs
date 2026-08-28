use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}, Json};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

use crate::db::records::findUserHash;
use crate::state::AppState;
use crate::web::error::ApiError;

// Login is limited per client IP to reduce brute-force exposure.
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW_SECONDS: u64 = 300;
const WS_AUTH_PROTOCOL_PREFIX: &str = "mydns-auth.";

#[derive(Clone)]
struct LoginAttemptTracker {
    attempts: u32,
    window_start: u64,
}

pub struct LoginRateLimiter {
    trackers: Arc<RwLock<HashMap<IpAddr, LoginAttemptTracker>>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self { trackers: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), ApiError> {
        let now = epoch_now();
        let mut trackers = self.trackers.write().await;
        let tracker = trackers.entry(ip).or_insert(LoginAttemptTracker { attempts: 0, window_start: now });

        if now.saturating_sub(tracker.window_start) > LOGIN_WINDOW_SECONDS {
            tracker.attempts = 0;
            tracker.window_start = now;
        }

        if tracker.attempts >= MAX_LOGIN_ATTEMPTS {
            tracing::warn!(ip = %ip, "Login rate limit exceeded");
            return Err(ApiError::TooManyRequests("Too many login attempts. Please try again later.".into()));
        }

        tracker.attempts += 1;
        Ok(())
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[allow(non_snake_case)]
pub async fn login(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    state.login_rate_limiter.check_rate_limit(addr.ip()).await?;

    let hash = findUserHash(&state.db, &body.username)
        .await
        .context("DB query failed")?
        .ok_or_else(|| {
            tracing::warn!(username = %body.username, "Login attempt with unknown username");
            ApiError::Unauthorized("Invalid credentials".into())
        })?;

    verify_password(&body.password, &hash).inspect_err(|_| {
        tracing::warn!(username = %body.username, "Login attempt with wrong password");
    })?;

    let token = issue_token(&body.username, &state.config.read().await.jwt_secret)
        .context("Token issuance failed")?;

    tracing::info!(username = %body.username, "Admin login successful");
    let _ = state.log_tx.send(format!("[AUTH] Login: user={}", body.username));
    Ok(Json(LoginResponse { token }))
}

#[allow(dead_code)]
pub struct JwtClaims(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for JwtClaims {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).map_err(|e| {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": e.to_string() })))
        })?;
        let secret = state.config.read().await.jwt_secret.clone();
        let claims = validate_token(token, &secret).map_err(|e| {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": e.to_string() })))
        })?;
        Ok(JwtClaims(claims))
    }
}

#[allow(non_snake_case)]
pub fn hashPassword(password: &str) -> anyhow::Result<String> {
    use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString}, Argon2};
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
    let claims = Claims { sub: username.to_string(), iat: now, exp: now + 3600 };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .context("JWT encode failed")
}

fn validate_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    Ok(decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::new(Algorithm::HS256))?.claims)
}

/// Extract JWTs from normal HTTP Authorization headers or the browser WebSocket
/// subprotocol used by the frontend. Browsers do not allow arbitrary headers
/// when constructing a WebSocket, so the latter is required for live logs.
fn extract_bearer(parts: &Parts) -> anyhow::Result<&str> {
    if let Some(token) = parts.headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        return Ok(token);
    }

    parts.headers
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok())
        .and_then(|protocols| protocols.split(',').map(str::trim).find_map(|p| p.strip_prefix(WS_AUTH_PROTOCOL_PREFIX)))
        .context("Missing or malformed Authorization header")
}

fn epoch_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn issued_token_validates_with_same_secret() {
        let token = issue_token("admin", "test-secret").unwrap();
        let claims = validate_token(&token, "test-secret").unwrap();
        assert_eq!(claims.sub, "admin");
    }

    #[test]
    fn tampered_token_is_rejected() {
        let token = issue_token("admin", "test-secret").unwrap();
        assert!(validate_token(&format!("{}x", token), "test-secret").is_err());
    }

    #[test]
    fn token_signed_with_wrong_secret_is_rejected() {
        let token = issue_token("admin", "test-secret").unwrap();
        assert!(validate_token(&token, "wrong-secret").is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let now = epoch_now();
        let claims = Claims { sub: "admin".into(), iat: now.saturating_sub(7200), exp: now.saturating_sub(3600) };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"test-secret")).unwrap();
        assert!(validate_token(&token, "test-secret").is_err());
    }

    #[test]
    fn password_hash_verifies_only_original_password() {
        let hash = hashPassword("correct-password").unwrap();
        assert!(verify_password("correct-password", &hash).is_ok());
        assert!(verify_password("wrong-password", &hash).is_err());
    }

    #[test]
    fn websocket_subprotocol_token_is_extracted() {
        let request = Request::builder()
            .header("Sec-WebSocket-Protocol", "chat, mydns-auth.test-token")
            .body(())
            .unwrap();
        let (_, parts) = request.into_parts();
        assert_eq!(extract_bearer(&parts).unwrap(), "test-token");
    }
}
