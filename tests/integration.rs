//! Integration tests for the MyDNS API.
//!
//! These tests exercise the full HTTP stack (login → auth → CRUD) against a
//! real in-process server bound on a random port.  Run with:
//!
//! ```sh
//! cargo test --test integration
//! ```

use std::sync::Arc;
use tokio::sync::OnceCell;
use mydns::{AppConfig, state::AppState, web, db, dns, web::auth::hashPassword};
use tokio_util::sync::CancellationToken;
use serde_json::{json, Value};

const BASE: &str = "http://127.0.0.1:8181/api/v1";
static INIT: OnceCell<()> = OnceCell::const_new();

async fn ensure_server_running() {
    INIT.get_or_init(|| async {
        // Setup a test configuration
        let mut cfg = AppConfig::fromEnv();
        cfg.http_port = 8181;
        cfg.dns_port = 1053; // Non-privileged port for tests
        cfg.db_path = "test_integration.db".to_string();
        
        // Ensure clean DB for tests
        let _ = std::fs::remove_file(&cfg.db_path);

        let pool = db::init(&cfg.db_path).await.expect("Failed to init test DB");
        
        // Seed admin
        let hash = hashPassword("changeme123").expect("Failed to hash");
        db::records::seedAdmin(&pool, "admin", &hash).await.expect("Failed to seed");

        let upstream = dns::upstream::UpstreamResolver::fromConfig(
            cfg.resolver_priority.clone(),
            cfg.cloudflare_dns,
            cfg.router_dns,
        ).expect("Failed to build resolver");

        let (log_tx, _) = tokio::sync::broadcast::channel(1024);
        let cancel = CancellationToken::new();
        let state = AppState::new(pool.clone(), cfg, upstream, log_tx, cancel.clone());

        let server_state = Arc::clone(&state);
        let server_cancel = cancel.clone();
        
        // Spawn HTTP server
        tokio::spawn(async move {
            let _ = web::server::run(server_state, server_cancel).await;
        });

        // Give it a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }).await;
}

fn client() -> Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

// ── helper ────────────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
async fn loginToken(c: &Client) -> String {
    ensure_server_running().await;
    let res = c
        .post(format!("{}/auth/login", BASE))
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
    ensure_server_running().await;
    let c = client();
    let res = c
        .post(format!("{}/auth/login", BASE))
        .json(&json!({"username": "admin", "password": "wrongpassword"}))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn test_records_unauthenticated_returns_401() {
    ensure_server_running().await;
    let c = client();
    let res = c
        .get(format!("{}/records", BASE))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
}

// ── records CRUD ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_records_full_crud_cycle() {
    let c = client();
    let token = loginToken(&c).await;
    let auth = format!("Bearer {}", token);

    // CREATE
    let res = c
        .post(format!("{}/records", BASE))
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
        .get(format!("{}/records", BASE))
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
        .delete(format!("{}/records/{}", BASE, id))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // LIST again — should be gone
    let res = c
        .get(format!("{}/records", BASE))
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
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stats_returned_without_auth() {
    ensure_server_running().await;
    // Stats endpoint is intentionally public (no sensitive info).
    let c = client();
    let res = c
        .get(format!("{}/stats", BASE))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["uptime_secs"].is_number());
    assert!(body["cache_hits"].is_number());
}
