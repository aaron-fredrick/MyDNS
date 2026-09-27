use crate::mydns::dns::blocklist::BlocklistIndex;

fn index(domains: &[&str]) -> BlocklistIndex {
    BlocklistIndex::from_domains(&domains.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn exact_domain_is_blocked() {
    let idx = index(&["example.com"]);
    assert!(idx.is_blocked("example.com"));
    assert!(idx.is_blocked("example.com."));
}

#[test]
fn subdomains_are_blocked() {
    let idx = index(&["example.com"]);
    assert!(idx.is_blocked("www.example.com"));
    assert!(idx.is_blocked("tracker.ads.example.com"));
}

#[test]
fn label_boundaries_are_enforced() {
    let idx = index(&["example.com"]);
    assert!(!idx.is_blocked("notexample.com"));
    assert!(!idx.is_blocked("example.com.evil.test"));
    assert!(!idx.is_blocked("example.net"));
}

#[test]
fn matching_is_case_insensitive() {
    let idx = index(&["ADS.Example.COM"]);
    assert!(idx.is_blocked("ads.example.com"));
    assert!(idx.is_blocked("WWW.ADS.EXAMPLE.COM."));
}

#[test]
fn trailing_dot_is_normalized() {
    let idx = index(&["example.com."]);
    assert!(idx.is_blocked("example.com"));
    assert!(idx.is_blocked("sub.example.com."));
}

#[test]
fn multiple_domains_are_independent() {
    let idx = index(&["ads.example.com", "tracker.other.org"]);
    assert!(idx.is_blocked("ads.example.com"));
    assert!(idx.is_blocked("sub.ads.example.com"));
    assert!(idx.is_blocked("tracker.other.org"));
    assert!(!idx.is_blocked("www.example.com"));
    assert!(!idx.is_blocked("other.org"));
}

#[test]
fn len_counts_unique_block_points() {
    let idx = index(&["a.com", "b.com", "sub.a.com"]);
    assert_eq!(idx.len(), 3);
    assert!(!idx.is_empty());
}

#[test]
fn duplicate_domains_do_not_increase_count() {
    let idx = index(&["example.com", "EXAMPLE.COM.", "example.com"]);
    assert_eq!(idx.len(), 1);
}

#[test]
fn empty_index_blocks_nothing() {
    let idx = index(&[]);
    assert!(idx.is_empty());
    assert!(!idx.is_blocked("example.com"));
}

#[test]
fn empty_domain_is_ignored() {
    let idx = index(&[".", ""]);
    assert!(idx.is_empty());
    assert!(!idx.is_blocked("example.com"));
}
