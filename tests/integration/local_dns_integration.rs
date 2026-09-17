use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RecordType};
use hickory_proto::serialize::binary::BinDecodable;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

mod common;

fn query_message(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
    message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
    let mut bytes = message.to_vec().expect("Failed to encode DNS query");
    bytes[2] |= 0b0000_0001;
    bytes
}

async fn udp_query(socket: &UdpSocket, addr: SocketAddr, name: &str, record_type: RecordType) -> Message {
    socket.send_to(&query_message(name, record_type), addr).await.unwrap();
    let mut buf = vec![0u8; 512];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf)).await.unwrap().unwrap();
    Message::from_bytes(&buf[..len]).unwrap()
}

#[tokio::test]
async fn test_local_dns_resolution() {
    let server = common::TestDnsServer::start_with_records(
        vec!["home.arpa".to_string()],
        &[("router.home.arpa", "A", "192.168.1.1")],
    ).await;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let response = udp_query(&socket, server.addr, "router.home.arpa.", RecordType::A).await;
    assert_eq!(response.metadata.response_code, ResponseCode::NoError);
    assert_eq!(response.answers.len(), 1);

    let response = udp_query(&socket, server.addr, "unknown.home.arpa.", RecordType::A).await;
    assert_eq!(response.metadata.response_code, ResponseCode::NXDomain);
}
