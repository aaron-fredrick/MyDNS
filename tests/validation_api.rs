//! HTTP-level tests for DNS record input validation.

mod common;

use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_record_api_rejects_invalid_inputs() {
    let server = common::TestServer::start().await;
    let client = Client::new();
    let auth = server.auth_header(&client).await;
    let base = &server.base_url;

    let cases = [
        (
            "empty name",
            json!({"name":"","record_type":"A","value":"192.0.2.1","ttl":300}),
        ),
        (
            "bad A value",
            json!({"name":"bad.local","record_type":"A","value":"not-an-ip","ttl":300}),
        ),
        (
            "bad AAAA value",
            json!({"name":"bad.local","record_type":"AAAA","value":"not-an-ip","ttl":300}),
        ),
        (
            "bad CNAME target",
            json!({"name":"bad.local","record_type":"CNAME","value":"not..a..name","ttl":300}),
        ),
        (
            "unsupported type",
            json!({"name":"bad.local","record_type":"SRV","value":"hello","ttl":300}),
        ),
        (
            "zero TTL",
            json!({"name":"bad.local","record_type":"A","value":"192.0.2.1","ttl":0}),
        ),
        (
            "excessive TTL",
            json!({"name":"bad.local","record_type":"A","value":"192.0.2.1","ttl":86401}),
        ),
        (
            "priority on A",
            json!({"name":"bad.local","record_type":"A","value":"192.0.2.1","ttl":300,"priority":10}),
        ),
    ];

    for (label, body) in cases {
        let response = client
            .post(format!("{base}/api/v1/records"))
            .header("Authorization", &auth)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "case should be rejected: {label}");
    }

    let valid = client
        .post(format!("{base}/api/v1/records"))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "valid.local.",
            "record_type": "A",
            "value": "192.0.2.1",
            "ttl": 300
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(valid.status(), 200);
}

#[tokio::test]
async fn test_record_api_validates_effective_update_state() {
    let server = common::TestServer::start().await;
    let client = Client::new();
    let auth = server.auth_header(&client).await;
    let base = &server.base_url;

    let created = client
        .post(format!("{base}/api/v1/records"))
        .header("Authorization", &auth)
        .json(&json!({
            "name": "update.local",
            "record_type": "A",
            "value": "192.0.2.1",
            "ttl": 300
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 200);
    let id = created.json::<serde_json::Value>().await.unwrap()["record"]["id"]
        .as_i64()
        .unwrap();

    let invalid_value = client
        .put(format!("{base}/api/v1/records/{id}"))
        .header("Authorization", &auth)
        .json(&json!({"value":"not-an-ip"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_value.status(), 400);

    let invalid_type = client
        .put(format!("{base}/api/v1/records/{id}"))
        .header("Authorization", &auth)
        .json(&json!({"record_type":"SRV"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_type.status(), 400);

    let valid_update = client
        .put(format!("{base}/api/v1/records/{id}"))
        .header("Authorization", &auth)
        .json(&json!({"value":"192.0.2.2","ttl":600}))
        .send()
        .await
        .unwrap();
    assert_eq!(valid_update.status(), 200);
}
