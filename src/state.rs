use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

use crate::cache::{CacheStats, DnsCache};
use crate::config::AppConfig;
use crate::dns::upstream::UpstreamResolver;
use crate::metrics::Metrics;
use crate::web::auth::LoginRateLimiter;

/// Central shared state threaded through all DNS and HTTP handlers via `Arc`.
pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<RwLock<DnsCache>>,
    pub cache_stats: Arc<CacheStats>,
    pub metrics: Arc<Metrics>,
    pub log_tx: broadcast::Sender<String>,
    pub start_time: Instant,
    pub config: Arc<RwLock<AppConfig>>,
    pub upstream: Arc<RwLock<UpstreamResolver>>,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
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
            cancel,
        })
    }
}
