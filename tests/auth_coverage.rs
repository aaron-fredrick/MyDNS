//! Auth coverage tests.
//!
//! Verifies that every authenticated API route returns HTTP 401 when called
//! without a JWT bearer token, and that the intentionally unauthenticated
//! `/stats` endpoint returns 200 without one.

use mydns::{config::AppConfig, db, dns, state::AppState, web};
use reqwest::Client;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn start_test_server() -> (String, String) {
    let test_id = mydns::config::generate_secret(8);
    let db_path = format!("auth_cov_{}.db", test_id);
    let port = rand::random::<u16>() % 10000 + 40000;

    let cfg = AppConfig {
        bind_host: "127.0.0.1".parse().unwrap(),
        dns_port: port + 1,
        http_host: "127.0.0.1".parse().unwrap(),
        http_port: port,
        cors_domains: vec!["mydns.local".to_string()],
        dashboard_domain: "mydns.local".to_string(),
        db_path: db_path.clone(),
        jwt_secret: mydns::config::generate_secret(64),
        admin_username: "admin".to_string(),
        admin_password: "changeme123".to_string(),
        resolver_priority: mydns::config::ResolverPriority::CloudflareFirst,
        cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
        router_dns: None,
        run_as_user: "nobody".to_string(),
        run_as_group: "nobody".to_string(),
        allowed_zones: vec![],
    };

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{db_path}-shm"));
    let _ = std::fs::remove_file(format!("{db_path}-wal"));

    let pool = db::init(&cfg.db_path)
        .await
        .expect("Failed to init test DB");

    let hash = mydns::web::auth::hashPassword(&cfg.admin_password).unwrap();
    db::records::seedAdmin(&pool, &cfg.admin_username, &hash)
        .await
        .unwrap();

    let upstream = dns::upstream::UpstreamResolver::fromConfig(
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
    )
    .unwrap();

    let (log_tx, _) = tokio::sync::broadcast::channel(1024);
    let cancel = CancellationToken::new();
    let state = AppState::new(pool, cfg, upstream, log_tx, cancel.clone());

    let server_state = Arc::clone(&state);
    let server_cancel = cancel.clone();
    tokio::spawn(async move {
        let _ = web::server::run(server_state, server_cancel).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    (format!("http://127.0.0.1:{port}"), db_path)
}

fn remove_test_db(path: &str) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{path}-shm"));
    let _ = std::fs::remove_file(format!("{path}-wal"));
}

type RequestBuilder = fn(&Client, String) -> reqwest::RequestBuilder;

/// Asserts that every authenticated endpoint returns 401 without a token,
/// and the unauthenticated /stats endpoint returns 200.
#[tokio::test]
async fn test_all_protected_routes_require_auth() {
    let (base, db_path) = start_test_server().await;
    let c = Client::new();
    let api = format!("{base}/api/v1");

    // -- Unauthenticated routes (must pass) --
    assert_eq!(
        c.get(format!("{api}/stats")).send().await.unwrap().status(),
        200,
        "/stats should not require auth"
    );

    // -- Authenticated routes (must return 401 without token) --
    let protected: &[(&str, RequestBuilder)] = &[
        ("GET /records", |c, u| c.get(u)),
        ("POST /records", |c, u| c.post(u)),
        ("PUT /records/1", |c, u| c.put(u)),
        ("DELETE /records/1", |c, u| c.delete(u)),
        ("GET /settings", |c, u| c.get(u)),
        ("PUT /settings", |c, u| c.put(u)),
        ("GET /cache", |c, u| c.get(u)),
        ("DELETE /cache", |c, u| c.delete(u)),
    ];

    for (label, build) in protected {
        let url = if label.contains("/records") {
            if label.contains("/records/1") {
                format!("{api}/records/1")
            } else {
                format!("{api}/records")
            }
        } else if label.contains("/settings") {
            format!("{api}/settings")
        } else {
            format!("{api}/cache")
        };

        let status = build(&c, url).send().await.unwrap().status();
        assert_eq!(status, 401, "{label} should return 401 without auth token");
    }

    remove_test_db(&db_path);
}
