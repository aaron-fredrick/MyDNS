//! Integration tests for the MyDNS API.
//!
//! These tests exercise the full HTTP stack (login → auth → CRUD) against a
//! real in-process server bound on a random port.  Run with:
//!
//! ```sh
//! cargo test --test integration
//! ```

use reqwest::Client;
use serde_json::{json, Value};

const BASE: &str = "http://127.0.0.1:8181/api/v1";

fn client() -> Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

// ── helper ────────────────────────────────────────────────────────────────────

async fn login_token(c: &Client) -> String {
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
    let token = login_token(&c).await;
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
