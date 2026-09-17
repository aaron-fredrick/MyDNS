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
    bytes[2] |= 0b0000_0001;
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

    let mut server = server;
    server.restart().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "ads.example.com.", RecordType::A).await;
    assert_eq!(response_code(&response), ResponseCode::NXDomain);

    let response = udp_query(&socket, server.addr, "sub.ads.example.com.", RecordType::A).await;
    assert_eq!(response_code(&response), ResponseCode::NXDomain);
}

#[tokio::test]
async fn test_cache_blocklist_race_behavior() {
    let server = common::TestDnsServer::start_with_zones_only(vec![]).await;
    let mut server = server;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "example.com.", RecordType::A).await;
    assert_eq!(response_code(&response), ResponseCode::NoError);

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

    server.restart().await;
    let response2 = udp_query(&socket, server.addr, "example.com.", RecordType::A).await;
    assert_eq!(response_code(&response2), ResponseCode::NXDomain);
}
