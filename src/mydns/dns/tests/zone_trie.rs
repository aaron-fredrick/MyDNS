use super::fixtures::test_name;
use crate::mydns::dns::zone_trie::ZoneTrie;

#[test]
fn finds_longest_matching_zone() {
    let zones = vec![
        "example.com".to_string(),
        "sub.example.com".to_string(),
        "mydns.local".to_string(),
    ];
    let trie = ZoneTrie::from_zones(&zones);

    assert_eq!(
        trie.find_zone("api.sub.example.com"),
        Some("sub.example.com")
    );
    assert_eq!(trie.find_zone("example.com"), Some("example.com"));
    assert_eq!(
        trie.find_zone(&test_name().to_string()),
        Some("example.com")
    );
}

#[test]
fn respects_dns_label_boundaries() {
    let trie = ZoneTrie::from_zones(&["example.com".to_string()]);

    assert_eq!(trie.find_zone("notexample.com"), None);
    assert_eq!(trie.find_zone("google.com"), None);
    assert_eq!(trie.find_zone("example.com.evil"), None);
}

#[test]
fn normalizes_trailing_dots_and_case() {
    let trie = ZoneTrie::from_zones(&["Example.COM.".to_string()]);

    assert_eq!(trie.find_zone("WWW.Example.Com."), Some("example.com"));
}
