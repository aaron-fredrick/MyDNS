use std::collections::HashMap;

/// A single node in the label-inverted trie.
#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    /// The original zone string stored at this node when it is a zone terminus.
    zone: Option<String>,
}

/// Label-inverted radix trie for O(depth) authoritative zone ownership lookup.
///
/// DNS names are stored with labels reversed (TLD first), so a single top-down
/// traversal finds the longest matching ancestor zone in one pass.
///
/// Example: zone `example.com` is stored as `root → "com" → "example"`.
/// A query for `sub.example.com` walks `"com" → "example"` and returns the
/// zone terminating at `"example"`.
///
/// Matching is strictly label-boundary-safe: `notexample.com` will not match
/// a configured zone of `example.com`.
///
/// Special case: inserting `"."` sets `root_authoritative = true`. When this
/// flag is set, `find_zone` returns `Some(".")` for every query without trie
/// traversal — the root zone is authoritative for the entire DNS namespace.
#[derive(Debug, Default)]
pub struct ZoneTrie {
    root: TrieNode,
    /// True when the root zone `"."` is configured. Overrides every other zone.
    root_authoritative: bool,
}

impl ZoneTrie {
    /// Builds a trie from a slice of zone strings (e.g. `["example.com", "home.local"]`).
    pub fn from_zones(zones: &[String]) -> Self {
        let mut trie = Self::default();
        for zone in zones {
            trie.insert(zone);
        }
        trie
    }

    fn insert(&mut self, zone: &str) {
        // The root zone "." is represented as the root_authoritative flag so
        // that find_zone can short-circuit without traversal.
        if zone == "." {
            self.root_authoritative = true;
            return;
        }
        let normalized = zone.trim_end_matches('.').to_lowercase();
        if normalized.is_empty() {
            return;
        }
        let labels: Vec<&str> = normalized.split('.').rev().collect();
        let mut node = &mut self.root;
        for label in &labels {
            node = node.children.entry(label.to_string()).or_default();
        }
        // Store the original zone string so callers get back what they configured.
        node.zone = Some(zone.to_string());
    }

    /// Returns `true` when the root zone `"."` is configured. In this mode
    /// every DNS name is considered to fall within this server's authority.
    pub fn is_root_authoritative(&self) -> bool {
        self.root_authoritative
    }

    /// Returns the deepest matching authoritative zone for `name`, or `None`
    /// if the name does not fall within any configured zone.
    ///
    /// When the root zone `"."` is configured, always returns `Some(".")`.
    pub fn find_zone<'a>(&'a self, name: &str) -> Option<&'a str> {
        if self.root_authoritative {
            return Some(".");
        }
        let normalized = name.trim_end_matches('.').to_lowercase();
        let labels: Vec<&str> = normalized.split('.').rev().collect();
        let mut node = &self.root;
        let mut best: Option<&'a str> = None;

        for label in &labels {
            match node.children.get(*label) {
                Some(child) => {
                    if let Some(ref z) = child.zone {
                        best = Some(z.as_str());
                    }
                    node = child;
                }
                None => break,
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::ZoneTrie;

    fn trie(zones: &[&str]) -> ZoneTrie {
        ZoneTrie::from_zones(&zones.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn exact_zone_match() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("example.com"), Some("example.com"));
    }

    #[test]
    fn subdomain_match() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("sub.example.com"), Some("example.com"));
    }

    #[test]
    fn deep_subdomain_match() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("a.b.c.example.com"), Some("example.com"));
    }

    #[test]
    fn no_partial_label_match() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("notexample.com"), None);
    }

    #[test]
    fn longest_ancestor_wins() {
        let t = trie(&["example.com", "sub.example.com"]);
        assert_eq!(t.find_zone("api.sub.example.com"), Some("sub.example.com"));
        assert_eq!(t.find_zone("www.example.com"), Some("example.com"));
    }

    #[test]
    fn unrelated_zone_returns_none() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("other.org"), None);
    }

    #[test]
    fn empty_trie_returns_none() {
        let t = trie(&[]);
        assert_eq!(t.find_zone("anything.com"), None);
    }

    #[test]
    fn trailing_dot_normalised_on_query() {
        let t = trie(&["example.com"]);
        assert_eq!(t.find_zone("sub.example.com."), Some("example.com"));
    }

    #[test]
    fn multiple_unrelated_zones() {
        let t = trie(&["home.local", "lab.local", "example.com"]);
        assert_eq!(t.find_zone("server.home.local"), Some("home.local"));
        assert_eq!(t.find_zone("node.lab.local"), Some("lab.local"));
        assert_eq!(t.find_zone("www.example.com"), Some("example.com"));
        assert_eq!(t.find_zone("google.com"), None);
    }

    // ── Root zone "." semantics ───────────────────────────────────────────────

    #[test]
    fn root_zone_is_authoritative_for_everything() {
        let t = trie(&["."]);
        assert_eq!(t.find_zone("google.com"), Some("."));
        assert_eq!(t.find_zone("anything.example.org"), Some("."));
        assert_eq!(t.find_zone("home.local"), Some("."));
    }

    #[test]
    fn root_zone_flag_set_correctly() {
        let empty = trie(&[]);
        assert!(!empty.is_root_authoritative());
        let root = trie(&["."]);
        assert!(root.is_root_authoritative());
    }

    #[test]
    fn root_zone_alongside_other_zones() {
        // "." takes precedence — every name matches "."
        let t = trie(&["home.local", "."]);
        assert_eq!(t.find_zone("home.local"), Some("."));
        assert_eq!(t.find_zone("google.com"), Some("."));
    }

    #[test]
    fn empty_trie_root_not_set() {
        let t = trie(&[]);
        assert!(!t.is_root_authoritative());
        assert_eq!(t.find_zone("anything.com"), None);
    }
}
