use super::fixtures::test_name;
use crate::mydns::dns::handler::build_record;
use hickory_proto::rr::RecordType;

#[test]
fn builds_common_record_types() {
    let cases = [
        (RecordType::A, "127.0.0.1"),
        (RecordType::AAAA, "::1"),
        (RecordType::CNAME, "target.example.com"),
        (RecordType::MX, "mail.example.com"),
        (RecordType::NS, "ns1.example.com"),
        (RecordType::PTR, "localhost"),
        (RecordType::TXT, "hello"),
    ];

    for (record_type, value) in cases {
        let record = build_record(&test_name().to_string(), record_type, value, 300, Some(10))
            .expect("record should build");
        assert_eq!(record.record_type(), record_type);
        assert_eq!(record.ttl, 300);
    }
}

#[test]
fn builds_soa_record() {
    let record = build_record(
        "example.com",
        RecordType::SOA,
        "ns1.example.com. hostmaster.example.com. 1 3600 600 86400 300",
        300,
        None,
    )
    .expect("SOA should build");

    assert_eq!(record.record_type(), RecordType::SOA);
}

#[test]
fn rejects_invalid_values() {
    assert!(build_record("fail.test", RecordType::A, "not-an-ip", 300, None).is_none());
    assert!(build_record("fail.test", RecordType::AAAA, "not-an-ip", 300, None).is_none());
    assert!(build_record("fail.test", RecordType::A, "127.0.0.1", 300, None).is_some());
}

#[test]
fn rejects_malformed_soa() {
    assert!(build_record(
        "example.com",
        RecordType::SOA,
        "ns1.example.com. hostmaster.example.com.",
        300,
        None
    )
    .is_none());
}

#[test]
fn normalizes_record_names_to_fqdns() {
    let record = build_record("example.com", RecordType::A, "127.0.0.1", 300, None)
        .expect("record should build");

    assert_eq!(record.name.to_string(), "example.com.");
}

#[test]
fn respects_mx_priority_and_default() {
    let explicit = build_record("mail.test", RecordType::MX, "mx.example.com", 600, Some(5))
        .expect("MX should build");
    let defaulted = build_record("mail.test", RecordType::MX, "mx.example.com", 600, None)
        .expect("MX should build");

    assert_eq!(explicit.record_type(), RecordType::MX);
    assert_eq!(defaulted.record_type(), RecordType::MX);
}
