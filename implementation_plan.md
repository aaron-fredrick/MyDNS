# DNS Architecture: Radix Trie Zone Index + In-Memory Record Index

## Background

`docs/dns-data-retrieval.md` defines a layered retrieval architecture. The current implementation already covers several layers:

| Layer | Status |
|---|---|
| **B-tree DB index** (§2) | ✅ SQLite with indexed `name` column |
| **In-memory hash cache** (§3) | ✅ `DnsCache` (HashMap, TTL-aware) |
| **Positive caching** (§6) | ✅ Memory + persistent cache, prune background task |
| **Negative caching** (§7) | ✅ `NX` sentinel in DB cache, `CacheResult::Negative` in memory |
| **TTL / expiration** (§8) | ✅ `expires_at` on entries, background pruner, lazy expiration |
| **Cache invalidation** (§9) | ✅ `removeName` / `remove` called on record mutation in records API |
| **Longest-ancestor matching** (§5) | ✅ `find_longest_matching_zone` (linear scan, label-safe) |
| **Authoritative NXDOMAIN** (§10) | ✅ Returns AA=1 NXDOMAIN without upstream for zone queries |
| **Root hints in config** | ✅ `IANA_ROOT_HINTS` + `config.toml` |

**What remains / can be improved per the spec:**

| Item | Gap |
|---|---|
| **Trie / radix tree for zone lookup** (§4) | `find_longest_matching_zone` is currently an O(n×m) linear scan over zones. The spec calls for a label-based trie. |
| **In-memory zone/record index** (§11, §12) | There is no hot-path in-memory record index; every authoritative hit goes through SQLite. The spec says the DB must not be on the synchronous hot path. |
| **NODATA distinct from NXDOMAIN** (§7) | `CacheResult::Negative` covers both. Cache key does not distinguish NXDOMAIN vs. NODATA. |
| **Cache cap eviction** (§8) | Current cap is arbitrary first-key removal (random). |
| **Coherent index rebuild** (§12) | No atomic-swap index reload on record change. |

## Scope of this Change

Focus on the two most impactful items:

1. **Label-based radix trie for zone ownership lookup** — replaces the linear scan
2. **In-memory authoritative record index** — HashMap keyed by `(name, rtype)` holding the DB records, loaded at startup and updated on mutation, so authoritative hits never hit SQLite on the hot path

NODATA/NXDOMAIN distinction in the negative cache and coherent index rebuild are left for a follow-up.

---

## CNAME Chain Resolution: Gap & Solution

> [!IMPORTANT]
> **Gap identified during analysis:** The original plan proposed `RecordIndex::lookup(name, rtype) -> Option<&[DnsRecord]>` — a flat lookup. However, the current `queryDatabase` in `handler.rs` handles CNAME chains iteratively (up to 10 hops, loop detection via `HashSet`). A flat index lookup would silently drop CNAME semantics for authoritative records.

**Solution:** `RecordIndex` exposes a `resolve_authoritative(name, rtype_str) -> IndexResolution` method that mirrors `queryDatabase`'s CNAME chain traversal logic entirely in memory:

```rust
pub enum IndexResolution {
    Found(Vec<DnsRecord>),  // full answer section: CNAME records prepended, target records appended
    Nodata,                 // name exists in index but no records of the requested type
    Miss,                   // name not in index at all
}
```

- CNAME chain is built by following `CNAME` entries in the index hop-by-hop (max 10 hops).
- Loop detection uses a `HashSet<String>` of visited names, same as the DB path.
- The result `Vec<DnsRecord>` contains the ordered chain (CNAME records first, target records last) — the handler iterates this and calls `build_record` for each, using `r.record_type.parse::<RecordType>()` to recover the hickory type.
- `queryDatabase` is **removed from `processResolution`** entirely; `queryRecordIndex` replaces it.

---

## Proposed Changes

### New file: `src/dns/zone_trie.rs`

A label-inverted radix trie for O(depth) zone ownership lookup.

**Design:**
- DNS labels are stored in reverse order (TLD first) matching natural tree traversal
- e.g. `example.com` → labels `["com", "example"]` → `root → com → example`
- `find_zone(name)` walks the trie returning the deepest matching zone
- Backed by a simple `HashMap<String, TrieNode>` at each level — no external crates needed

```rust
pub struct ZoneTrie { root: TrieNode }
impl ZoneTrie {
    pub fn from_zones(zones: &[String]) -> Self;
    pub fn find_zone<'a>(&'a self, name: &str) -> Option<&'a str>;
}
```

**Correctness:** Matches only at label boundaries — `notexample.com` cannot match zone `example.com`.

