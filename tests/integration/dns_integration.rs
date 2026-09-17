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

// ── Apex SOA / NS tests ───────────────────────────────────────────────────────

/// Helper: start a server with a single zone and no extra user records.
/// The DB layer automatically inserts apex SOA and NS on zone creation.
async fn start_apex_server() -> common::TestDnsServer {
    common::TestDnsServer::start_with_zones_only(vec!["apex-test.local".to_string()]).await
}

// --- UDP: apex SOA ---

#[tokio::test]
async fn test_apex_soa_udp_noerror_aa_with_answer() {
    let server = start_apex_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "apex-test.local.", RecordType::SOA).await;

    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError,
        "Apex SOA must return NOERROR"
    );
    assert!(
        response.metadata.authoritative,
        "Apex SOA must have AA flag set"
    );
    assert!(
        !response.answers.is_empty(),
        "Apex SOA must have at least one answer record"
    );
    assert_eq!(
        response.answers[0].record_type(),
        RecordType::SOA,
        "Answer must be a SOA record"
    );
}

// --- UDP: apex NS ---

#[tokio::test]
async fn test_apex_ns_udp_noerror_aa_with_answer() {
    let server = start_apex_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "apex-test.local.", RecordType::NS).await;

    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError,
        "Apex NS must return NOERROR"
    );
    assert!(
        response.metadata.authoritative,
        "Apex NS must have AA flag set"
    );
    assert!(
        !response.answers.is_empty(),
        "Apex NS must have at least one answer record"
    );
    assert_eq!(
        response.answers[0].record_type(),
        RecordType::NS,
        "Answer must be an NS record"
    );
}

// --- TCP: apex SOA ---

#[tokio::test]
async fn test_apex_soa_tcp_noerror_aa_with_answer() {
    let server = start_apex_server().await;
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

    let query = query_message("apex-test.local.", RecordType::SOA);
    let len = u16::try_from(query.len()).unwrap();
    stream.write_all(&len.to_be_bytes()).await.unwrap();
    stream.write_all(&query).await.unwrap();

    let mut length = [0u8; 2];
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut length))
        .await
        .expect("TCP SOA length timed out")
        .unwrap();
    let response_len = u16::from_be_bytes(length) as usize;
    let mut response_bytes = vec![0u8; response_len];
    tokio::time::timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_bytes),
    )
    .await
    .expect("TCP SOA response timed out")
    .unwrap();

    use hickory_proto::serialize::binary::BinDecodable;
    let response: hickory_proto::op::Message =
        BinDecodable::from_bytes(&response_bytes).expect("Invalid TCP SOA response");

    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(response.metadata.authoritative, "TCP SOA must have AA flag");
    assert!(!response.answers.is_empty(), "TCP SOA must have answers");
    assert_eq!(response.answers[0].record_type(), RecordType::SOA);
}

// --- TCP: apex NS ---

#[tokio::test]
async fn test_apex_ns_tcp_noerror_aa_with_answer() {
    let server = start_apex_server().await;
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

    let query = query_message("apex-test.local.", RecordType::NS);
    let len = u16::try_from(query.len()).unwrap();
    stream.write_all(&len.to_be_bytes()).await.unwrap();
    stream.write_all(&query).await.unwrap();

    let mut length = [0u8; 2];
    tokio::time::timeout(Duration::from_secs(2), stream.read_exact(&mut length))
        .await
        .expect("TCP NS length timed out")
        .unwrap();
    let response_len = u16::from_be_bytes(length) as usize;
    let mut response_bytes = vec![0u8; response_len];
    tokio::time::timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_bytes),
    )
    .await
    .expect("TCP NS response timed out")
    .unwrap();

    use hickory_proto::serialize::binary::BinDecodable;
    let response: hickory_proto::op::Message =
        BinDecodable::from_bytes(&response_bytes).expect("Invalid TCP NS response");

    assert_eq!(
        response_code(&response),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(response.metadata.authoritative, "TCP NS must have AA flag");
    assert!(!response.answers.is_empty(), "TCP NS must have answers");
    assert_eq!(response.answers[0].record_type(), RecordType::NS);
}

// --- Restart persistence ---

