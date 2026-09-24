use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::config::{AppConfig, ResolverMode, ResolverPriority};
use crate::dns::blocklist::BlocklistIndex;
use crate::dns::record_index::RecordIndex;
use crate::dns::upstream::UpstreamResolver;
use crate::dns::zone_trie::ZoneTrie;
use crate::state::AppState;

pub struct TestContext {
    pub state: Arc<AppState>,
    pub _temp_dir: TempDir,
}

impl TestContext {
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create temporary DB directory");
        let path = temp_dir.path().join("test.db");
        let pool = crate::mydns::db::init(&path.to_string_lossy())
            .await
            .expect("failed to initialize test database");

        // Seed default admin user with password "admin"
        let hash = crate::mydns::web::auth::hash_password("admin").expect("hash password");
        crate::mydns::db::users::seed_admin(&pool, "admin", &hash)
            .await
            .expect("seed admin user");

        let config = AppConfig::from_toml_str(
            r#"
[auth]
admin_username = "admin"
admin_password = "admin-password"
jwt_secret = "test-jwt-secret-key-12345678901234567890"

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
allowed = ["example.com"]
"#,
        )
        .expect("valid test config");

        let upstream = UpstreamResolver::from_config(
            ResolverMode::Forwarding,
            ResolverPriority::RouterFirst,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5354),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5355)),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 53)],
        )
        .expect("upstream resolver");

        let timezone = config
            .timezone
            .parse::<chrono_tz::Tz>()
            .expect("valid test timezone");
        let observability_path = temp_dir.path().join("observability.db");
        let observability_db = crate::observability::database::ObservabilityDatabase::init(
            &observability_path.to_string_lossy(),
        )
        .await
        .expect("failed to initialize observability database");
        let telemetry_metrics =
            crate::observability::telemetry::metrics::MetricsAggregator::new(timezone);

        let (log_tx, _) = broadcast::channel(64);
        let cancel = CancellationToken::new();

        let state = AppState::new(
            pool,
            config,
            upstream,
            log_tx,
            cancel,
            RecordIndex::default(),
            ZoneTrie::from_zones(&["example.com".to_string()]),
            BlocklistIndex::from_domains(&["blocked.com".to_string()]),
            telemetry_metrics,
            Arc::new(observability_db),
        );

        Self {
            state,
            _temp_dir: temp_dir,
        }
    }
}
