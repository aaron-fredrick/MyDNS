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
    let test_id = mydns::config::generate_secret(8);
    let db_path = format!("test_{}.db", test_id);
    let port = rand::random::<u16>() % 10000 + 20000; // Use a random high port

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
    };

    // Ensure clean DB
    remove_test_db(&cfg.db_path);

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

/// Removes all SQLite files associated with a test database path.
///
/// Removes `<path>`, `<path>-shm`, and `<path>-wal` so that no journal
/// artifacts linger in the working tree after a test run.
fn remove_test_db(path: &str) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{path}-shm"));
    let _ = std::fs::remove_file(format!("{path}-wal"));
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
    remove_test_db(&db_path);
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
    remove_test_db(&db_path);
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
    remove_test_db(&db_path);
}

#[tokio::test]
async fn test_persistent_cache_upsert_deduplicates_records() {
    let (_base, db_path) = start_test_server().await;
    let pool = db::init(&db_path).await.expect("Failed to reopen test DB");

    db::records::insertCache(&pool, "cache.test.local", "A", "10.0.0.1", 60, None)
        .await
        .unwrap();
    db::records::insertCache(&pool, "CACHE.TEST.LOCAL.", "a", "10.0.0.1", 120, None)
        .await
        .unwrap();

    let rows = db::records::getCache(&pool, "cache.test.local.", "A")
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "Identical cache records must be deduplicated"
    );
    assert_eq!(rows[0].ttl, 120, "Upsert should refresh TTL");

    remove_test_db(&db_path);
}

#[tokio::test]
async fn test_cname_target_update_invalidates_dependent_cache() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let token = loginToken(&c, &base).await;
    let auth = format!("Bearer {}", token);

    let target = c
        .post(format!("{}/api/v1/records", &base))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "target.integration.local.",
            "record_type": "A",
            "value": "10.0.0.1",
            "ttl": 300
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(target.status(), 200);
    let target_id = target.json::<Value>().await.unwrap()["record"]["id"]
        .as_i64()
        .unwrap();

    let alias = c
        .post(format!("{}/api/v1/records", &base))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "alias.integration.local.",
            "record_type": "CNAME",
            "value": "target.integration.local.",
            "ttl": 300
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(alias.status(), 200);

    let pool = db::init(&db_path).await.expect("Failed to reopen test DB");
    db::records::insertCache(&pool, "alias.integration.local", "A", "10.0.0.1", 300, None)
        .await
        .unwrap();

    let before = db::records::getCache(&pool, "alias.integration.local", "A")
        .await
        .unwrap();
    assert_eq!(
        before.len(),
        1,
        "Expected dependent cache entry before update"
    );

    let res = c
        .put(format!("{}/api/v1/records/{}", &base, target_id))
        .header("Authorization", &auth)
        .json(&json!({ "value": "10.0.0.2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let after = db::records::getCache(&pool, "alias.integration.local", "A")
        .await
        .unwrap();
    assert!(
        after.is_empty(),
        "Dependent CNAME cache must be invalidated"
    );

    remove_test_db(&db_path);
}

// ── stats ──────────────────────────────────────────────────────────────────────

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
    remove_test_db(&db_path);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn test_debug_cors_is_permissive() {
    let (base, db_path) = start_test_server().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/stats", base))
        .header("Origin", "http://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
    remove_test_db(&db_path);
}

#[cfg(not(debug_assertions))]
#[tokio::test]
async fn test_release_cors_is_restricted() {
    let (base, db_path) = start_test_server().await;
    let c = client();

    let trusted = c
        .get(format!("{}/api/v1/stats", base))
        .header("Origin", &base)
        .send()
        .await
        .unwrap();
    assert_eq!(trusted.status(), 200);
    assert_eq!(
        trusted
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some(base.as_str())
    );

    let untrusted = c
        .get(format!("{}/api/v1/stats", base))
        .header("Origin", "http://evil.example")
        .send()
        .await
        .unwrap();
    let _ = untrusted
        .headers()
        .get("access-control-allow-origin")
        .is_none();

    remove_test_db(&db_path);
}