#[tokio::test]
async fn test_apex_soa_ns_survive_restart() {
    let mut server = start_apex_server().await;

    // Verify SOA/NS exist before restart.
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let pre_soa = udp_query(&socket, server.addr, "apex-test.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&pre_soa),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!pre_soa.answers.is_empty(), "SOA must exist before restart");

    let pre_ns = udp_query(&socket, server.addr, "apex-test.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&pre_ns),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!pre_ns.answers.is_empty(), "NS must exist before restart");

    // Simulate restart against the same DB file.
    server.restart().await;
    let socket2 = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let post_soa = udp_query(&socket2, server.addr, "apex-test.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&post_soa),
        hickory_proto::op::ResponseCode::NoError,
        "SOA must still be present after restart"
    );
    assert!(
        !post_soa.answers.is_empty(),
        "SOA answer must survive restart"
    );
    assert!(post_soa.metadata.authoritative);

    let post_ns = udp_query(&socket2, server.addr, "apex-test.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&post_ns),
        hickory_proto::op::ResponseCode::NoError,
        "NS must still be present after restart"
    );
    assert!(
        !post_ns.answers.is_empty(),
        "NS answer must survive restart"
    );
    assert!(post_ns.metadata.authoritative);
}

// --- Zone deletion cleanup ---

#[tokio::test]
async fn test_apex_soa_ns_removed_on_zone_deletion() {
    use mydns::db::records;

    let server = start_apex_server().await;

    // Confirm SOA/NS exist before deletion.
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let pre_soa = udp_query(&socket, server.addr, "apex-test.local.", RecordType::SOA).await;
    assert!(
        !pre_soa.answers.is_empty(),
        "SOA must exist before zone deletion"
    );

    // Delete the zone via DB (same as zones_api::remove_zone).
    records::remove_zone(&server.pool, "apex-test.local")
        .await
        .expect("Zone removal failed");

    // Verify no SOA/NS records remain in the DB.
    let rows = records::find_by_name(&server.pool, "apex-test.local")
        .await
        .expect("find_by_name failed");
    assert!(
        rows.iter().all(|r| r.record_type != "SOA"),
        "SOA record must be purged from dns_records after zone deletion"
    );
    assert!(
        rows.iter().all(|r| r.record_type != "NS"),
        "NS record must be purged from dns_records after zone deletion"
    );

    // Verify zone is gone from zones table.
    let zone_names = records::list_zone_names(&server.pool)
        .await
        .expect("Failed to reload zone names");
    assert!(
        !zone_names.contains(&"apex-test.local".to_string()),
        "Zone must not appear in zone names after deletion"
    );
}

// --- Multiple zones: isolation ---

#[tokio::test]
async fn test_multiple_zones_soa_ns_isolated() {
    let server = common::TestDnsServer::start_with_zones_only(vec![
        "zone-a.local".to_string(),
        "zone-b.local".to_string(),
    ])
    .await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // zone-a SOA and NS
    let resp = udp_query(&socket, server.addr, "zone-a.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!resp.answers.is_empty(), "zone-a SOA must have answers");
    assert_eq!(resp.answers[0].record_type(), RecordType::SOA);

    let resp = udp_query(&socket, server.addr, "zone-a.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!resp.answers.is_empty(), "zone-a NS must have answers");

    // zone-b SOA and NS
    let resp = udp_query(&socket, server.addr, "zone-b.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!resp.answers.is_empty(), "zone-b SOA must have answers");
    assert_eq!(resp.answers[0].record_type(), RecordType::SOA);

    let resp = udp_query(&socket, server.addr, "zone-b.local.", RecordType::NS).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!resp.answers.is_empty(), "zone-b NS must have answers");

    // Verify SOA mname for zone-a refers to zone-a, not zone-b.
    let resp = udp_query(&socket, server.addr, "zone-a.local.", RecordType::SOA).await;
    if let hickory_proto::rr::RData::SOA(ref soa) = resp.answers[0].data {
        let mname = soa.mname.to_string();
        assert!(
            mname.contains("zone-a"),
            "zone-a SOA mname must reference zone-a, got: {mname}"
        );
    } else {
        panic!("Expected SOA rdata for zone-a");
    }

    // Verify SOA mname for zone-b refers to zone-b.
    let resp = udp_query(&socket, server.addr, "zone-b.local.", RecordType::SOA).await;
    if let hickory_proto::rr::RData::SOA(ref soa) = resp.answers[0].data {
        let mname = soa.mname.to_string();
        assert!(
            mname.contains("zone-b"),
            "zone-b SOA mname must reference zone-b, got: {mname}"
        );
    } else {
        panic!("Expected SOA rdata for zone-b");
    }

    // Non-apex child with no record → NXDOMAIN (zones must not bleed answers).
    let resp = udp_query(&socket, server.addr, "child.zone-a.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NXDomain,
        "Non-apex child with no record must be NXDOMAIN"
    );
}

// --- Regression: NODATA / NXDOMAIN semantics preserved ---

#[tokio::test]
async fn test_regression_nodata_nxdomain_with_apex_records_present() {
    let server = common::TestDnsServer::start_with_records(
        vec!["reg-test.local".to_string()],
        &[("host.reg-test.local", "A", "10.0.0.1")],
    )
    .await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // Existing child + matching RR → NOERROR with answer.
    let resp = udp_query(&socket, server.addr, "host.reg-test.local.", RecordType::A).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(
        !resp.answers.is_empty(),
        "Existing A must produce an answer"
    );

    // Existing child + missing RR → NODATA.
    let resp = udp_query(
        &socket,
        server.addr,
        "host.reg-test.local.",
        RecordType::AAAA,
    )
    .await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(resp.answers.is_empty(), "Missing AAAA must be NODATA");
    assert!(resp.metadata.authoritative);

    // Nonexistent name inside zone → NXDOMAIN + AA.
    let resp = udp_query(&socket, server.addr, "ghost.reg-test.local.", RecordType::A).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NXDomain
    );
    assert!(resp.answers.is_empty());
    assert!(resp.metadata.authoritative);

    // Apex SOA exists → must return NOERROR with answer.
    let resp = udp_query(&socket, server.addr, "reg-test.local.", RecordType::SOA).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(!resp.answers.is_empty(), "Apex SOA must have an answer");
    assert!(resp.metadata.authoritative);

    // Apex has SOA/NS but no A → must be NODATA for A.
    let resp = udp_query(&socket, server.addr, "reg-test.local.", RecordType::A).await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NoError
    );
    assert!(
        resp.answers.is_empty(),
        "Apex with no A record must be NODATA for A query"
    );
    assert!(resp.metadata.authoritative);
}

#[tokio::test]
async fn test_regression_authoritative_miss_not_forwarded_upstream() {
    let server = start_apex_server().await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // Nonexistent name inside authoritative zone → NXDOMAIN, not forwarded.
    let resp = udp_query(
        &socket,
        server.addr,
        "does-not-exist.apex-test.local.",
        RecordType::A,
    )
    .await;
    assert_eq!(
        response_code(&resp),
        hickory_proto::op::ResponseCode::NXDomain,
        "Authoritative miss must be NXDOMAIN, never forwarded upstream"
    );
    assert!(resp.metadata.authoritative);
}
