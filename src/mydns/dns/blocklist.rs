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
