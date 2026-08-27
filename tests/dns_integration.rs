//! Integration tests for the DNS wire server.
//!
//! These tests exercise the real DNS UDP/TCP listeners and the resolution
//! outcome model against an isolated SQLite database.

use hickory_proto::op::{Message, MessageType, OpCode, Query};
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
        dashboard_domain: "mydns.local".to_string(),
        db_path: db_path.clone(),
        jwt_secret: mydns::config::generate_secret(64),
        admin_username: "admin".to_string(),
        admin_password: "changeme123".to_string(),
        resolver_priority: ResolverPriority::CloudflareFirst,
        cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
        router_dns: None,
        run_as_user: "nobody".to_string(),
        run_as_group: "nobody".to_string(),
        allowed_zones: vec![],
    };

    let _ = std::fs::remove_file(&db_path);
    let pool = db::init(&db_path).await.expect("Failed to init test DB");

    for record in [
        ("dns-test.local", "A", "10.20.30.40"),
        (
            "alias-one.dns-test.local",
            "CNAME",
            "alias-two.dns-test.local",
        ),
        ("alias-two.dns-test.local", "CNAME", "dns-test.local"),
        (
            "loop-one.dns-test.local",
            "CNAME",
            "loop-two.dns-test.local",
        ),
        (
            "loop-two.dns-test.local",
            "CNAME",
            "loop-one.dns-test.local",
        ),
        ("dns6-test.local", "AAAA", "2001:db8::1"),
        ("mail-test.local", "MX", "mail.example.com"),
        ("ns-test.local", "NS", "ns1.example.com"),
        (
            "txt-test.local",
            "TXT",
            "v=spf1 include:_spf.example.com ~all",
        ),
    ] {
        db::records::createRecord(
            &pool,
            &db::records::CreateRecord {
                name: record.0.to_string(),
                record_type: record.1.to_string(),
                value: record.2.to_string(),
                ttl: 60,
                priority: if record.1 == "MX" { Some(10) } else { None },
            },
        )
        .await
        .expect("Failed to seed DNS record");
    }

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

    for _ in 0..50 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    (addr, db_path, cancel, task)
}

fn query_message(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
    message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
    message.to_vec().expect("Failed to encode DNS query")
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
        .unwrap();
    let mut buf = [0u8; 4096];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .expect("UDP DNS response timed out")
        .unwrap();
    Message::from_bytes(&buf[..len]).expect("Invalid DNS response")
}

fn response_code(message: &Message) -> hickory_proto::op::ResponseCode {
    message.metadata.response_code
}

#[tokio::test]
async fn test_dns_udp_positive_nxdomain_and_nodata() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "dns-test.local.", RecordType::A).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert!(!response.answers.is_empty(), "Expected A answer");

    let response = udp_query(&socket, addr, "dns-test.local.", RecordType::AAAA).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert!(response.answers.is_empty(), "Expected NODATA response");

    let response = udp_query(&socket, addr, "missing.dns-test.local.", RecordType::A).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NXDomain);
    assert!(response.answers.is_empty());

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_tcp_positive_answer() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let mut stream = None;
    for _ in 0..10 {
        if let Ok(Ok(s)) =
            tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(addr)).await
        {
            stream = Some(s);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut stream = stream.expect("TCP DNS connection failed after retries");

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
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut response_bytes))
        .await
        .expect("TCP DNS response timed out")
        .unwrap();

    let response = Message::from_bytes(&response_bytes).expect("Invalid TCP DNS response");
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert!(!response.answers.is_empty(), "Expected A answer over TCP");

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_authoritative_cname_chain_and_cname_only() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "alias-one.dns-test.local.", RecordType::A).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 3, "Expected two CNAMEs and the target A record");
    assert_eq!(response.answers[0].record_type(), RecordType::CNAME);
    assert_eq!(response.answers[1].record_type(), RecordType::CNAME);
    assert_eq!(response.answers[2].record_type(), RecordType::A);

    let response = udp_query(&socket, addr, "alias-one.dns-test.local.", RecordType::CNAME).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1, "Expected the requested CNAME only");
    assert_eq!(response.answers[0].record_type(), RecordType::CNAME);

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_authoritative_cname_loop_returns_servfail() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "loop-one.dns-test.local.", RecordType::A).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::ServFail);
    assert!(response.answers.is_empty());

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_aaaa_record() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "dns6-test.local.", RecordType::AAAA).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::AAAA);

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_mx_record() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "mail-test.local.", RecordType::MX).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::MX);

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_ns_record() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "ns-test.local.", RecordType::NS).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::NS);

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_dns_udp_txt_record() {
    let (addr, db_path, cancel, task) = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, addr, "txt-test.local.", RecordType::TXT).await;
    assert_eq!(response_code(&response), hickory_proto::op::ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::TXT);

    cancel.cancel();
    let _ = task.await;
    let _ = std::fs::remove_file(db_path);
}
