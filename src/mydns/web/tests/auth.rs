use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::Request;
use axum::Json;

use crate::error::ApiError;
use crate::mydns::web::auth::{
    epoch_now, extract_bearer, hash_password, issue_token, login, validate_token, verify_password,
    JwtClaims, LoginAttemptTracker, LoginRateLimiter, LoginRequest, LOGIN_WINDOW_SECONDS,
    MAX_LOGIN_ATTEMPTS, MAX_TRACKED_LOGIN_IPS, WS_AUTH_PROTOCOL_PREFIX,
};
use crate::mydns::web::tests::fixtures::TestContext;

#[tokio::test]
async fn password_hashing_and_verification() {
    let password = "super-secret-password-123";
    let hash = hash_password(password).expect("hashing should succeed");
    assert!(!hash.is_empty());
    assert_ne!(password, hash);

    assert!(verify_password(password, &hash).is_ok());
    assert!(verify_password("wrong-password", &hash).is_err());
    assert!(verify_password(password, "invalid$argon$hash").is_err());
}

#[tokio::test]
async fn token_issuance_and_validation() {
    let secret = "test-secret-key-12345678901234567890";
    let username = "testuser";

    let token = issue_token(username, secret).expect("issue token");
    assert!(!token.is_empty());

    let claims = validate_token(&token, secret).expect("validate token");
    assert_eq!(claims.sub, username);
    assert!(claims.exp > claims.iat);
    assert_eq!(claims.exp - claims.iat, 3600);

    // Wrong secret
    assert!(validate_token(&token, "wrong-secret-key-12345678901234567890").is_err());

    // Tampered token
    let tampered = format!("{}extra", token);
    assert!(validate_token(&tampered, secret).is_err());

    // Invalid format
    assert!(validate_token("not-a-jwt", secret).is_err());
}

#[tokio::test]
async fn extract_bearer_from_headers() {
    // 1. Standard Authorization: Bearer <token>
    let req = Request::builder()
        .header("Authorization", "Bearer my-secret-jwt-token")
        .body(())
        .unwrap();
    let (parts, _) = req.into_parts();
    let token = extract_bearer(&parts).expect("extract bearer");
    assert_eq!(token, "my-secret-jwt-token");

    // 2. Non-bearer Authorization header fails
    let req = Request::builder()
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .body(())
        .unwrap();
    let (parts, _) = req.into_parts();
    assert!(extract_bearer(&parts).is_err());

    // 3. WebSocket subprotocol header
    let req = Request::builder()
        .header(
            "Sec-WebSocket-Protocol",
            format!("{}ws-jwt-token", WS_AUTH_PROTOCOL_PREFIX),
        )
        .body(())
        .unwrap();
    let (parts, _) = req.into_parts();
    let token = extract_bearer(&parts).expect("extract ws protocol token");
    assert_eq!(token, "ws-jwt-token");

    // 4. WebSocket subprotocol header among multiple protocols
    let req = Request::builder()
        .header(
            "Sec-WebSocket-Protocol",
            format!("chat, {}multi-protocol-token, log", WS_AUTH_PROTOCOL_PREFIX),
        )
        .body(())
        .unwrap();
    let (parts, _) = req.into_parts();
    let token = extract_bearer(&parts).expect("extract ws token from multiple protocols");
    assert_eq!(token, "multi-protocol-token");

    // 5. Missing headers fails
    let req = Request::builder().body(()).unwrap();
    let (parts, _) = req.into_parts();
    assert!(extract_bearer(&parts).is_err());
}

#[tokio::test]
async fn login_rate_limiter_allows_under_limit_and_blocks_excess() {
    let limiter = LoginRateLimiter::default();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));

    for _ in 0..MAX_LOGIN_ATTEMPTS {
        assert!(limiter.check_rate_limit(ip).await.is_ok());
    }

    // Next attempt exceeds limit
    let err = limiter.check_rate_limit(ip).await.unwrap_err();
    match err {
        ApiError::TooManyRequests(msg) => {
            assert!(msg.contains("Too many login attempts"));
        }
        other => panic!("expected TooManyRequests, got {other:?}"),
    }

    // Different IP is not affected
    let other_ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 20));
    assert!(limiter.check_rate_limit(other_ip).await.is_ok());
}

