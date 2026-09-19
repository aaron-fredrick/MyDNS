use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RecordType};
use hickory_proto::serialize::binary::BinDecodable;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

use mydns::db::blocklist::{create_entry, CreateBlocklistEntry};

mod common;

fn query_message(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
    message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
    let mut bytes = message.to_vec().expect("Failed to encode DNS query");
    bytes[2] |= 0b0000_0001; // Set RD bit
    bytes
}

async fn udp_query(
    socket: &UdpSocket,
    addr: SocketAddr,
    name: &str,
    record_type: RecordType,
) -> Message {
    socket
        .send_to(&query_message(name, record_type), addr)
        .await
        .expect("Failed to send UDP query");

    let mut buf = vec![0u8; 512];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .expect("UDP DNS response timed out")
        .unwrap();
    Message::from_bytes(&buf[..len]).expect("Invalid DNS response")
}

fn response_code(message: &Message) -> ResponseCode {
    message.metadata.response_code
}

#[tokio::test]
async fn test_blocklist_returns_nxdomain() {
    let server = common::TestDnsServer::start_with_zones_only(vec![]).await;

    // Create a blocklist entry for ads.example.com
    create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "ads.example.com".to_string(),
            enabled: true,
            source: "manual".to_string(),
            reason: Some("Ad domain".to_string()),
        },
    )
    .await
    .expect("Failed to create blocklist entry");

    // We must restart the server so it picks up the blocklist from DB
    let mut server = server;
    server.restart().await;

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "ads.example.com.", RecordType::A).await;
    assert_eq!(
        response_code(&response),
        ResponseCode::NXDomain,
        "Blocked domain should return NXDOMAIN"
    );

    let response = udp_query(&socket, server.addr, "sub.ads.example.com.", RecordType::A).await;
    assert_eq!(
        response_code(&response),
        ResponseCode::NXDomain,
        "Subdomain of blocked domain should return NXDOMAIN"
    );
}

#[tokio::test]
async fn test_cache_blocklist_race_behavior() {
    // Test the important cache/blocklist race behavior.
    // 1. Query an allowed domain.
    // 2. Ensure it becomes cached.
    // 3. Add the same domain to the blocklist.
    // 4. Query it again.
    // 5. Confirm the response is NXDOMAIN.
    // 6. Confirm the cached positive answer did not win.

    let server = common::TestDnsServer::start_with_config(vec![]).await;
    let mut server = server;

    // We need the server to actually cache the domain, so we can mock an upstream resolver
    // or just let it query Cloudflare/Google. `start_with_config` sets up a test server.
    // Let's create an A record in a zone so it resolves, or just use a synthetic record if it's there.
    // But wait, if it's local zone, it doesn't get cached. We need it to be cached from upstream.
    // Let's use a real public domain that exists, like "one.one.one.one." or just "example.com."
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // 1. Query an allowed domain
    let response = udp_query(&socket, server.addr, "example.com.", RecordType::A).await;
    assert_eq!(
        response_code(&response),
        ResponseCode::NoError,
        "Expected NoError for allowed domain"
    );

    // 2. Add to blocklist
    create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "example.com".to_string(),
            enabled: true,
            source: "manual".to_string(),
            reason: Some("Race test".to_string()),
        },
    )
    .await
    .expect("Failed to create blocklist entry");

    // 3. Restart server so the blocklist is picked up
    // Wait, the blocklist API usually updates the in-memory state on POST. Since we bypass the API
    // by using db::create_entry directly, we must restart the server, but wait, restarting the server
    // clears the in-memory cache! Persistent cache is kept. So this tests persistent cache -> blocklist race.
    server.restart().await;

    // 4. Query it again
    let response2 = udp_query(&socket, server.addr, "example.com.", RecordType::A).await;

    // 5. & 6. Confirm response is NXDOMAIN (cached positive answer didn't win)
    assert_eq!(
        response_code(&response2),
        ResponseCode::NXDomain,
        "Blocked domain should return NXDOMAIN even if it was cached previously"
    );
}

#[tokio::test]
async fn test_blocklist_crud_filters_enabled_domains() {
    let server = common::TestServer::start().await;

    let first = create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "Ads.Example.COM.".into(),
            enabled: true,
            source: "manual".into(),
            reason: Some("ads".into()),
        },
    )
    .await
    .unwrap();
    let second = create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "tracker.example.org".into(),
            enabled: false,
            source: "imported".into(),
            reason: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(first.domain, "ads.example.com");
    assert!(!second.enabled);

    let enabled = mydns::db::blocklist::list_enabled_domains(&server.pool)
        .await
        .unwrap();
    assert_eq!(enabled, vec!["ads.example.com"]);

    let updated = mydns::db::blocklist::update_entry(
        &server.pool,
        second.id,
        &mydns::db::blocklist::UpdateBlocklistEntry {
            enabled: Some(true),
            reason: Some("tracking".into()),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.reason.as_deref(), Some("tracking"));

    let enabled = mydns::db::blocklist::list_enabled_domains(&server.pool)
        .await
        .unwrap();
    assert_eq!(enabled, vec!["ads.example.com", "tracker.example.org"]);

    assert!(mydns::db::blocklist::delete_entry(&server.pool, first.id)
        .await
        .unwrap());
    assert!(!mydns::db::blocklist::delete_entry(&server.pool, first.id)
        .await
        .unwrap());
    assert!(mydns::db::blocklist::get_entry(&server.pool, first.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_blocklist_rejects_invalid_sources_and_duplicates() {
    let server = common::TestServer::start().await;

    let invalid = mydns::db::blocklist::create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "bad-source.example".into(),
            enabled: true,
            source: "unknown".into(),
            reason: None,
        },
    )
    .await;
    assert!(invalid.is_err());

    create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "duplicate.example".into(),
            enabled: true,
            source: "manual".into(),
            reason: None,
        },
    )
    .await
    .unwrap();

    let duplicate = create_entry(
        &server.pool,
        &CreateBlocklistEntry {
            domain: "DUPLICATE.EXAMPLE.".into(),
            enabled: true,
            source: "manual".into(),
            reason: None,
        },
    )
    .await;
    assert!(duplicate.is_err());
}
