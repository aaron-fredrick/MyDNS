use crate::mydns::dns::zone_trie::ZoneTrie;

fn trie(zones: &[&str]) -> ZoneTrie {
    ZoneTrie::from_zones(&zones.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn finds_exact_zone() {
    let trie = trie(&["example.com"]);
    assert_eq!(trie.find_zone("example.com"), Some("example.com"));
}

#[test]
fn finds_subdomain_zone() {
    let trie = trie(&["example.com"]);
    assert_eq!(trie.find_zone("api.example.com"), Some("example.com"));
}

#[test]
fn finds_deep_subdomain_zone() {
    let trie = trie(&["example.com"]);
    assert_eq!(trie.find_zone("a.b.c.example.com"), Some("example.com"));
}

#[test]
fn chooses_longest_matching_zone() {
    let trie = trie(&["example.com", "sub.example.com"]);
    assert_eq!(trie.find_zone("api.sub.example.com"), Some("sub.example.com"));
    assert_eq!(trie.find_zone("www.example.com"), Some("example.com"));
}

#[test]
fn enforces_label_boundaries() {
    let trie = trie(&["example.com"]);
    assert_eq!(trie.find_zone("notexample.com"), None);
    assert_eq!(trie.find_zone("example.com.evil"), None);
}

#[test]
fn unrelated_name_returns_none() {
    let trie = trie(&["example.com"]);
    assert_eq!(trie.find_zone("other.org"), None);
}

#[test]
fn normalizes_case_and_trailing_dot() {
    let trie = trie(&["Example.COM."]);
    assert_eq!(trie.find_zone("WWW.Example.Com."), Some("Example.COM."));
}

#[test]
fn supports_multiple_unrelated_zones() {
    let trie = trie(&["home.local", "lab.local", "example.com"]);
    assert_eq!(trie.find_zone("server.home.local"), Some("home.local"));
    assert_eq!(trie.find_zone("node.lab.local"), Some("lab.local"));
    assert_eq!(trie.find_zone("www.example.com"), Some("example.com"));
}

#[test]
fn empty_trie_returns_none() {
    let trie = trie(&[]);
    assert_eq!(trie.find_zone("anything.com"), None);
}

#[test]
fn root_zone_matches_every_name() {
    let trie = trie(&["."]);
    assert!(trie.is_root_authoritative());
    assert_eq!(trie.find_zone("google.com"), Some("."));
    assert_eq!(trie.find_zone("anything.example.org"), Some("."));
}

#[test]
fn root_zone_takes_precedence_over_specific_zones() {
    let trie = trie(&["home.local", "."]);
    assert_eq!(trie.find_zone("home.local"), Some("."));
    assert_eq!(trie.find_zone("google.com"), Some("."));
}
