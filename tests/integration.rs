//! Integration tests for the MyDNS API.
//!
//! These tests exercise the full HTTP stack (login → auth → CRUD) against a
//! real in-process server bound on a random port.  Run with:
//!
//! ```sh
//! cargo test --test integration
//! ```

use mydns::{config::AppConfig, db, dns, state::AppState, web, web::auth::hashPassword};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn start_test_server() -> (String, String) {
    let test_id = mydns::config::generateSecret(8);
    let db_path = format!("test_{}.db", test_id);
    let port = rand::random::<u16>() % 10000 + 20000; // Use a random high port

    let cfg = AppConfig {
        bind_host: "127.0.0.1".parse().unwrap(),
        dns_port: port + 1,

        http_host: "127.0.0.1".parse().unwrap(),
        http_port: port,

        db_path: db_path.clone(),
        jwt_secret: mydns::config::generateSecret(64),
        admin_username: "admin".to_string(),
        admin_password: "changeme123".to_string(),
        resolver_priority: mydns::config::ResolverPriority::CloudflareFirst,
        cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
        router_dns: None,
    };

    // Ensure clean DB
    let _ = std::fs::remove_file(&cfg.db_path);

    let pool = db::init(&cfg.db_path)
        .await
        .expect("Failed to init test DB");

    // Seed admin
    let hash = hashPassword(&cfg.admin_password).expect("Failed to hash");
    db::records::seedAdmin(&pool, &cfg.admin_username, &hash)
        .await
        .expect("Failed to seed");

    let upstream = dns::upstream::UpstreamResolver::fromConfig(
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
    )
    .expect("Failed to build resolver");

    let (log_tx, _) = tokio::sync::broadcast::channel(1024);
    let cancel = CancellationToken::new();
    let state = AppState::new(pool.clone(), cfg, upstream, log_tx, cancel.clone());

    let server_state = Arc::clone(&state);
    let server_cancel = cancel.clone();

    tokio::spawn(async move {
        let _ = web::server::run(server_state, server_cancel).await;
    });

    // Give it a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    (format!("http://127.0.0.1:{}", port), db_path)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

#[allow(non_snake_case)]
async fn loginToken(c: &Client, base: &str) -> String {
    let res = c
        .post(format!("{}/api/v1/auth/login", base))
        .json(&json!({"username": "admin", "password": "changeme123"}))
        .send()
        .await
        .expect("Login request failed");
    assert_eq!(res.status(), 200, "Expected 200 on valid login");
    let body: Value = res.json().await.unwrap();
    body["token"].as_str().unwrap().to_owned()
}

// ── auth ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let res = c
        .post(format!("{}/api/v1/auth/login", base))
        .json(&json!({"username": "admin", "password": "wrongpassword"}))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_records_unauthenticated_returns_401() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/records", base))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
    let _ = std::fs::remove_file(db_path);
}

// ── records CRUD ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_records_full_crud_cycle() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let token = loginToken(&c, &base).await;
    let auth = format!("Bearer {}", token);

    // CREATE
    let res = c
        .post(format!("{}/api/v1/records", &base))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "test.integration.local.",
            "record_type": "A",
            "value": "10.0.0.1",
            "ttl": 60
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "Create should return 200");
    let body: Value = res.json().await.unwrap();
    let id = body["record"]["id"].as_i64().unwrap();

    // LIST — ensure the new record appears
    let res = c
        .get(format!("{}/api/v1/records", &base))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let found = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == id);
    assert!(found, "Newly created record should appear in list");

    // DELETE
    let res = c
        .delete(format!("{}/api/v1/records/{}", &base, id))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // LIST again — should be gone
    let res = c
        .get(format!("{}/api/v1/records", &base))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let still_present = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == id);
    assert!(!still_present, "Deleted record should not appear in list");
    let _ = std::fs::remove_file(db_path);
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stats_returned_without_auth() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/stats", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["uptime_secs"].is_number());
    assert!(body["cache_hits"].is_number());
    let _ = std::fs::remove_file(db_path);
}
