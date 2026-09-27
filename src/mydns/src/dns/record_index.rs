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
    owner_names: HashSet<String>,
}

impl RecordIndex {
    fn normalize_name(name: &str) -> String {
        name.trim_end_matches('.').to_lowercase()
    }

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
        index.rebuild_owner_names();
        Ok(index)
    }

    fn rebuild_owner_names(&mut self) {
        self.owner_names.clear();
        for (name, _) in self.inner.keys() {
            let trimmed = name.trim_end_matches('.');
            self.owner_names.insert(trimmed.to_string());
            let mut current = trimmed;
            while let Some(idx) = current.find('.') {
                current = &current[idx + 1..];
                if current.is_empty() {
                    break;
                }
                self.owner_names.insert(current.to_string());
            }
        }
    }

    /// Inserts or updates a single record in the index.
    ///
    /// If a record with the same `id` already exists under the same
    /// `(name, rtype)` key it is replaced. This handles in-place updates
    /// where neither name nor type change.
    pub fn upsert(&mut self, record: DnsRecord) {
        let key = (
            Self::normalize_name(&record.name),
            record.record_type.to_uppercase(),
        );
        let bucket = self.inner.entry(key).or_default();
        bucket.retain(|r| r.id != record.id);
        bucket.push(record);
        self.rebuild_owner_names();
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
        self.rebuild_owner_names();
    }

    /// Removes records by name and optionally by type.
    ///
    /// - `rtype = None` removes all records for the name across every type.
    /// - `rtype = Some(t)` removes only records of that specific type.
    pub fn remove(&mut self, name: &str, rtype: Option<&str>) {
        let lower_name = Self::normalize_name(name);
        match rtype {
            Some(t) => {
                self.inner.remove(&(lower_name, t.to_uppercase()));
            }
            None => {
                self.inner.retain(|(n, _), _| n != &lower_name);
            }
        }
        self.rebuild_owner_names();
    }

    /// Flat lookup — returns raw records for an exact `(name, rtype)` pair
    /// without CNAME chain traversal.
    fn lookup_raw(&self, name: &str, rtype: &str) -> Option<&[DnsRecord]> {
        self.inner
            .get(&(Self::normalize_name(name), rtype.to_uppercase()))
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
    pub fn resolve_authoritative(
        &self,
        name: &str,
        rtype_str: &str,
        zone_apex: Option<&str>,
    ) -> IndexResolution {
        let mut current = Self::normalize_name(name);
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
                        current = Self::normalize_name(&cname.value);
                        continue; // Loop to collect records at the target
                    }
                    _ => {
                        // No CNAME, collect all records at the current name
                        let all_records: Vec<DnsRecord> = self
                            .inner
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
                        return if self.name_exists(&current, zone_apex) {
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
                    current = Self::normalize_name(&cname.value);
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
                    return if self.name_exists(&current, zone_apex) {
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

    /// Returns `true` if any record exists for `name` regardless of type,
    /// or if the name matches the configured authoritative zone apex.
    fn name_exists(&self, name: &str, zone_apex: Option<&str>) -> bool {
        let lower = Self::normalize_name(name);
        if Some(lower.as_str())
            == zone_apex
                .map(|s| s.trim_end_matches('.').to_lowercase())
                .as_deref()
        {
            return true;
        }
        self.owner_names.contains(&lower)
    }
}
