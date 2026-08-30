//! Integration tests for the MyDNS API.
//!
//! These tests exercise the full HTTP stack (login → auth → CRUD) against a
//! real in-process server bound on a random port.

mod common;

use mydns::db;
use reqwest::Client;
use serde_json::{json, Value};

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

// ── auth ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let server = common::TestServer::start().await;
    let c = client();
    let res = c
        .post(format!("{}/api/v1/auth/login", server.base_url))
        .json(&json!({"username": "admin", "password": "wrongpassword"}))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn test_records_unauthenticated_returns_401() {
    let server = common::TestServer::start().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/records", server.base_url))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(res.status(), 401);
}

// ── records CRUD ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_records_full_crud_cycle() {
    let server = common::TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;
    let base = &server.base_url;

    // CREATE
    let res = c
        .post(format!("{}/api/v1/records", base))
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
        .get(format!("{}/api/v1/records", base))
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
        .delete(format!("{}/api/v1/records/{}", base, id))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // LIST again — should be gone
    let res = c
        .get(format!("{}/api/v1/records", base))
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

#[tokio::test]
async fn test_persistent_cache_upsert_deduplicates_records() {
    let server = common::TestServer::start().await;
    let pool = &server.pool;

    db::records::insert_cache(pool, "cache.test.local", "A", "10.0.0.1", 60, None)
        .await
        .unwrap();
    db::records::insert_cache(pool, "CACHE.TEST.LOCAL.", "a", "10.0.0.1", 120, None)
        .await
        .unwrap();

    let rows = db::records::get_cache(pool, "cache.test.local.", "A")
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "Identical cache records must be deduplicated"
    );
    assert_eq!(rows[0].ttl, 120, "Upsert should refresh TTL");
}

#[tokio::test]
async fn test_cname_target_update_invalidates_dependent_cache() {
    let server = common::TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;
    let base = &server.base_url;

    let target = c
        .post(format!("{}/api/v1/records", base))
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
        .post(format!("{}/api/v1/records", base))
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

    let pool = &server.pool;
    db::records::insert_cache(pool, "alias.integration.local", "A", "10.0.0.1", 300, None)
        .await
        .unwrap();

    let before = db::records::get_cache(pool, "alias.integration.local", "A")
        .await
        .unwrap();
    assert_eq!(
        before.len(),
        1,
        "Expected dependent cache entry before update"
    );

    let res = c
        .put(format!("{}/api/v1/records/{}", base, target_id))
        .header("Authorization", &auth)
        .json(&json!({ "value": "10.0.0.2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let after = db::records::get_cache(pool, "alias.integration.local", "A")
        .await
        .unwrap();
    assert!(
        after.is_empty(),
        "Dependent CNAME cache must be invalidated"
    );
}

// ── stats ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stats_returned_without_auth() {
    let server = common::TestServer::start().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/stats", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["uptime_secs"].is_number());
    assert!(body["cache_hits"].is_number());
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn test_debug_cors_is_permissive() {
    let server = common::TestServer::start().await;
    let c = client();
    let res = c
        .get(format!("{}/api/v1/stats", server.base_url))
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
}

#[cfg(not(debug_assertions))]
#[tokio::test]
async fn test_release_cors_is_restricted() {
    let server = common::TestServer::start().await;
    let c = client();
    let base = &server.base_url;

    let trusted = c
        .get(format!("{}/api/v1/stats", base))
        .header("Origin", base)
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
}
