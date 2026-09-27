use super::{CacheResult, CacheStats, DnsCache};
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
        false,
    );
    let (result, _, records) = cache
        .get("example.com.", RecordType::A)
        .expect("cache entry should exist");
    assert_eq!(result, CacheResult::Positive);
    assert!(records.is_empty());
}

#[test]
fn lookup_normalizes_name_case() {
    let mut cache = DnsCache::new();
    cache.insert(
        "Example.COM.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        true,
    );

    let (_, authoritative, _) = cache
        .get("example.com.", RecordType::A)
        .expect("normalized lookup should hit");
    assert!(authoritative);
}

#[test]
fn authoritative_flag_is_preserved() {
    let mut cache = DnsCache::new();
    cache.insert(
        "authoritative.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        true,
    );

    let (_, authoritative, _) = cache
        .get("authoritative.test.", RecordType::A)
        .expect("cache entry should exist");
    assert!(authoritative);
}

#[test]
fn replacing_existing_key_does_not_grow_cache() {
    let mut cache = DnsCache::new();
    cache.insert(
        "example.com.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "example.com.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        true,
    );

    assert_eq!(cache.len(), 1);
    let (_, authoritative, _) = cache
        .get("example.com.", RecordType::A)
        .expect("replacement should remain present");
    assert!(authoritative);
}

#[test]
fn negative_entry_is_distinguishable_from_empty_positive_response() {
    let mut cache = DnsCache::new();
    cache.insert_negative("missing.example.", RecordType::A, Duration::from_secs(60));

    let (result, _, records) = cache
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
        false,
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
        false,
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
        false,
    );
    cache.insert(
        "valid.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
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
        false,
    );
    cache.remove("target.test.", RecordType::A);
    assert!(cache.get("target.test.", RecordType::A).is_none());
}

#[test]
fn remove_name_removes_all_record_types_but_preserves_other_names() {
    let mut cache = DnsCache::new();
    for rtype in [RecordType::A, RecordType::AAAA, RecordType::MX] {
        cache.insert(
            "target.test.",
            rtype,
            vec![],
            Duration::from_secs(300),
            false,
        );
    }
    cache.insert(
        "other.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );

    cache.remove_name("TARGET.TEST.");

    assert_eq!(cache.len(), 1);
    assert!(cache.get("target.test.", RecordType::A).is_none());
    assert!(cache.get("target.test.", RecordType::AAAA).is_none());
    assert!(cache.get("other.test.", RecordType::A).is_some());
}

#[test]
fn list_all_returns_correct_data() {
    let mut cache = DnsCache::new();
    cache.insert(
        "list.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    let all = cache.list_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, "list.test.");
}

#[test]
fn list_all_excludes_expired_entries() {
    let mut cache = DnsCache::new();
    cache.insert(
        "expired.test.",
        RecordType::A,
        vec![],
        Duration::from_millis(0),
        false,
    );
    cache.insert(
        "valid.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );

    let all = cache.list_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, "valid.test.");
}

#[test]
fn clear_removes_all_entries() {
    let mut cache = DnsCache::new();
    cache.insert(
        "one.test.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "two.test.",
        RecordType::AAAA,
        vec![],
        Duration::from_secs(300),
        false,
    );

    cache.clear();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn clear_zone_evicts_apex_and_subdomains() {
    let mut cache = DnsCache::new();
    cache.insert(
        "example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "host.example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "other.net",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );

    cache.clear_zone("example.com");

    assert!(cache.get("example.com", RecordType::A).is_none());
    assert!(cache.get("host.example.com", RecordType::A).is_none());
    assert!(cache.get("other.net", RecordType::A).is_some());
}

#[test]
fn clear_zone_does_not_evict_parent_zone() {
    let mut cache = DnsCache::new();
    cache.insert(
        "com",
        RecordType::NS,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "example.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );

    cache.clear_zone("example.com");

    assert!(cache.get("com", RecordType::NS).is_some());
}

#[test]
fn clear_zone_does_not_evict_similar_suffix() {
    let mut cache = DnsCache::new();
    cache.insert(
        "notexample.com",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );
    cache.insert(
        "host.example.com.",
        RecordType::A,
        vec![],
        Duration::from_secs(300),
        false,
    );

    cache.clear_zone("EXAMPLE.COM.");

    assert!(cache.get("notexample.com", RecordType::A).is_some());
    assert!(cache.get("host.example.com", RecordType::A).is_none());
}

#[test]
fn cache_capacity_is_bounded() {
    let mut cache = DnsCache::new();

    for i in 0..5001 {
        cache.insert(
            &format!("host-{i}.test."),
            RecordType::A,
            vec![],
            Duration::from_secs(300),
            false,
        );
    }

    assert_eq!(cache.len(), 5000);
}

#[test]
fn cache_stats_record_hits_and_misses() {
    let stats = CacheStats::new();

    stats.record_hit();
    stats.record_hit();
    stats.record_miss();

    assert_eq!(stats.snapshot(), (2, 1));
}

#[test]
fn cache_stats_default_starts_at_zero() {
    assert_eq!(CacheStats::default().snapshot(), (0, 0));
}

#[test]
fn default_cache_is_empty() {
    let cache = DnsCache::default();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}
