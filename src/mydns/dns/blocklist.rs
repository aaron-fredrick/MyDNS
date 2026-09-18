//! In-memory domain blocklist index.
//!
//! [`BlocklistIndex`] stores blocked domains in a reversed-label trie so that
//! both exact-domain and subdomain lookups resolve in O(label count) time
//! without any SQLite access on the DNS hot path.
//!
//! # Matching semantics
//!
//! If `example.com` is blocked then:
//! - `example.com`                → **blocked** (exact)
//! - `www.example.com`            → **blocked** (subdomain)
//! - `tracker.ads.example.com`    → **blocked** (deep subdomain)
//! - `notexample.com`             → **allowed** (different TLD suffix)
//! - `example.com.evil.test`      → **allowed** (label boundary enforced)
//!
//! The trie is traversed from the TLD inward (right-to-left label order).
//! A node is a **block point** when its `blocked` flag is set.  Traversal
//! stops at the first node that has no matching child; the result is
//! blocked if and only if a block-point was reached somewhere along the path.

use std::collections::HashMap;

#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    /// True when this node itself is a configured block point.
    blocked: bool,
}

/// Reversed-label trie for O(depth) domain-and-subdomain blocklist lookups.
///
/// Wrapped in `Arc<RwLock<BlocklistIndex>>` in [`AppState`] so the DNS
/// hot path can read without blocking and API mutations can swap the whole
/// index atomically.
#[derive(Debug, Default)]
pub struct BlocklistIndex {
    root: TrieNode,
}

impl BlocklistIndex {
    /// Builds an index from a slice of pre-normalised canonical domain strings
    /// (lowercase, no trailing dot).
    pub fn from_domains(domains: &[String]) -> Self {
        let mut index = Self::default();
        for domain in domains {
            index.insert(domain);
        }
        index
    }

    /// Returns the number of blocked domains in the index.
    pub fn len(&self) -> usize {
        count_block_points(&self.root)
    }

    /// Returns `true` when no domains are blocked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn insert(&mut self, domain: &str) {
        let normalized = domain.trim_end_matches('.').to_lowercase();
        if normalized.is_empty() {
            return;
        }
        let labels: Vec<&str> = normalized.split('.').rev().collect();
        let mut node = &mut self.root;
        for label in &labels {
            node = node.children.entry(label.to_string()).or_default();
        }
        node.blocked = true;
    }

    /// Returns `true` when `name` or any ancestor zone matches a blocked entry.
    ///
    /// Traverses from the TLD inward; as soon as a block point is encountered
    /// anywhere along the matched path the query is blocked. This covers both
    /// the exact domain and all subdomains in one pass.
    pub fn is_blocked(&self, name: &str) -> bool {
        let normalized = name.trim_end_matches('.').to_lowercase();
        if normalized.is_empty() {
            return false;
        }
        let labels: Vec<&str> = normalized.split('.').rev().collect();
        let mut node = &self.root;
        for label in &labels {
            match node.children.get(*label) {
                Some(child) => {
                    if child.blocked {
                        // Block point reached: this domain (or a parent) is blocked.
                        return true;
                    }
                    node = child;
                }
                None => return false,
            }
        }
        false
    }
}

fn count_block_points(node: &TrieNode) -> usize {
    let self_count = if node.blocked { 1 } else { 0 };
    self_count
        + node
            .children
            .values()
            .map(count_block_points)
            .sum::<usize>()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::BlocklistIndex;

    fn index(domains: &[&str]) -> BlocklistIndex {
        BlocklistIndex::from_domains(&domains.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    // ── exact match ──────────────────────────────────────────────────────────

    #[test]
    fn exact_domain_blocked() {
        let idx = index(&["example.com"]);
        assert!(idx.is_blocked("example.com"));
    }

    #[test]
    fn exact_domain_with_trailing_dot() {
        let idx = index(&["example.com"]);
        assert!(idx.is_blocked("example.com."));
    }

    // ── subdomain match ───────────────────────────────────────────────────────

    #[test]
    fn subdomain_is_blocked() {
        let idx = index(&["example.com"]);
        assert!(idx.is_blocked("www.example.com"));
        assert!(idx.is_blocked("ads.example.com"));
        assert!(idx.is_blocked("tracker.ads.example.com"));
    }

    // ── label boundary — must NOT match unrelated names ───────────────────────

    #[test]
    fn different_tld_not_blocked() {
        let idx = index(&["example.com"]);
        assert!(!idx.is_blocked("notexample.com"));
    }

    #[test]
    fn suffix_in_parent_label_not_blocked() {
        // "example.com.evil.test" must NOT be blocked even though it contains
        // the string "example.com".
        let idx = index(&["example.com"]);
        assert!(!idx.is_blocked("example.com.evil.test"));
    }

    #[test]
    fn unrelated_domain_not_blocked() {
        let idx = index(&["example.com"]);
        assert!(!idx.is_blocked("other.org"));
        assert!(!idx.is_blocked("example.net"));
    }

    // ── case normalisation ────────────────────────────────────────────────────

    #[test]
    fn query_uppercase_blocked() {
        let idx = index(&["example.com"]);
        assert!(idx.is_blocked("ADS.EXAMPLE.COM"));
    }

    #[test]
    fn stored_uppercase_normalised() {
        let idx = index(&["ADS.Example.COM"]);
        assert!(idx.is_blocked("ads.example.com"));
    }

    // ── trailing dot normalisation ────────────────────────────────────────────

    #[test]
    fn stored_with_trailing_dot_normalised() {
        let idx = index(&["example.com."]);
        assert!(idx.is_blocked("example.com"));
        assert!(idx.is_blocked("sub.example.com"));
    }

    // ── empty index ───────────────────────────────────────────────────────────

    #[test]
    fn empty_index_blocks_nothing() {
        let idx = index(&[]);
        assert!(!idx.is_blocked("example.com"));
        assert!(idx.is_empty());
    }

    // ── multiple entries ──────────────────────────────────────────────────────

    #[test]
    fn multiple_blocked_domains() {
        let idx = index(&["ads.example.com", "tracker.other.org"]);
        assert!(idx.is_blocked("ads.example.com"));
        assert!(idx.is_blocked("sub.ads.example.com"));
        assert!(idx.is_blocked("tracker.other.org"));
        assert!(!idx.is_blocked("www.example.com")); // only ads.example.com blocked
        assert!(!idx.is_blocked("other.org"));
    }

    // ── len ───────────────────────────────────────────────────────────────────

    #[test]
    fn len_counts_block_points() {
        let idx = index(&["a.com", "b.com", "sub.a.com"]);
        assert_eq!(idx.len(), 3);
    }

    // ── disabled entry is not in index ───────────────────────────────────────

    #[test]
    fn disabled_domain_not_in_index() {
        // BlocklistIndex only contains what is passed in — the caller (startup
        // and API) is responsible for filtering to enabled entries.
        let idx = index(&[]); // nothing passed → nothing blocked
        assert!(!idx.is_blocked("ads.example.com"));
    }
}
