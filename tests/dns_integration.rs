//! Integration tests for the DNS wire server.
//!
//! These tests exercise the real DNS UDP/TCP listeners and the resolution
//! outcome model against an isolated SQLite database.

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RecordType};
use hickory_proto::serialize::binary::BinDecodable;
use mydns::{
    config::{AppConfig, ResolverPriority},
    db, dns,
    state::AppState,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::sync::CancellationToken;

async fn start_dns_server() -> (
    SocketAddr,
    String,
    CancellationToken,
    tokio::task::JoinHandle<()>,
) {
    let test_id = mydns::config::generate_secret(8);
    let db_path = format!("test_dns_{}.db", test_id);
    let port = rand::random::<u16>() % 10000 + 30000;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let cfg = AppConfig {
        bind_host: "127.0.0.1".parse().unwrap(),
        dns_port: port,
        http_host: "127.0.0.1".parse().unwrap(),
        http_port: port + 1,
        cors_domains: vec!["mydns.local".to_string()],
        db_path: db_path.clone(),
        jwt_secret: mydns::config::generate_secret(64),
        admin_username: "admin".to_string(),
        admin_password: "changeme123".to_string(),
        resolver_priority: ResolverPriority::CloudflareFirst,
        cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
        router_dns: None,
    };

    let _ = std::fs::remove_file(&db_path);
    let pool = db::init(&db_path).await.expect("Failed to init test DB");

    db::records::createRecord(
        &pool,
        &db::records::CreateRecord {
            // The DNS handler normalizes query names by removing the trailing
            // root label separator before looking them up in SQLite.
            name: "dns-test.local".to_string(),
            record_type: "A".to_string(),
            value: "10.20.30.40".to_string(),
            ttl: 60,
            priority: None,
        },
    )
    .await
    .expect("Failed to seed DNS record");

    let upstream = dns::upstream::UpstreamResolver::fromConfig(
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
    )
    .expect("Failed to build resolver");

    let (log_tx, _) = tokio::sync::broadcast::channel(1024);
    let cancel = CancellationToken::new();
    let state = AppState::new(pool, cfg, upstream, log_tx, cancel.clone());
    let server_state = Arc::clone(&state);
    let server_cancel = cancel.clone();

    let task = tokio::spawn(async move {
        let _ = dns::server::run(server_state, server_cancel).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    (addr, db_path, cancel, task)
}

fn query_message(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new();
    message.set_id(0x1234);
    message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
    message.to_vec().expect("Failed to encode DNS query")
}

#[tokio::test]
async fn test_dns_udp_positive_nxdomain_and_nodata() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    socket
        .send_to(&query_message("dns-test.local.", RecordType::A), addr)
        .await
        .unwrap();
    let mut buf = [0u8; 4096];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .expect("UDP DNS response timed out")
        .unwrap();
    let response = Message::from_bytes(&buf[..len]).expect("Invalid DNS response");
    assert_eq!(
        response.response_code(),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!response.answers().is_empty(), "Expected A answer");

    socket
        .send_to(&query_message("dns-test.local.", RecordType::AAAA), addr)
        .await
        .unwrap();
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .expect("UDP NODATA response timed out")
        .unwrap();
    let response = Message::from_bytes(&buf[..len]).expect("Invalid DNS response");
    assert_eq!(
        response.response_code(),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(response.answers().is_empty(), "Expected NODATA response");

    socket
        .send_to(
            &query_message("missing.dns-test.local.", RecordType::A),
            addr,
        )
        .await
        .unwrap();
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .expect("UDP NXDOMAIN response timed out")
        .unwrap();
    let response = Message::from_bytes(&buf[..len]).expect("Invalid DNS response");
    assert_eq!(
        response.response_code(),
        hickory_proto::op::ResponseCode::NXDomain
    );
    assert!(response.answers().is_empty());

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_tcp_positive_answer() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let mut stream = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(addr))
        .await
        .expect("TCP DNS connection timed out")
        .unwrap();

    let query = query_message("dns-test.local.", RecordType::A);
    let len = u16::try_from(query.len()).expect("DNS query too large");
    stream.write_all(&len.to_be_bytes()).await.unwrap();
    stream.write_all(&query).await.unwrap();

    let mut length = [0u8; 2];
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut length))
        .await
        .expect("TCP DNS length response timed out")
        .unwrap();
    let response_len = u16::from_be_bytes(length) as usize;
    let mut response_bytes = vec![0u8; response_len];
    tokio::time::timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_bytes),
    )
    .await
    .expect("TCP DNS response timed out")
    .unwrap();

    let response = Message::from_bytes(&response_bytes).expect("Invalid TCP DNS response");
    assert_eq!(
        response.response_code(),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!response.answers().is_empty(), "Expected A answer over TCP");

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}
