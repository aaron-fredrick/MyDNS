use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hickory_proto::rr::{Record, RecordType};

/// Key into the DNS cache: normalised lowercase domain name + record type.
pub type CacheKey = (String, RecordType);

/// The result represented by a cache entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheResult {
    /// A successful DNS response containing records.
    Positive,
    /// A negative DNS response (NXDOMAIN/NODATA) with no answer records.
    Negative,
}

/// A single cached DNS response.
pub struct CacheEntry {
    /// The answer records to return. Empty when `result` is `Negative`.
    pub records: Vec<Record>,
    /// Whether this entry represents a negative DNS result.
    pub result: CacheResult,
    /// Absolute point-in-time after which this entry is considered stale.
    pub expires_at: Instant,
}

#[allow(non_snake_case)]
impl CacheEntry {
    pub fn isExpired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Thread-safe TTL-aware in-memory DNS cache.
///
/// Wrapped in `Arc<tokio::sync::RwLock<DnsCache>>` inside [`AppState`].
pub struct DnsCache {
    inner: HashMap<CacheKey, CacheEntry>,
}

#[allow(non_snake_case)]
impl DnsCache {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Returns the cached result for the key if present and not expired.
    pub fn get(&self, name: &str, rtype: RecordType) -> Option<(CacheResult, &Vec<Record>)> {
        let key = (name.to_lowercase(), rtype);
        self.inner
            .get(&key)
            .filter(|e| !e.isExpired())
            .map(|e| (e.result, &e.records))
    }

    /// Inserts a positive cache entry with the given TTL.
    pub fn insert(&mut self, name: &str, rtype: RecordType, records: Vec<Record>, ttl: Duration) {
        self.insertResult(name, rtype, CacheResult::Positive, records, ttl);
    }

    /// Inserts a negative cache entry with the given TTL.
    pub fn insertNegative(&mut self, name: &str, rtype: RecordType, ttl: Duration) {
        self.insertResult(name, rtype, CacheResult::Negative, Vec::new(), ttl);
    }

    fn insertResult(
        &mut self,
        name: &str,
        rtype: RecordType,
        result: CacheResult,
        records: Vec<Record>,
        ttl: Duration,
    ) {
        let key = (name.to_lowercase(), rtype);

        // Simple cap to prevent memory bloat since we have DB persistence now
        if self.inner.len() >= 5000 {
            // Remove an arbitrary entry (HashMap doesn't have order, but this is fine)
            if let Some(k) = self.inner.keys().next().cloned() {
                self.inner.remove(&k);
            }
        }

        self.inner.insert(
            key,
            CacheEntry {
                records,
                result,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    /// Removes a specific entry (used when an admin modifies a record).
    pub fn remove(&mut self, name: &str, rtype: RecordType) {
        let key = (name.to_lowercase(), rtype);
        self.inner.remove(&key);
    }

    /// Removes all entries for a DNS name.
    pub fn removeName(&mut self, name: &str) {
        let name = name.to_lowercase();
        self.inner
            .retain(|(cached_name, _), _| cached_name != &name);
    }

    /// Removes all entries that have passed their expiry time.
    pub fn prune(&mut self) -> usize {
        let before = self.inner.len();
        self.inner.retain(|_, entry| !entry.isExpired());
        before - self.inner.len()
    }

    /// Total number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears the entire cache.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns a list of all non-expired cache entries for the UI.
    ///
    /// Returns: Vec<(Name, RecordType, TTL_Remaining_Secs, Values)>
    pub fn listAll(&self) -> Vec<(String, RecordType, u32, Vec<String>)> {
        let now = Instant::now();
        self.inner
            .iter()
            .filter(|(_, entry)| !entry.isExpired())
            .map(|(key, entry)| {
                let ttl_remaining = entry
                    .expires_at
                    .checked_duration_since(now)
                    .unwrap_or_default()
                    .as_secs() as u32;

                let values = entry
                    .records
                    .iter()
                    .map(|r| r.data.to_string())
                    .collect();

                (key.0.clone(), key.1, ttl_remaining, values)
            })
            .collect()
    }
}

impl Default for DnsCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomically tracked cache statistics.
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
}

#[allow(non_snake_case)]
impl CacheStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    pub fn recordHit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn recordMiss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }
}

/// Spawns a background task that prunes expired cache entries every 60 seconds.
#[allow(non_snake_case)]
pub fn spawnPruner(
    cache: Arc<tokio::sync::RwLock<DnsCache>>,
    db: sqlx::SqlitePool,
    cancel: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    let pruned_mem = cache.write().await.prune();

                    let pruned_db = match crate::db::records::pruneCache(&db).await {
                        Ok(n) => n,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to prune DB cache");
                            0
                        }
                    };

                    if pruned_mem > 0 || pruned_db > 0 {
                        tracing::debug!(
                            mem = pruned_mem,
                            db = pruned_db,
                            "Pruned expired cache entries"
                        );
                    }
                }
                _ = cancel.cancelled() => break,
            }
        }
    });
}

#[cfg(test)]
mod tests;
