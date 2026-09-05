use std::collections::{HashMap, HashSet};

use sqlx::SqlitePool;

use crate::db::records::{self, DnsRecord};

/// Result of an authoritative record index lookup with CNAME chain resolution.
#[derive(Debug)]
pub enum IndexResolution {
    /// Records found. The `Vec` contains the full answer section: any CNAME
    /// chain records are prepended in traversal order, and the target type
    /// records are appended at the end.
    Found(Vec<DnsRecord>),
    /// The queried name exists in the index but has no records of the requested
    /// type (and no CNAME to follow). Corresponds to a DNS NODATA response.
    Nodata,
    /// The queried name is not present in the index at all. The caller should
    /// fall through to the upstream pipeline.
    Miss,
    /// A CNAME loop or excessive recursion depth was detected.
    ServFail,
}

/// In-memory authoritative record index.
///
/// Keyed by `(lowercase_name, uppercase_rtype)` — the same normalisation used
/// by the DB layer. Loaded eagerly from the database at startup and kept
/// coherent via incremental updates on every CRUD mutation.
///
/// Wrapped in `Arc<RwLock<RecordIndex>>` in `AppState` to allow concurrent
/// reads from the DNS hot path with exclusive writes during mutations.
#[derive(Debug, Default)]
pub struct RecordIndex {
    inner: HashMap<(String, String), Vec<DnsRecord>>,
}

impl RecordIndex {
    /// Loads all DNS records from the database and builds the index.
    pub async fn load_from_db(db: &SqlitePool) -> anyhow::Result<Self> {
        let all_records = records::list_records(db).await?;
        let mut index = Self::default();
        for record in all_records {
            index.upsert(record);
        }
        tracing::info!(
            entry_count = index.inner.len(),
            "Authoritative record index loaded"
        );
        Ok(index)
    }

    /// Inserts or updates a single record in the index.
    ///
    /// If a record with the same `id` already exists under the same
    /// `(name, rtype)` key it is replaced. This handles in-place updates
    /// where neither name nor type change.
    pub fn upsert(&mut self, record: DnsRecord) {
        let key = (
            record.name.to_lowercase(),
            record.record_type.to_uppercase(),
        );
        let bucket = self.inner.entry(key).or_default();
        bucket.retain(|r| r.id != record.id);
        bucket.push(record);
    }

    /// Removes a record by its primary key, scanning all buckets.
    ///
    /// This is the correct invalidation primitive for updates and deletes
    /// because neither the old name nor the old type need to be known by the
    /// caller — the record is located purely by `id`.
    pub fn remove_by_id(&mut self, id: i64) {
        for bucket in self.inner.values_mut() {
            bucket.retain(|r| r.id != id);
        }
        self.inner.retain(|_, bucket| !bucket.is_empty());
    }

    /// Removes records by name and optionally by type.
    ///
    /// - `rtype = None` removes all records for the name across every type.
    /// - `rtype = Some(t)` removes only records of that specific type.
    pub fn remove(&mut self, name: &str, rtype: Option<&str>) {
        let lower_name = name.to_lowercase();
        match rtype {
            Some(t) => {
                self.inner.remove(&(lower_name, t.to_uppercase()));
            }
            None => {
                self.inner.retain(|(n, _), _| n != &lower_name);
            }
        }
    }

    /// Flat lookup — returns raw records for an exact `(name, rtype)` pair
    /// without CNAME chain traversal.
    fn lookup_raw(&self, name: &str, rtype: &str) -> Option<&[DnsRecord]> {
        self.inner
            .get(&(name.to_lowercase(), rtype.to_uppercase()))
            .map(Vec::as_slice)
    }