---

### New file: `src/dns/record_index.rs`

An in-memory authoritative record index with CNAME-chain-aware resolution.

```rust
pub enum IndexResolution { Found(Vec<DnsRecord>), Nodata, Miss }

pub struct RecordIndex { inner: HashMap<(String, String), Vec<DnsRecord>> }
impl RecordIndex {
    pub async fn load_from_db(db: &SqlitePool) -> anyhow::Result<Self>;
    pub fn upsert(&mut self, record: DnsRecord);
    pub fn remove(&mut self, name: &str, rtype: Option<&str>);
    pub fn remove_by_id(&mut self, id: i64);
    pub fn resolve_authoritative(&self, name: &str, rtype_str: &str) -> IndexResolution;
}
```

- Loaded eagerly at startup from `db::records::listRecords`
- Updated incrementally on every management API mutation (create/update/delete)
- Wrapped in `Arc<RwLock<RecordIndex>>` in `AppState` — concurrent read, exclusive write

---

### Modified: [`src/dns/mod.rs`](file:///d:/projects/MyDNS/src/dns/mod.rs)

Add `pub mod zone_trie;` and `pub mod record_index;`.

---

### Modified: [`src/dns/handler.rs`](file:///d:/projects/MyDNS/src/dns/handler.rs)

- Replace `find_longest_matching_zone` (linear scan) with `ZoneTrie::find_zone` via `state.zone_trie`
- Add `queryRecordIndex` method that calls `RecordIndex::resolve_authoritative` and converts `IndexResolution` → `ResolutionResult`
- In `processResolution` query ordering:
  1. Memory cache (existing)
  2. **Record index** (`queryRecordIndex`) — new hot path, avoids DB for authoritative records
  3. Persistent cache (existing, for cached upstream results)
  4. Special records (loopback PTR / dashboard)
  5. Authoritative NXDOMAIN if in-zone (existing logic, now using trie)
  6. Upstream if recursion desired and not in zone (existing)
- Remove `queryDatabase` method entirely (replaced by index)
- Migrate `test_find_longest_matching_zone` in `src/dns/tests.rs` to use `ZoneTrie`

---

### Modified: [`src/state.rs`](file:///d:/projects/MyDNS/src/state.rs)

Add `record_index: Arc<RwLock<RecordIndex>>` and `zone_trie: Arc<RwLock<ZoneTrie>>`.
`AppState::new` gains two new parameters: `record_index: RecordIndex, zone_trie: ZoneTrie`.

---

### Modified: [`src/main.rs`](file:///d:/projects/MyDNS/src/main.rs)

Eagerly build `ZoneTrie` from `cfg.allowed_zones` and load `RecordIndex` from DB before constructing `AppState`.

---

### Modified: [`src/web/records_api.rs`](file:///d:/projects/MyDNS/src/web/records_api.rs)

After every mutation:
- `createRecord`: `index.upsert(row.clone())`
- `updateRecord`: `index.remove_by_id(id); index.upsert(updated.clone())`
- `deleteRecord`: `index.remove_by_id(id)`

---

### Modified: Integration tests (5 files)

All five test files call `AppState::new` with the old signature. Each needs to build a `RecordIndex` (loaded from the test DB) and `ZoneTrie` (from test config's `allowed_zones`) before calling the updated `AppState::new`.

Files affected:
- [`tests/integration.rs`](file:///d:/projects/MyDNS/tests/integration.rs)
- [`tests/dns_integration.rs`](file:///d:/projects/MyDNS/tests/dns_integration.rs)
- [`tests/upstream_integration.rs`](file:///d:/projects/MyDNS/tests/upstream_integration.rs)
- [`tests/validation_api.rs`](file:///d:/projects/MyDNS/tests/validation_api.rs)
- [`tests/auth_coverage.rs`](file:///d:/projects/MyDNS/tests/auth_coverage.rs)

---

## Verification Plan

### Automated Tests

```powershell
cargo test --lib   # All unit tests including new ZoneTrie and RecordIndex tests
cargo test         # All integration tests
```

New unit tests:
- `zone_trie::tests::*` — exact, subdomain, non-match, longest-ancestor, empty, trailing-dot cases
- `record_index::tests::*` — lookup, miss, nodata, cname chain, upsert replace-by-id, remove by type, remove all

### Manual Verification
- Run `cargo run` and confirm `nslookup` resolves authoritative records
- Confirm a record update via the API is reflected immediately in subsequent DNS queries
- Check tracing logs show `[INDEX]` source instead of `[DB]` for authoritative hits
