//! Additional API integration coverage for edge cases and less-traveled management paths.

mod common;

use common::TestServer;
use reqwest::Client;
use serde_json::{json, Value};

fn client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

#[tokio::test]
async fn settings_update_validates_and_persists_supported_values() {
    let server = TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;

    let response = c
        .put(format!("{}/api/v1/settings", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({
            "resolver_mode": "recursive",
            "resolver_priority": "router_first",
            "cloudflare_dns": "9.9.9.9:53",
            "router_dns": "192.0.2.53:53"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["resolver_mode"], "recursive");
    assert_eq!(body["resolver_priority"], "router_first");
    assert_eq!(body["cloudflare_dns"], "9.9.9.9:53");
    assert_eq!(body["router_dns"], "192.0.2.53:53");

    assert_eq!(
        mydns::db::get_setting(&server.pool, "resolver_mode")
            .await
            .unwrap(),
        Some("recursive".into())
    );
    assert_eq!(
        mydns::db::get_setting(&server.pool, "resolver_priority")
            .await
            .unwrap(),
        Some("router_first".into())
    );
}

#[tokio::test]
async fn settings_reject_invalid_values_without_mutating_config() {
    let server = TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;

    let invalid_mode = c
        .put(format!("{}/api/v1/settings", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({"resolver_mode": "not-a-mode"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_mode.status(), 400);

    let invalid_address = c
        .put(format!("{}/api/v1/settings", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({"cloudflare_dns": "not-an-address"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_address.status(), 400);
}

#[tokio::test]
async fn zones_reject_invalid_names_and_duplicate_zones() {
    let server = TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;

    for name in [".", "", "bad/name", "bad:name", "-bad.example", "bad-.example"] {
        let response = c
            .post(format!("{}/api/v1/zones", server.base_url))
            .header("Authorization", &auth)
            .json(&json!({"name": name}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "zone {name:?} should be rejected");
    }

    let created = c
        .post(format!("{}/api/v1/zones", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({"name": "Lab.Example.ARPA."}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 200);

    let duplicate = c
        .post(format!("{}/api/v1/zones", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({"name": "lab.example.arpa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(duplicate.status(), 400);

    let missing = c
        .delete(format!("{}/api/v1/zones/missing.example"))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
}

#[tokio::test]
async fn cache_api_covers_listing_and_delete_paths() {
    let server = TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;

    mydns::db::records::insert_cache(
        &server.pool,
        "api-cache.home.arpa",
        "A",
        "192.0.2.20",
        300,
        None,
    )
    .await
    .unwrap();

    let list = c
        .get(format!("{}/api/v1/cache", server.base_url))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), 200);
    let entries: Vec<Value> = list.json().await.unwrap();
    assert!(entries.iter().any(|entry| entry["name"] == "api-cache.home.arpa"));

    let deleted = c
        .delete(format!(
            "{}/api/v1/cache/api-cache.home.arpa/A",
            server.base_url
        ))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), 204);

    let invalid_type = c
        .delete(format!(
            "{}/api/v1/cache/api-cache.home.arpa/NOT_A_RECORD",
            server.base_url
        ))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_type.status(), 400);

    let cleared = c
        .delete(format!("{}/api/v1/cache", server.base_url))
        .header("Authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(cleared.status(), 204);
}

#[tokio::test]
async fn stats_history_handles_defaults_invalid_ranges_and_future_end() {
    let server = TestServer::start().await;
    let c = client();

    let default = c
        .get(format!("{}/api/v1/stats/history", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(default.status(), 200);

    let invalid = c
        .get(format!(
            "{}/api/v1/stats/history?from=not-a-timestamp",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), 400);

    let reversed = c
        .get(format!(
            "{}/api/v1/stats/history?from=2030-01-02T00:00:00Z&to=2030-01-01T00:00:00Z",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(reversed.status(), 400);

    let future = c
        .get(format!(
            "{}/api/v1/stats/history?from=2000-01-01T00:00:00Z&to=2099-01-01T00:00:00Z",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(future.status(), 200);
    let body: Value = future.json().await.unwrap();
    assert!(body["to"].as_str().is_some());
}

#[tokio::test]
async fn protected_routes_reject_malformed_bearer_tokens() {
    let server = TestServer::start().await;
    let c = client();

    for header in ["Bearer", "Bearer nope", "Basic nope"] {
        let response = c
            .get(format!("{}/api/v1/settings", server.base_url))
            .header("Authorization", header)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401, "header {header:?}");
    }
}

#[tokio::test]
async fn records_reject_invalid_payloads_through_real_http_stack() {
    let server = TestServer::start().await;
    let c = client();
    let auth = server.auth_header(&c).await;

    let unsupported = c
        .post(format!("{}/api/v1/records", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "invalid.home.arpa",
            "record_type": "SRV",
            "value": "invalid",
            "ttl": 60
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(unsupported.status(), 400);

    let bad_ttl = c
        .post(format!("{}/api/v1/records", server.base_url))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "invalid.home.arpa",
            "record_type": "A",
            "value": "192.0.2.10",
            "ttl": 0
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_ttl.status(), 400);
}