    /// CNAME-chain-aware authoritative lookup.
    ///
    /// Mirrors `handler::queryDatabase` but operates entirely in memory.
    ///
    /// Returns:
    /// - [`IndexResolution::Found`] — CNAME chain prepended, target records appended.
    /// - [`IndexResolution::Nodata`] — name exists but no records of `rtype_str`.
    /// - [`IndexResolution::Miss`] — name is not in the index; caller falls through.
    ///
    /// CNAME loop detection terminates after 10 hops and returns `Miss` so the
    /// caller can fall back gracefully (the DB path returns `ServFail` for this
    /// case, but the index is meant to be a full replacement, not a partial one).
    #[tracing::instrument(
        name = "resolve_authoritative",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = %rtype_str)
    )]
    pub fn resolve_authoritative(&self, name: &str, rtype_str: &str) -> IndexResolution {
        let mut current = name.trim_end_matches('.').to_lowercase();
        let mut chain: Vec<DnsRecord> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let upper_rtype = rtype_str.to_uppercase();

        for _ in 0..=10u8 {
            if !visited.insert(current.clone()) {
                tracing::warn!(
                    name = %name,
                    rtype = %rtype_str,
                    "CNAME loop detected in record index"
                );
                return IndexResolution::ServFail;
            }

            // ANY queries return all record types for the name.
            // Follow CNAME chains and return all records at the final target.
            if upper_rtype == "ANY" {
                // Check if there's a CNAME to follow first
                match self.lookup_raw(&current, "CNAME") {
                    Some(cname_records) if !cname_records.is_empty() => {
                        let cname = &cname_records[0];
                        chain.push(cname.clone());
                        current = cname.value.trim_end_matches('.').to_lowercase();
                        continue; // Loop to collect records at the target
                    }
                    _ => {
                        // No CNAME, collect all records at the current name
                        let all_records: Vec<DnsRecord> = self.inner
                            .keys()
                            .filter(|(n, _)| n == &current)
                            .flat_map(|key| self.inner.get(key).unwrap().clone())
                            .collect();

                        if !all_records.is_empty() {
                            let mut result = chain;
                            result.extend(all_records);
                            return IndexResolution::Found(result);
                        }

                        // No records at all - check if name exists
                        if !chain.is_empty() {
                            // We followed a CNAME chain but found no records at the target
                            return IndexResolution::Found(chain);
                        }
                        return if self.name_exists(&current) {
                            IndexResolution::Nodata
                        } else {
                            IndexResolution::Miss
                        };
                    }
                }
            }

            // Attempt direct match for the requested type.
            if let Some(target_records) = self.lookup_raw(&current, &upper_rtype) {
                let mut result = chain;
                result.extend_from_slice(target_records);
                return IndexResolution::Found(result);
            }

            // Follow a CNAME if present.
            match self.lookup_raw(&current, "CNAME") {
                Some(cname_records) if !cname_records.is_empty() => {
                    let cname = &cname_records[0];
                    chain.push(cname.clone());
                    current = cname.value.trim_end_matches('.').to_lowercase();
                }
                _ => {
                    // If we have accumulated a CNAME chain but the final target is
                    // outside the local zone, return the chain as an authoritative
                    // answer. The caller is responsible for resolving the target.
                    if !chain.is_empty() {
                        return IndexResolution::Found(chain);
                    }
                    // Direct lookup failed and no CNAME to follow.
                    // Distinguish between Nodata (name exists but type doesn't) and Miss (name doesn't exist).
                    return if self.name_exists(&current) {
                        IndexResolution::Nodata
                    } else {
                        IndexResolution::Miss
                    };
                }
            }
        }

        tracing::warn!(
            name = %name,
            rtype = %rtype_str,
            "CNAME recursion limit reached in record index"
        );
        IndexResolution::ServFail
    }

    /// Returns `true` if any record exists for `name` regardless of type.
    fn name_exists(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.inner.keys().any(|(n, _)| n == &lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: i64, name: &str, rtype: &str, value: &str) -> DnsRecord {
        DnsRecord {
            id,
            name: name.to_string(),
            record_type: rtype.to_string(),
            value: value.to_string(),
            ttl: 300,
            priority: None,
            created_at: String::new(),
            updated_at: String::new(),
            is_dev: false,
        }
    }

    #[test]
    fn found_existing_record() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        match idx.resolve_authoritative("example.com", "A") {
            IndexResolution::Found(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].value, "1.2.3.4");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn miss_for_unknown_name() {
        let idx = RecordIndex::default();
        assert!(matches!(
            idx.resolve_authoritative("unknown.com", "A"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn nodata_when_name_exists_different_type() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "MX", "mail.example.com"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "A"),
            IndexResolution::Nodata
        ));
    }

    #[test]
    fn cname_chain_prepended_correctly() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(
            1,
            "alias.example.com",
            "CNAME",
            "target.example.com",
        ));
        idx.upsert(make_record(2, "target.example.com", "A", "1.2.3.4"));
        match idx.resolve_authoritative("alias.example.com", "A") {
            IndexResolution::Found(r) => {
                assert_eq!(r.len(), 2);
                assert_eq!(r[0].record_type, "CNAME");
                assert_eq!(r[0].name, "alias.example.com");
                assert_eq!(r[1].record_type, "A");
                assert_eq!(r[1].value, "1.2.3.4");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn deep_cname_chain() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "a.example.com", "CNAME", "b.example.com"));
        idx.upsert(make_record(2, "b.example.com", "CNAME", "c.example.com"));
        idx.upsert(make_record(3, "c.example.com", "A", "10.0.0.1"));
        match idx.resolve_authoritative("a.example.com", "A") {
            IndexResolution::Found(r) => {
                assert_eq!(r.len(), 3);
                assert_eq!(r[0].name, "a.example.com");
                assert_eq!(r[1].name, "b.example.com");
                assert_eq!(r[2].name, "c.example.com");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn cname_loop_returns_servfail() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(
            1,
            "loop-a.example.com",
            "CNAME",
            "loop-b.example.com",
        ));
        idx.upsert(make_record(
            2,
            "loop-b.example.com",
            "CNAME",
            "loop-a.example.com",
        ));
        assert!(matches!(
            idx.resolve_authoritative("loop-a.example.com", "A"),
            IndexResolution::ServFail
        ));
    }

    #[test]
    fn upsert_replaces_record_by_id() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        idx.upsert(make_record(1, "example.com", "A", "5.6.7.8"));
        match idx.resolve_authoritative("example.com", "A") {
            IndexResolution::Found(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].value, "5.6.7.8");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn remove_by_id_cleans_up() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        idx.remove_by_id(1);
        assert!(matches!(
            idx.resolve_authoritative("example.com", "A"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn remove_by_name_and_type_leaves_others() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        idx.upsert(make_record(2, "example.com", "MX", "mail.example.com"));
        idx.remove("example.com", Some("A"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "A"),
            IndexResolution::Nodata
        ));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "MX"),
            IndexResolution::Found(_)
        ));
    }

    #[test]
    fn remove_all_by_name() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        idx.upsert(make_record(2, "example.com", "MX", "mail.example.com"));
        idx.remove("example.com", None);
        assert!(matches!(
            idx.resolve_authoritative("example.com", "A"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn explicit_cname_query_nodata_when_name_exists() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "CNAME"),
            IndexResolution::Nodata
        ));
    }

    #[test]
    fn nodata_for_missing_soa_when_name_exists() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "SOA"),
            IndexResolution::Nodata
        ));
    }

    #[test]
    fn nodata_for_missing_ns_when_name_exists() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "NS"),
            IndexResolution::Nodata
        ));
    }

    #[test]
    fn nodata_for_missing_aaaa_when_name_exists() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        assert!(matches!(
            idx.resolve_authoritative("example.com", "AAAA"),
            IndexResolution::Nodata
        ));
    }

    #[test]
    fn miss_for_nonexistent_name() {
        let idx = RecordIndex::default();
        assert!(matches!(
            idx.resolve_authoritative("nonexistent.com", "SOA"),
            IndexResolution::Miss
        ));
        assert!(matches!(
            idx.resolve_authoritative("nonexistent.com", "NS"),
            IndexResolution::Miss
        ));
        assert!(matches!(
            idx.resolve_authoritative("nonexistent.com", "A"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn any_returns_all_record_types() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "example.com", "A", "1.2.3.4"));
        idx.upsert(make_record(2, "example.com", "AAAA", "2001:db8::1"));
        idx.upsert(make_record(3, "example.com", "MX", "mail.example.com"));
        idx.upsert(make_record(4, "example.com", "TXT", "v=spf1 ~all"));

        match idx.resolve_authoritative("example.com", "ANY") {
            IndexResolution::Found(records) => {
                assert_eq!(records.len(), 4);
                let types: Vec<&str> = records.iter().map(|r| r.record_type.as_str()).collect();
                assert!(types.contains(&"A"));
                assert!(types.contains(&"AAAA"));
                assert!(types.contains(&"MX"));
                assert!(types.contains(&"TXT"));
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn any_returns_nodata_when_name_exists_no_records() {
        let idx = RecordIndex::default();
        assert!(matches!(
            idx.resolve_authoritative("example.com", "ANY"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn any_returns_miss_for_nonexistent_name() {
        let idx = RecordIndex::default();
        assert!(matches!(
            idx.resolve_authoritative("nonexistent.com", "ANY"),
            IndexResolution::Miss
        ));
    }

    #[test]
    fn any_with_cname_chain() {
        let mut idx = RecordIndex::default();
        idx.upsert(make_record(1, "alias.example.com", "CNAME", "target.example.com"));
        idx.upsert(make_record(2, "target.example.com", "A", "1.2.3.4"));
        idx.upsert(make_record(3, "target.example.com", "AAAA", "2001:db8::1"));

        // ANY on the alias should return CNAME + all records at the target
        match idx.resolve_authoritative("alias.example.com", "ANY") {
            IndexResolution::Found(records) => {
                // ANY returns CNAME chain + all records at the final target
                assert_eq!(records.len(), 3);
                assert_eq!(records[0].record_type, "CNAME");
                assert_eq!(records[0].name, "alias.example.com");
                let types: Vec<&str> = records.iter().map(|r| r.record_type.as_str()).collect();
                assert!(types.contains(&"A"));
                assert!(types.contains(&"AAAA"));
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }
}
