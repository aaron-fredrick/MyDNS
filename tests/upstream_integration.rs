mod common;

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RecordType};
use mydns::dns::record_index::RecordIndex;
use mydns::dns::zone_trie::ZoneTrie;
use mydns::{config::AppConfig, db, dns, state::AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

async fn start_mock_upstream() -> (SocketAddr, tokio::sync::mpsc::Receiver<Message>) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let socket = Arc::new(socket);
    let socket_clone = socket.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok((len, src)) = socket_clone.recv_from(&mut buf).await {
            let msg = Message::from_vec(&buf[..len]).unwrap();

            // Create a response based on the query name.
            let mut response = Message::new(msg.id, MessageType::Response, msg.op_code);
            response.add_query(msg.queries.first().unwrap().clone());

            let name = msg
                .queries
                .first()
                .unwrap()
                .name()
                .to_string()
                .to_lowercase();
            if name.contains("nxdomain") {
                response.metadata.response_code = ResponseCode::NXDomain;
                let bytes = response.to_vec().unwrap();
                let _ = socket_clone.send_to(&bytes, src).await;
            } else if name.contains("servfail") {
                response.metadata.response_code = ResponseCode::ServFail;
                let bytes = response.to_vec().unwrap();
                let _ = socket_clone.send_to(&bytes, src).await;
            } else if name.contains("timeout") {
                // Do nothing, let it timeout.
            } else {
                response.metadata.response_code = ResponseCode::NoError;
                let bytes = response.to_vec().unwrap();
                let _ = socket_clone.send_to(&bytes, src).await;
            }
            let _ = tx.send(msg).await;
        }
    });

    (addr, rx)
}

struct TestUpstreamServerContext {
    pub db: common::TestDb,
    pub addr: SocketAddr,
    pub cancel: CancellationToken,
    pub task: tokio::task::JoinHandle<()>,
}

impl Drop for TestUpstreamServerContext {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn start_dns_server(upstream_addr: SocketAddr) -> TestUpstreamServerContext {
    let _ = tracing_subscriber::fmt::try_init();
    let db = common::TestDb::new();
    let port = common::get_ephemeral_port().await;
    let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), port);

    let cfg = AppConfig {
        bind_host: "127.0.0.1".parse().unwrap(),
        dns_port: port,
        http_host: "127.0.0.1".parse().unwrap(),
        http_port: port + 1,
        cors_domains: vec!["mydns.local".to_string()],
        dashboard_domain: "mydns.local".to_string(),
        db_path: db.path_str(),
        jwt_secret: mydns::config::generate_secret(64),
        admin_username: "admin".to_string(),
        admin_password: "changeme123".to_string(),
        resolver_mode: mydns::config::ResolverMode::Forwarding,
        resolver_priority: mydns::config::ResolverPriority::CloudflareFirst,
        cloudflare_dns: upstream_addr,
        router_dns: Some(upstream_addr),
        run_as_user: "nobody".to_string(),
        run_as_group: "nobody".to_string(),
        allowed_zones: vec![],
        root_hints: vec![],
    };

    let pool = db.init_pool().await;

    let hash = mydns::web::auth::hash_password(&cfg.admin_password).unwrap();
    db::records::seed_admin(&pool, &cfg.admin_username, &hash)
        .await
        .unwrap();

    let upstream = dns::upstream::UpstreamResolver::from_config(
        cfg.resolver_mode.clone(),
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
        cfg.root_hints.clone(),
    )
    .expect("Failed to build resolver");

    let (log_tx, _) = tokio::sync::broadcast::channel(1024);
    let cancel = CancellationToken::new();
    let zone_trie = ZoneTrie::from_zones(&cfg.allowed_zones);
    let record_index = RecordIndex::load_from_db(&pool)
        .await
        .expect("Failed to load record index");
    let state = AppState::new(
        pool,
        cfg,
        upstream,
        log_tx,
        cancel.clone(),
        record_index,
        zone_trie,
    );
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

    TestUpstreamServerContext {
        db,
        addr,
        cancel,
        task,
    }
}

fn query_message(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(0x5678, MessageType::Query, OpCode::Query);
    message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
    let mut bytes = message.to_vec().unwrap();
    bytes[2] |= 0b0000_0001; // Set RD bit
    bytes
}

async fn udp_query(socket: &UdpSocket, addr: SocketAddr, name: &str) -> Message {
    socket
        .send_to(&query_message(name, RecordType::A), addr)
        .await
        .unwrap();
    let mut buf = [0u8; 4096];
    let (len, _) = tokio::time::timeout(Duration::from_secs(15), socket.recv_from(&mut buf))
        .await
        .expect("Test client timed out waiting for DNS response")
        .unwrap();
    Message::from_vec(&buf[..len]).unwrap()
}

#[tokio::test]
async fn test_upstream_nxdomain() {
    let (mock_addr, _rx) = start_mock_upstream().await;
    let ctx = start_dns_server(mock_addr).await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let res = udp_query(&socket, ctx.addr, "nxdomain.example.com.").await;
    assert_eq!(res.response_code, ResponseCode::NXDomain);
}

#[tokio::test]
async fn test_upstream_servfail() {
    let (mock_addr, _rx) = start_mock_upstream().await;
    let ctx = start_dns_server(mock_addr).await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let res = udp_query(&socket, ctx.addr, "servfail.example.com.").await;
    assert_eq!(res.response_code, ResponseCode::ServFail);
}

#[tokio::test]
async fn test_upstream_timeout() {
    let (mock_addr, _rx) = start_mock_upstream().await;
    let ctx = start_dns_server(mock_addr).await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // The timeout one will cause the server to wait for upstream, time out, and return ServFail.
    let res = udp_query(&socket, ctx.addr, "timeout.example.com.").await;
    assert_eq!(res.response_code, ResponseCode::ServFail);
}
