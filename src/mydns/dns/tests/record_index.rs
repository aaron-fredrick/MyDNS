use crate::db::records::DnsRecord;
use crate::mydns::dns::record_index::{IndexResolution, RecordIndex};

fn record(id: i64, name: &str, rtype: &str, value: &str) -> DnsRecord {
    DnsRecord {
        id,
        name: name.to_string(),
        record_type: rtype.to_string(),
        value: value.to_string(),
        ttl: 300,
        priority: None,
        created_at: String::new(),
        updated_at: String::new(),
        is_dev: false,
    }
}

#[test]
fn finds_exact_record() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));

    match index.resolve_authoritative("example.com", "A", None) {
        IndexResolution::Found(records) => assert_eq!(records[0].value, "1.2.3.4"),
        other => panic!("expected Found, got {other:?}"),
    }
}

#[test]
fn lookup_normalizes_case_and_trailing_dot() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "HOST.Example.COM.", "a", "192.0.2.1"));

    assert!(matches!(
        index.resolve_authoritative("host.example.com.", "A", None),
        IndexResolution::Found(_)
    ));
}

#[test]
fn missing_name_is_a_miss() {
    let index = RecordIndex::default();
    assert!(matches!(
        index.resolve_authoritative("missing.example.com", "A", None),
        IndexResolution::Miss
    ));
}

#[test]
fn existing_name_with_missing_type_is_nodata() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));

    assert!(matches!(
        index.resolve_authoritative("example.com", "AAAA", None),
        IndexResolution::Nodata
    ));
}

#[test]
fn cname_chain_is_prepended_to_target_record() {
    let mut index = RecordIndex::default();
    index.upsert(record(
        1,
        "alias.example.com",
        "CNAME",
        "target.example.com",
    ));
    index.upsert(record(2, "target.example.com", "A", "1.2.3.4"));

    match index.resolve_authoritative("alias.example.com", "A", None) {
        IndexResolution::Found(records) => {
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].record_type, "CNAME");
            assert_eq!(records[1].record_type, "A");
        }
        other => panic!("expected Found, got {other:?}"),
    }
}

#[test]
fn cname_to_missing_target_returns_cname_chain() {
    let mut index = RecordIndex::default();
    index.upsert(record(
        1,
        "alias.example.com",
        "CNAME",
        "external.example.net.",
    ));

    match index.resolve_authoritative("alias.example.com", "A", None) {
        IndexResolution::Found(records) => {
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].record_type, "CNAME");
        }
        other => panic!("expected CNAME chain, got {other:?}"),
    }
}

#[test]
fn cname_loop_returns_servfail() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "a.example.com", "CNAME", "b.example.com"));
    index.upsert(record(2, "b.example.com", "CNAME", "a.example.com"));

    assert!(matches!(
        index.resolve_authoritative("a.example.com", "A", None),
        IndexResolution::ServFail
    ));
}

#[test]
fn excessive_cname_depth_returns_servfail() {
    let mut index = RecordIndex::default();
    for i in 0..12 {
        index.upsert(record(
            i,
            &format!("node{i}.example.com"),
            "CNAME",
            &format!("node{}.example.com", i + 1),
        ));
    }

    assert!(matches!(
        index.resolve_authoritative("node0.example.com", "A", None),
        IndexResolution::ServFail
    ));
}

#[test]
fn upsert_replaces_existing_record_by_id() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));
    index.upsert(record(1, "example.com", "A", "5.6.7.8"));

    match index.resolve_authoritative("example.com", "A", None) {
        IndexResolution::Found(records) => {
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].value, "5.6.7.8");
        }
        other => panic!("expected Found, got {other:?}"),
    }
}

#[test]
fn remove_by_id_removes_record_and_owner() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));
    index.remove_by_id(1);

    assert!(matches!(
        index.resolve_authoritative("example.com", "A", None),
        IndexResolution::Miss
    ));
}

#[test]
fn remove_by_name_and_type_preserves_other_types() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));
    index.upsert(record(2, "example.com", "MX", "mail.example.com"));
    index.remove("example.com", Some("A"));

    assert!(matches!(
        index.resolve_authoritative("example.com", "A", None),
        IndexResolution::Nodata
    ));
    assert!(matches!(
        index.resolve_authoritative("example.com", "MX", None),
        IndexResolution::Found(_)
    ));
}

#[test]
fn remove_by_name_removes_all_types() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));
    index.upsert(record(2, "example.com", "MX", "mail.example.com"));
    index.remove("example.com", None);

    assert!(matches!(
        index.resolve_authoritative("example.com", "A", None),
        IndexResolution::Miss
    ));
}

#[test]
fn any_returns_all_records_at_owner() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "example.com", "A", "1.2.3.4"));
    index.upsert(record(2, "example.com", "AAAA", "2001:db8::1"));
    index.upsert(record(3, "example.com", "MX", "mail.example.com"));

    match index.resolve_authoritative("example.com", "ANY", None) {
        IndexResolution::Found(records) => assert_eq!(records.len(), 3),
        other => panic!("expected Found, got {other:?}"),
    }
}

#[test]
fn any_follows_cname_chain() {
    let mut index = RecordIndex::default();
    index.upsert(record(
        1,
        "alias.example.com",
        "CNAME",
        "target.example.com",
    ));
    index.upsert(record(2, "target.example.com", "A", "1.2.3.4"));
    index.upsert(record(3, "target.example.com", "AAAA", "2001:db8::1"));

    match index.resolve_authoritative("alias.example.com", "ANY", None) {
        IndexResolution::Found(records) => assert_eq!(records.len(), 3),
        other => panic!("expected Found, got {other:?}"),
    }
}

#[test]
fn empty_zone_apex_is_nodata() {
    let index = RecordIndex::default();

    assert!(matches!(
        index.resolve_authoritative("mydns.local", "SOA", Some("mydns.local")),
        IndexResolution::Nodata
    ));
}

#[test]
fn empty_non_terminal_is_nodata() {
    let mut index = RecordIndex::default();
    index.upsert(record(1, "deep.sub.mydns.local", "TXT", "hello"));

    assert!(matches!(
        index.resolve_authoritative("sub.mydns.local", "A", Some("mydns.local")),
        IndexResolution::Nodata
    ));
}
