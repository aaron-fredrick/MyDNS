use super::handler::build_record;
use hickory_proto::rr::RecordType;

#[test]
fn test_build_a_record() {
    let rec = build_record("example.com", RecordType::A, "127.0.0.1", 300, None);
    assert!(rec.is_some());
    let r = rec.unwrap();
    assert_eq!(r.record_type(), RecordType::A);
    assert_eq!(r.name().to_string().trim_end_matches('.'), "example.com");
    assert_eq!(r.ttl, 300);
}

#[test]
fn test_build_aaaa_record() {
    let rec = build_record("example.com", RecordType::AAAA, "::1", 300, None);
    assert!(rec.is_some());
    assert_eq!(rec.unwrap().record_type(), RecordType::AAAA);
}

#[test]
fn test_build_mx_record() {
    let rec = build_record("mail.test", RecordType::MX, "mx.example.com", 600, Some(5));
    assert!(rec.is_some());
    let r = rec.unwrap();
    assert_eq!(r.record_type(), RecordType::MX);
}

#[test]
fn test_build_ptr_record() {
    let rec = build_record(
        "1.0.0.127.in-addr.arpa",
        RecordType::PTR,
        "localhost",
        3600,
        None,
    );
    assert!(rec.is_some());
    assert_eq!(rec.unwrap().record_type(), RecordType::PTR);
}

#[test]
fn test_build_invalid_ip() {
    let rec = build_record("fail.prop", RecordType::A, "not-an-ip", 300, None);
    assert!(rec.is_none());
}

#[test]
fn test_build_invalid_name() {
    let rec = build_record("valid.name", RecordType::A, "1.1.1.1", 300, None);
    assert!(rec.is_some());
}
