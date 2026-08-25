use super::DnsCache;
use hickory_proto::rr::RecordType;
use std::time::Duration;

#[test]
fn insert_and_hit() {
    let mut cache = DnsCache::new();
    cache.insert(
        "example.com.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    assert!(cache.get("example.com.", RecordType::A).is_some());
}

#[test]
fn miss_on_wrong_type() {
    let mut cache = DnsCache::new();
    cache.insert(
        "example.com.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    assert!(cache.get("example.com.", RecordType::AAAA).is_none());
}

#[test]
fn expired_entry_returns_none() {
    let mut cache = DnsCache::new();
    cache.insert(
        "example.com.",
        RecordType::A,
        vec![],
        Duration::from_millis(0),
    );
    assert!(cache.get("example.com.", RecordType::A).is_none());
}

#[test]
fn prune_removes_expired_entries() {
    let mut cache = DnsCache::new();
    cache.insert(
        "expired.test.",
        RecordType::A,
        vec![],
        Duration::from_millis(0),
    );
    cache.insert(
        "valid.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    let pruned = cache.prune();
    assert_eq!(pruned, 1);
    assert_eq!(cache.len(), 1);
}

#[test]
fn remove_specific_entry() {
    let mut cache = DnsCache::new();
    cache.insert(
        "target.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    cache.remove("target.test.", RecordType::A);
    assert!(cache.get("target.test.", RecordType::A).is_none());
}

#[test]
fn list_all_returns_correct_data() {
    let mut cache = DnsCache::new();
    cache.insert(
        "list.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    let all = cache.listAll();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, "list.test.");
}
