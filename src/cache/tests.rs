use super::{CacheResult, DnsCache};
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
    let (result, records) = cache
        .get("example.com.", RecordType::A)
        .expect("cache entry should exist");
    assert_eq!(result, CacheResult::Positive);
    assert!(records.is_empty());
}

#[test]
fn negative_entry_is_distinguishable_from_empty_positive_response() {
    let mut cache = DnsCache::new();
    cache.insert_negative("missing.example.", RecordType::A, Duration::from_secs(60));

    let (result, records) = cache
        .get("missing.example.", RecordType::A)
        .expect("negative cache entry should exist");
    assert_eq!(result, CacheResult::Negative);
    assert!(records.is_empty());
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
    let all = cache.list_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, "list.test.");
}

#[test]
fn clear_zone_evicts_apex_and_subdomains() {
    let mut cache = DnsCache::new();
    // Zone apex
    cache.insert(
        "example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    // Subdomain
    cache.insert(
        "host.example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );
    // Unrelated — must survive
    cache.insert("other.net", RecordType::A, vec![], Duration::from_secs(300));

    cache.clear_zone("example.com");

    assert!(
        cache.get("example.com", RecordType::A).is_none(),
        "apex should be evicted"
    );
    assert!(
        cache.get("host.example.com", RecordType::A).is_none(),
        "subdomain should be evicted"
    );
    assert!(
        cache.get("other.net", RecordType::A).is_some(),
        "unrelated entry must be preserved"
    );
}

#[test]
fn clear_zone_does_not_evict_parent_zone() {
    let mut cache = DnsCache::new();
    cache.insert("com", RecordType::NS, vec![], Duration::from_secs(300));
    cache.insert(
        "example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
    );

    cache.clear_zone("example.com");

    assert!(
        cache.get("com", RecordType::NS).is_some(),
        "parent zone entry must be preserved"
    );
}
