//! Integration tests for the DNS wire server.
//!
//! These tests exercise the real DNS UDP/TCP listeners and the resolution
//! outcome model against an isolated SQLite database.

mod common;

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RecordType};
use hickory_proto::serialize::binary::BinDecodable;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

async fn start_dns_server() -> common::TestDnsServer {
    common::TestDnsServer::start_with_records(
        vec!["dns-test.local".to_string()],
        &[
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
        ],
    )
    .await
}

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

fn response_code(message: &Message) -> hickory_proto::op::ResponseCode {
    message.metadata.response_code
}

#[tokio::test]
async fn test_dns_udp_positive_nxdomain_and_nodata() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::A).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!response.answers.is_empty(), "Expected A answer");

    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::AAAA).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(response.answers.is_empty(), "Expected NODATA response");

    let response = udp_query(
        &socket,
        server.addr,
        "missing.dns-test.local.",
        RecordType::A,
    )
    .await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NXDomain
    );
    assert!(response.answers.is_empty());
}

#[tokio::test]
async fn test_dns_tcp_positive_answer() {
    let server = start_dns_server().await;
    let mut stream = None;
    for _ in 0..10 {
        if let Ok(Ok(s)) =
            tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(server.addr)).await
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
    tokio::time::timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_bytes),
    )
    .await
    .expect("TCP DNS response timed out")
    .unwrap();

    let response = Message::from_bytes(&response_bytes).expect("Invalid TCP DNS response");
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!response.answers.is_empty(), "Expected A answer over TCP");
}

#[tokio::test]
async fn test_dns_udp_authoritative_cname_chain_and_cname_only() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(
        &socket,
        server.addr,
        "alias-one.dns-test.local.",
        RecordType::A,
    )
    .await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(
        response.answers.len(),
        3,
        "Expected two CNAMEs and the target A record"
    );
    assert_eq!(response.answers[0].record_type(), RecordType::CNAME);
    assert_eq!(response.answers[1].record_type(), RecordType::CNAME);
    assert_eq!(response.answers[2].record_type(), RecordType::A);

    let response = udp_query(
        &socket,
        server.addr,
        "alias-one.dns-test.local.",
        RecordType::CNAME,
    )
    .await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(
        response.answers.len(),
        1,
        "Expected the requested CNAME only"
    );
    assert_eq!(response.answers[0].record_type(), RecordType::CNAME);
}

#[tokio::test]
async fn test_dns_udp_authoritative_cname_loop_returns_servfail() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(
        &socket,
        server.addr,
        "loop-one.dns-test.local.",
        RecordType::A,
    )
    .await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::ServFail
    );
    assert!(response.answers.is_empty());
}

#[tokio::test]
async fn test_dns_udp_aaaa_record() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "dns6-test.local.", RecordType::AAAA).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::AAAA);
}

#[tokio::test]
async fn test_dns_udp_mx_record() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "mail-test.local.", RecordType::MX).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::MX);
}

#[tokio::test]
async fn test_dns_udp_ns_record() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "ns-test.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::NS);
}

#[tokio::test]
async fn test_dns_udp_txt_record() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "txt-test.local.", RecordType::TXT).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert_eq!(response.answers.len(), 1);
    assert_eq!(response.answers[0].record_type(), RecordType::TXT);
}

#[tokio::test]
async fn test_authoritative_zone_missing_soa_returns_nodata() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // dns-test.local has an A record but no SOA record
    // Should return NOERROR (NODATA) with AA flag set, not NXDOMAIN
    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError,
        "Expected NOERROR (NODATA) for missing SOA on existing name"
    );
    assert!(
        response.answers.is_empty(),
        "Expected zero answers for NODATA"
    );
    assert!(
        response.metadata.authoritative,
        "Expected AA flag set for authoritative zone"
    );
}

#[tokio::test]
async fn test_authoritative_zone_missing_ns_returns_nodata() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // dns-test.local has an A record but no NS record
    // Should return NOERROR (NODATA) with AA flag set, not NXDOMAIN
    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError,
        "Expected NOERROR (NODATA) for missing NS on existing name"
    );
    assert!(
        response.answers.is_empty(),
        "Expected zero answers for NODATA"
    );
    assert!(
        response.metadata.authoritative,
        "Expected AA flag set for authoritative zone"
    );
}

#[tokio::test]
async fn test_authoritative_zone_missing_rr_type_returns_nodata() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // dns-test.local has an A record but no MX record
    // Should return NOERROR (NODATA) with AA flag set, not NXDOMAIN
    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::MX).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError,
        "Expected NOERROR (NODATA) for missing RR type on existing name"
    );
    assert!(
        response.answers.is_empty(),
        "Expected zero answers for NODATA"
    );
    assert!(
        response.metadata.authoritative,
        "Expected AA flag set for authoritative zone"
    );
}

#[tokio::test]
async fn test_authoritative_zone_nonexistent_name_returns_nxdomain() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // nonexistent.dns-test.local does not exist in the zone
    // Should return NXDOMAIN with AA flag set
    let response = udp_query(
        &socket,
        server.addr,
        "nonexistent.dns-test.local.",
        RecordType::A,
    )
    .await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NXDomain,
        "Expected NXDOMAIN for nonexistent name in authoritative zone"
    );
    assert!(
        response.answers.is_empty(),
        "Expected zero answers for NXDOMAIN"
    );
    assert!(
        response.metadata.authoritative,
        "Expected AA flag set for authoritative zone"
    );
}

#[tokio::test]
async fn test_authoritative_zone_a_query_with_aa_flag() {
    let server = start_dns_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // dns-test.local has an A record
    // Should return NOERROR with AA flag set and the A record
    let response = udp_query(&socket, server.addr, "dns-test.local.", RecordType::A).await;
    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!response.answers.is_empty(), "Expected A answer");
    assert_eq!(response.answers[0].record_type(), RecordType::A);
    assert!(
        response.metadata.authoritative,
        "Expected AA flag set for authoritative zone"
    );
}
