#[cfg(test)]
mod cache_tests {
    use std::time::Duration;
    use hickory_proto::rr::RecordType;
    use crate::cache::DnsCache;

    #[test]
    fn insert_and_hit() {
        let mut cache = DnsCache::new();
        cache.insert("example.com.", RecordType::A, vec![], Duration::from_secs(300));
        assert!(
            cache.get("example.com.", RecordType::A).is_some(),
            "Expected a cache hit immediately after insert"
        );
    }

    #[test]
    fn miss_on_wrong_type() {
        let mut cache = DnsCache::new();
        cache.insert("example.com.", RecordType::A, vec![], Duration::from_secs(300));
        assert!(
            cache.get("example.com.", RecordType::AAAA).is_none(),
            "A record should not match an AAAA query"
        );
    }

    #[test]
    fn expired_entry_returns_none() {
        let mut cache = DnsCache::new();
        // Insert with an already-expired TTL.
        cache.insert("example.com.", RecordType::A, vec![], Duration::from_millis(0));
        // Even without sleeping, Duration(0) means expires_at == now(), so it's expired.
        assert!(
            cache.get("example.com.", RecordType::A).is_none(),
            "Entry with zero TTL should be immediately expired"
        );
    }

    #[test]
    fn prune_removes_expired_entries() {
        let mut cache = DnsCache::new();
        cache.insert("expired.test.", RecordType::A, vec![], Duration::from_millis(0));
        cache.insert("valid.test.", RecordType::A, vec![], Duration::from_secs(300));
        let pruned = cache.prune();
        assert_eq!(pruned, 1, "Prune should remove exactly one expired entry");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn remove_specific_entry() {
        let mut cache = DnsCache::new();
        cache.insert("target.test.", RecordType::A, vec![], Duration::from_secs(300));
        cache.remove("target.test.", RecordType::A);
        assert!(cache.get("target.test.", RecordType::A).is_none());
    }
}