#[tokio::test]
async fn login_rate_limiter_resets_after_window_expires() {
    let limiter = LoginRateLimiter::new();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 11));

    {
        let mut trackers = limiter.trackers.write().await;
        trackers.insert(
            ip,
            LoginAttemptTracker {
                attempts: MAX_LOGIN_ATTEMPTS,
                // Simulate window start 400 seconds ago (> 300s window)
                window_start: epoch_now().saturating_sub(LOGIN_WINDOW_SECONDS + 100),
            },
        );
    }

    // Should reset window and succeed
    assert!(limiter.check_rate_limit(ip).await.is_ok());
}

#[tokio::test]
async fn login_rate_limiter_prunes_on_capacity() {
    let limiter = LoginRateLimiter::new();
    let now = epoch_now();

    {
        let mut trackers = limiter.trackers.write().await;
        // Populate up to MAX_TRACKED_LOGIN_IPS
        for i in 0..MAX_TRACKED_LOGIN_IPS {
            let ip = IpAddr::V4(Ipv4Addr::from((i as u32) + 1));
            trackers.insert(
                ip,
                LoginAttemptTracker {
                    attempts: 1,
                    // Half expired, half active
                    window_start: if i % 2 == 0 {
                        now.saturating_sub(LOGIN_WINDOW_SECONDS + 10)
                    } else {
                        now
                    },
                },
            );
        }
    }

    let new_ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1));
    assert!(limiter.check_rate_limit(new_ip).await.is_ok());

    let trackers = limiter.trackers.read().await;
    assert!(trackers.len() < MAX_TRACKED_LOGIN_IPS);
}

#[tokio::test]
async fn login_handler_success_and_failures() {
    let ctx = TestContext::new().await;
    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);

    // 1. Success login
    let req = LoginRequest {
        username: "admin".into(),
        password: "admin".into(),
    };
    let res = login(
        axum::extract::State(Arc::clone(&ctx.state)),
        axum::extract::ConnectInfo(client_addr),
        Json(req),
    )
    .await;
    assert!(res.is_ok());
    let token = res.unwrap().0.token;
    assert!(!token.is_empty());

    // 2. Wrong password
    let req = LoginRequest {
        username: "admin".into(),
        password: "wrong-password".into(),
    };
    let res = login(
        axum::extract::State(Arc::clone(&ctx.state)),
        axum::extract::ConnectInfo(client_addr),
        Json(req),
    )
    .await;
    assert!(matches!(res, Err(ApiError::Unauthorized(_))));

    // 3. Unknown user
    let req = LoginRequest {
        username: "nonexistent".into(),
        password: "admin".into(),
    };
    let res = login(
        axum::extract::State(Arc::clone(&ctx.state)),
        axum::extract::ConnectInfo(client_addr),
        Json(req),
    )
    .await;
    assert!(matches!(res, Err(ApiError::Unauthorized(_))));
}

#[tokio::test]
async fn jwt_claims_extractor_from_request_parts() {
    let ctx = TestContext::new().await;
    let token = issue_token("admin", &ctx.state.config.read().await.jwt_secret).unwrap();

    // 1. Valid bearer token
    let req = Request::builder()
        .header("Authorization", format!("Bearer {token}"))
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    let claims = JwtClaims::from_request_parts(&mut parts, &ctx.state)
        .await
        .expect("extract JwtClaims");
    assert_eq!(claims.0.sub, "admin");

    // 2. Missing authorization header
    let req = Request::builder().body(()).unwrap();
    let (mut parts, _) = req.into_parts();
    let rejection = JwtClaims::from_request_parts(&mut parts, &ctx.state)
        .await
        .unwrap_err();
    assert_eq!(rejection.0, axum::http::StatusCode::UNAUTHORIZED);

    // 3. Invalid token string
    let req = Request::builder()
        .header("Authorization", "Bearer invalid-token")
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    let rejection = JwtClaims::from_request_parts(&mut parts, &ctx.state)
        .await
        .unwrap_err();
    assert_eq!(rejection.0, axum::http::StatusCode::UNAUTHORIZED);
}
