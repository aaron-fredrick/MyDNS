use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

use crate::cache::{CacheStats, DnsCache};
use crate::config::AppConfig;
use crate::dns::upstream::UpstreamResolver;
use crate::web::auth::LoginRateLimiter;

/// Central shared state threaded through all DNS and HTTP handlers via `Arc`.
pub struct AppState {
    /// SQLite connection pool.
    pub db: SqlitePool,
    /// In-memory TTL cache, shared between the DNS handler and admin API.
    pub cache: Arc<RwLock<DnsCache>>,
    /// Cache hit/miss counters.
    pub cache_stats: Arc<CacheStats>,
    /// Log broadcast channel. The DNS handler and tracing layer send here;
    /// WebSocket clients subscribe to it.
    pub log_tx: broadcast::Sender<String>,
    /// Server start time, used to compute uptime in the stats endpoint.
    pub start_time: Instant,
    /// Runtime-mutable application config (resolver priority, upstream IPs, etc.).
    pub config: Arc<RwLock<AppConfig>>,
    /// Upstream DNS resolver chain, rebuilt whenever settings change.
    pub upstream: Arc<RwLock<UpstreamResolver>>,
    /// Login rate limiter to prevent brute force attacks.
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    /// Signals all background tasks and servers to stop.
    #[allow(dead_code)]
    pub cancel: CancellationToken,
}

impl AppState {
    /// Constructs the shared state. Only called once in `main`.
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
            log_tx,
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(config)),
            upstream: Arc::new(RwLock::new(upstream)),
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            cancel,
        })
    }
}
