use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

use crate::cache::{CacheStats, DnsCache};
use crate::config::AppConfig;
use crate::dns::blocklist::BlocklistIndex;
use crate::dns::record_index::RecordIndex;
use crate::dns::upstream::UpstreamResolver;
use crate::dns::zone_trie::ZoneTrie;
use crate::observability::database::ObservabilityDatabase;
use crate::observability::Metrics;
use crate::web::auth::LoginRateLimiter;

pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<RwLock<DnsCache>>,
    pub cache_stats: Arc<CacheStats>,
    /// Backend-owned operational telemetry shared by DNS and management surfaces.
    pub metrics: Arc<Metrics>,
    /// Shared SQLite store for observability data.
    pub observability_db: Arc<ObservabilityDatabase>,
    pub log_tx: broadcast::Sender<String>,
    pub start_time: Instant,
    pub config: Arc<RwLock<AppConfig>>,
    pub upstream: Arc<RwLock<UpstreamResolver>>,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    pub zone_trie: Arc<RwLock<ZoneTrie>>,
    pub record_index: Arc<RwLock<RecordIndex>>,
    pub blocklist_index: Arc<RwLock<BlocklistIndex>>,
    #[allow(dead_code)]
    pub cancel: CancellationToken,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: SqlitePool,
        config: AppConfig,
        upstream: UpstreamResolver,
        log_tx: broadcast::Sender<String>,
        cancel: CancellationToken,
        record_index: RecordIndex,
        zone_trie: ZoneTrie,
        blocklist_index: BlocklistIndex,
        observability_db: Arc<ObservabilityDatabase>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            cache: Arc::new(RwLock::new(DnsCache::new())),
            cache_stats: CacheStats::new(),
            metrics: Metrics::new(),
            observability_db,
            log_tx,
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(config)),
            upstream: Arc::new(RwLock::new(upstream)),
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            zone_trie: Arc::new(RwLock::new(zone_trie)),
            record_index: Arc::new(RwLock::new(record_index)),
            blocklist_index: Arc::new(RwLock::new(blocklist_index)),
            cancel,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResolverMode, ResolverPriority};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tempfile::NamedTempFile;

    fn test_config() -> AppConfig {
        AppConfig::from_toml_str(
            r#"
[system]
timezone = "Asia/Colombo"

[auth]
admin_username = "admin"
admin_password = "test-password"
jwt_secret = "test-secret"

[server]
dns_port = 5353
http_port = 8080

[resolver]
mode = "forwarding"
priority = "router_first"
cloudflare_dns = "127.0.0.1:5354"
router_dns = "127.0.0.1:5355"
root_hints = ["192.0.2.1:53"]

[zones]
allowed = ["home.arpa"]
"#,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn new_initializes_all_state_and_preserves_inputs() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let observability_file = NamedTempFile::new().unwrap();
        let observability_db = Arc::new(
            ObservabilityDatabase::init(observability_file.path().to_str().unwrap())
                .await
                .unwrap(),
        );
        let (log_tx, mut log_rx) = broadcast::channel(4);
        let cancel = CancellationToken::new();
        let upstream = UpstreamResolver::from_config(
            ResolverMode::Forwarding,
            ResolverPriority::RouterFirst,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5354),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5355)),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 53)],
        )
        .unwrap();

        let state = AppState::new(
            db.clone(),
            test_config(),
            upstream,
            log_tx.clone(),
            cancel.clone(),
            RecordIndex::default(),
            ZoneTrie::from_zones(&["home.arpa".to_string()]),
            BlocklistIndex::from_domains(&["blocked.example".to_string()]),
            observability_db,
        );

        assert_eq!(state.db.size(), db.size());
        assert!(state.cache.read().await.is_empty());
        assert_eq!(state.cache_stats.snapshot(), (0, 0));
        assert_eq!(state.metrics.snapshot().queries_total, 0);
        assert_eq!(state.metrics.snapshot().queries_blocked, 0);
        assert!(state.start_time.elapsed().as_secs() < 1);

        let config = state.config.read().await;
        assert_eq!(config.admin_username, "admin");
        assert_eq!(config.resolver_priority, ResolverPriority::RouterFirst);
        assert_eq!(config.timezone, "Asia/Colombo");
        drop(config);

        let upstream = state.upstream.read().await;
        assert_eq!(upstream.mode, ResolverMode::Forwarding);
        assert_eq!(upstream.priority, ResolverPriority::RouterFirst);
        assert_eq!(upstream.cloudflare_addr.port(), 5354);
        assert_eq!(upstream.router_addr.unwrap().port(), 5355);
        drop(upstream);

        let zone_trie = state.zone_trie.read().await;
        assert!(!zone_trie.is_root_authoritative());
        assert_eq!(zone_trie.find_zone("host.home.arpa"), Some("home.arpa"));
        drop(zone_trie);

        assert!(state
            .blocklist_index
            .read()
            .await
            .is_blocked("blocked.example"));
        assert!(!state
            .blocklist_index
            .read()
            .await
            .is_blocked("allowed.example"));
        assert!(!state.cancel.is_cancelled());

        log_tx.send("state-test".to_string()).unwrap();
        assert_eq!(log_rx.recv().await.unwrap(), "state-test");
        assert!(state
            .login_rate_limiter
            .check_rate_limit("192.0.2.1".parse().unwrap())
            .await
            .is_ok());
    }
}
