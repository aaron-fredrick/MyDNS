use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::Service;
use tower_http::cors::CorsLayer;

use crate::config::AppConfig;
use crate::mydns::web::routes::{build_app, build_cors_layer, origin_header, parse_cors_origins};
use crate::mydns::web::tests::fixtures::TestContext;

#[tokio::test]
async fn origin_header_formatting() {
    // IPv4 with standard port
    let h1 = origin_header("127.0.0.1", 80).unwrap();
    assert_eq!(h1.to_str().unwrap(), "http://127.0.0.1");

    // IPv4 with non-standard port
    let h2 = origin_header("192.168.1.100", 8080).unwrap();
    assert_eq!(h2.to_str().unwrap(), "http://192.168.1.100:8080");

    // Domain name with standard port
    let h3 = origin_header("mydns.local", 80).unwrap();
    assert_eq!(h3.to_str().unwrap(), "http://mydns.local");

    // Domain name with non-standard port
    let h4 = origin_header("mydns.local", 3000).unwrap();
    assert_eq!(h4.to_str().unwrap(), "http://mydns.local:3000");

    // IPv6 address
    let h5 = origin_header("::1", 8080).unwrap();
    assert_eq!(h5.to_str().unwrap(), "http://[::1]:8080");

    let h6 = origin_header("2001:db8::1", 80).unwrap();
    assert_eq!(h6.to_str().unwrap(), "http://[2001:db8::1]");
}

#[tokio::test]
async fn parse_cors_origins_with_specified_host_and_custom_domains() {
    let mut config = AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "admin"
admin_password = "password"
jwt_secret = "secret"
[server]
http_host = "127.0.0.1"
http_port = 8080
[zones]
cors_domains = ["dashboard.local", "dns.home.arpa."]
"#,
    )
    .unwrap();

    let origins = parse_cors_origins(&config).expect("parse cors origins");
    let origin_strs: Vec<&str> = origins.iter().map(|o| o.to_str().unwrap()).collect();

    assert!(origin_strs.contains(&"http://127.0.0.1:8080"));
    assert!(origin_strs.contains(&"http://dashboard.local:8080"));
    assert!(origin_strs.contains(&"http://dns.home.arpa:8080"));

    // Reject invalid cors_domains
    config.cors_domains = vec!["http://invalid-with-scheme.com".into()];
    assert!(parse_cors_origins(&config).is_err());

    config.cors_domains = vec!["invalid.com/path".into()];
    assert!(parse_cors_origins(&config).is_err());

    config.cors_domains = vec!["   ".into()];
    assert!(parse_cors_origins(&config).is_err());
}

#[tokio::test]
async fn parse_cors_origins_with_unspecified_host() {
    let config = AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "admin"
admin_password = "password"
jwt_secret = "secret"
[server]
http_host = "0.0.0.0"
http_port = 8080
"#,
    )
    .unwrap();

    let origins = parse_cors_origins(&config).expect("parse cors origins from 0.0.0.0");
    // Should have enumerated interface IPs
    assert!(!origins.is_empty());
}

#[tokio::test]
async fn build_cors_layer_returns_layer() {
    let config = AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "admin"
admin_password = "password"
jwt_secret = "secret"
[server]
http_host = "127.0.0.1"
http_port = 8080
"#,
    )
    .unwrap();

    let layer = build_cors_layer(&config);
    assert!(layer.is_ok());
}

#[tokio::test]
async fn build_app_applies_security_headers() {
    let ctx = TestContext::new().await;
    let mut app = build_app(Arc::clone(&ctx.state), CorsLayer::permissive());

    let req = Request::builder()
        .uri("/nonexistent-page")
        .body(Body::empty())
        .unwrap();

    let response = app.call(req).await.unwrap();

    // The SPA router serves index.html with 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Check security headers
    let headers = response.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        headers.get("referrer-policy").unwrap(),
        "strict-origin-when-cross-origin"
    );
    assert!(headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("default-src 'self'"));
}

#[tokio::test]
async fn build_app_serves_frontend_root_and_routes() {
    let ctx = TestContext::new().await;
    let mut app = build_app(Arc::clone(&ctx.state), CorsLayer::permissive());

    // Root route
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // SPA fallback route
    let req = Request::builder()
        .uri("/dashboard/settings")
        .body(Body::empty())
        .unwrap();
    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
