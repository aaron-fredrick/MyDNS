use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

use crate::cache::{CacheStats, DnsCache};
use crate::config::AppConfig;
use crate::dns::record_index::RecordIndex;
use crate::dns::upstream::UpstreamResolver;
use crate::dns::zone_trie::ZoneTrie;
use crate::observability::Metrics;
use crate::web::auth::LoginRateLimiter;

/// Central shared state threaded through all DNS and HTTP handlers via `Arc`.
pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<RwLock<DnsCache>>,
    pub cache_stats: Arc<CacheStats>,
    /// Backend-owned operational telemetry shared by DNS and management surfaces.
    pub metrics: Arc<Metrics>,
    pub log_tx: broadcast::Sender<String>,
    pub start_time: Instant,
    pub config: Arc<RwLock<AppConfig>>,
    pub upstream: Arc<RwLock<UpstreamResolver>>,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    /// Label-inverted trie for O(depth) authoritative zone ownership lookup.
    pub zone_trie: Arc<RwLock<ZoneTrie>>,
    /// In-memory authoritative record index for zero-DB-hit hot-path resolution.
    pub record_index: Arc<RwLock<RecordIndex>>,
    #[allow(dead_code)]
    pub cancel: CancellationToken,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        config: AppConfig,
        upstream: UpstreamResolver,
        log_tx: broadcast::Sender<String>,
        cancel: CancellationToken,
        record_index: RecordIndex,
        zone_trie: ZoneTrie,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            cache: Arc::new(RwLock::new(DnsCache::new())),
            cache_stats: CacheStats::new(),
            metrics: Metrics::new(),
            log_tx,
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(config)),
            upstream: Arc::new(RwLock::new(upstream)),
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            zone_trie: Arc::new(RwLock::new(zone_trie)),
            record_index: Arc::new(RwLock::new(record_index)),
            cancel,
        })
    }
}
