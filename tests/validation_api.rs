//! HTTP-level tests for DNS record input validation.

use mydns::{config::AppConfig, db, dns, state::AppState, web, web::auth::hashPassword};
use mydns::dns::record_index::RecordIndex;
use mydns::dns::zone_trie::ZoneTrie;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn start_test_server() -> (String, String) {
    let test_id = mydns::config::generate_secret(8);
    let db_path = format!("test_{}.db", test_id);
    let port = rand::random::<u16>() % 10000 + 20000;
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
        resolver_mode: mydns::config::ResolverMode::Forwarding,
        resolver_priority: mydns::config::ResolverPriority::CloudflareFirst,
        cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
        router_dns: None,
        run_as_user: "nobody".to_string(),
        run_as_group: "nobody".to_string(),
        allowed_zones: vec![],
        root_hints: vec![],
    };

    let _ = std::fs::remove_file(&db_path);
    let pool = db::init(&db_path).await.unwrap();
    let hash = hashPassword(&cfg.admin_password).unwrap();
    db::records::seedAdmin(&pool, &cfg.admin_username, &hash)
        .await
        .unwrap();
    let upstream = dns::upstream::UpstreamResolver::fromConfig(
        cfg.resolver_mode.clone(),
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
        cfg.root_hints.clone(),
    )
    .unwrap();
    let (log_tx, _) = tokio::sync::broadcast::channel(128);
    let cancel = CancellationToken::new();
    let zone_trie = ZoneTrie::from_zones(&cfg.allowed_zones);
    let record_index = RecordIndex::load_from_db(&pool)
        .await
        .expect("Failed to load record index");
    let state = AppState::new(pool, cfg, upstream, log_tx, cancel.clone(), record_index, zone_trie);
    let server_state = Arc::clone(&state);
    let server_cancel = cancel.clone();
    tokio::spawn(async move {
        let _ = web::server::run(server_state, server_cancel).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    (format!("http://127.0.0.1:{}", port), db_path)
}

async fn login(client: &Client, base: &str) -> String {
    let response = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({"username": "admin", "password": "changeme123"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    response.json::<serde_json::Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn test_record_api_rejects_invalid_inputs() {
    let (base, db_path) = start_test_server().await;
    let client = Client::new();
    let auth = format!("Bearer {}", login(&client, &base).await);

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

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_record_api_validates_effective_update_state() {
    let (base, db_path) = start_test_server().await;
    let client = Client::new();
    let auth = format!("Bearer {}", login(&client, &base).await);

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

    let _ = std::fs::remove_file(db_path);
}
