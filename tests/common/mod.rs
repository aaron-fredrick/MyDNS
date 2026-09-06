#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mydns::config::{AppConfig, ResolverMode, ResolverPriority};
use mydns::db;
use mydns::dns;
use mydns::dns::record_index::RecordIndex;
use mydns::dns::zone_trie::ZoneTrie;
use mydns::state::AppState;
use mydns::web;
use mydns::web::auth::hash_password;
use reqwest::Client;
use serde_json::json;
use sqlx::SqlitePool;
use tempfile::TempDir;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

/// An isolated temporary database fixture with automatic cleanup on drop.
pub struct TestDb {
    pub temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestDb {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test DB");
        let path = temp_dir.path().join("test.db");
        Self { temp_dir, path }
    }

    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    pub async fn init_pool(&self) -> SqlitePool {
        db::init(&self.path_str())
            .await
            .expect("Failed to initialize test SQLite database")
    }
}

impl Default for TestDb {
    fn default() -> Self {
        Self::new()
    }
}

/// A running in-process test HTTP server.
pub struct TestServer {
    pub db: TestDb,
    pub base_url: String,
    pub port: u16,
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub admin_user: String,
    pub admin_pass: String,
}

impl TestServer {
    pub async fn start() -> Self {
        Self::start_with_config(|_| {}).await
    }

    pub async fn start_with_config<F>(configure: F) -> Self
    where
        F: FnOnce(&mut AppConfig),
    {
        let db = TestDb::new();
        let port = get_ephemeral_port().await;
        let dns_port = port + 1;

        let mut cfg = AppConfig {
            bind_host: "127.0.0.1".parse().unwrap(),
            dns_port,
            http_host: "127.0.0.1".parse().unwrap(),
            http_port: port,
            cors_domains: vec!["mydns.local".to_string()],
            dashboard_domain: "mydns.local".to_string(),
            db_path: db.path_str(),
            jwt_secret: mydns::config::generate_secret(64),
            admin_username: "admin".to_string(),
            admin_password: "changeme123".to_string(),
            resolver_mode: ResolverMode::Forwarding,
            resolver_priority: ResolverPriority::CloudflareFirst,
            cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
            router_dns: None,
            run_as_user: "nobody".to_string(),
            run_as_group: "nobody".to_string(),
            allowed_zones: vec![],
            root_hints: vec![],
        };
        configure(&mut cfg);

        let pool = db.init_pool().await;

        let hash = hash_password(&cfg.admin_password).expect("Failed to hash admin password");
        db::records::seed_admin(&pool, &cfg.admin_username, &hash)
            .await
            .expect("Failed to seed admin user");

        mydns::db::records::seed_zones(&pool, &cfg.allowed_zones)
            .await
            .expect("Failed to seed zones in TestServer");

        let upstream = dns::upstream::UpstreamResolver::from_config(
            cfg.resolver_mode.clone(),
            cfg.resolver_priority.clone(),
            cfg.cloudflare_dns,
            cfg.router_dns,
            cfg.root_hints.clone(),
        )
        .expect("Failed to create UpstreamResolver");

        let (log_tx, _) = tokio::sync::broadcast::channel(256);
        let cancel = CancellationToken::new();
        let zone_trie = ZoneTrie::from_zones(&cfg.allowed_zones);
        let record_index = RecordIndex::load_from_db(&pool)
            .await
            .expect("Failed to load RecordIndex");
        let state = AppState::new(
            pool.clone(),
            cfg.clone(),
            upstream,
            log_tx,
            cancel.clone(),
            record_index,
            zone_trie,
        );

        let server_state = Arc::clone(&state);
        let server_cancel = cancel.clone();

        tokio::spawn(async move {
            let _ = web::server::run(server_state, server_cancel).await;
        });

        // Give the listener time to bind
        tokio::time::sleep(Duration::from_millis(150)).await;

        Self {
            db,
            base_url: format!("http://127.0.0.1:{}", port),
            port,
            pool,
            cancel,
            admin_user: cfg.admin_username,
            admin_pass: cfg.admin_password,
        }
    }

    pub async fn login(&self, client: &Client) -> String {
        let response = client
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&json!({
                "username": self.admin_user,
                "password": self.admin_pass
            }))
            .send()
            .await
            .expect("Login request failed");

        assert_eq!(response.status(), 200);
        let json: serde_json::Value = response
            .json()
            .await
            .expect("Failed to parse JSON login response");
        json["token"]
            .as_str()
            .expect("Token missing in login response")
            .to_string()
    }

    pub async fn auth_header(&self, client: &Client) -> String {
        format!("Bearer {}", self.login(client).await)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// A running in-process test DNS server (UDP + TCP).
pub struct TestDnsServer {
    pub db: TestDb,
    pub addr: SocketAddr,
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub handle: tokio::task::JoinHandle<()>,
}

impl TestDnsServer {
    pub async fn start_with_records(
        allowed_zones: Vec<String>,
        records: &[(&str, &str, &str)],
    ) -> Self {
        let db = TestDb::new();
        let port = get_ephemeral_port().await;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let cfg = AppConfig {
            bind_host: "127.0.0.1".parse().unwrap(),
            dns_port: port,
            http_host: "127.0.0.1".parse().unwrap(),
            http_port: port + 1,
            cors_domains: vec!["mydns.local".to_string()],
            dashboard_domain: "mydns.local".to_string(),
            db_path: db.path_str(),
            jwt_secret: mydns::config::generate_secret(64),
            admin_username: "admin".to_string(),
            admin_password: "changeme123".to_string(),
            resolver_mode: ResolverMode::Forwarding,
            resolver_priority: ResolverPriority::CloudflareFirst,
            cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
            router_dns: None,
            run_as_user: "nobody".to_string(),
            run_as_group: "nobody".to_string(),
            allowed_zones,
            root_hints: vec![],
        };

        let pool = db.init_pool().await;

        mydns::db::records::seed_zones(&pool, &cfg.allowed_zones)
            .await
            .expect("Failed to seed zones in TestDnsServer");

        for r in records {
            db::records::create_record(
                &pool,
                &db::records::CreateRecord {
                    name: r.0.to_string(),
                    record_type: r.1.to_string(),
                    value: r.2.to_string(),
                    ttl: 60,
                    priority: if r.1 == "MX" { Some(10) } else { None },
                    is_dev: false,
                },
            )
            .await
            .expect("Failed to seed DNS record");
        }

        let upstream = dns::upstream::UpstreamResolver::from_config(
            cfg.resolver_mode.clone(),
            cfg.resolver_priority.clone(),
            cfg.cloudflare_dns,
            cfg.router_dns,
            cfg.root_hints.clone(),
        )
        .expect("Failed to create UpstreamResolver");

        let (log_tx, _) = tokio::sync::broadcast::channel(256);
        let cancel = CancellationToken::new();
        let zone_trie = ZoneTrie::from_zones(&cfg.allowed_zones);
        let record_index = RecordIndex::load_from_db(&pool)
            .await
            .expect("Failed to load RecordIndex");
        let state = AppState::new(
            pool.clone(),
            cfg,
            upstream,
            log_tx,
            cancel.clone(),
            record_index,
            zone_trie,
        );

        let server_state = Arc::clone(&state);
        let server_cancel = cancel.clone();

        let handle = tokio::spawn(async move {
            let _ = dns::server::run(server_state, server_cancel).await;
        });

        // Wait until TCP socket is ready to accept queries
        for _ in 0..50 {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        Self {
            db,
            addr,
            pool,
            cancel,
            handle,
        }
    }
    /// Creates a DNS server fixture with only zone entries — no additional records.
    ///
    /// Apex SOA and NS records are created automatically by the DB layer, so
    /// this helper is useful for testing bare-apex SOA/NS behaviour.
    pub async fn start_with_zones_only(allowed_zones: Vec<String>) -> Self {
        Self::start_with_records(allowed_zones, &[]).await
    }

    /// Simulates a server restart by cancelling the current server and starting
    /// a new one against the same database path.
    ///
    /// Records stored in `dns_records` (including apex SOA/NS) must survive
    /// this cycle unchanged. The original `TestDb` is moved into the new server
    /// so its temp directory is not deleted prematurely.
    pub async fn restart(&mut self) {
        // Stop the current server.
        self.cancel.cancel();

        // Wait for the old server task to exit by swapping out its handle.
        let dummy_handle = tokio::spawn(async {});
        let old_handle = std::mem::replace(&mut self.handle, dummy_handle);
        let _ = old_handle.await;

        let db_path = self.db.path_str();
        let port = get_ephemeral_port().await;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let cfg = AppConfig {
            bind_host: "127.0.0.1".parse().unwrap(),
            dns_port: port,
            http_host: "127.0.0.1".parse().unwrap(),
            http_port: port + 1,
            cors_domains: vec!["mydns.local".to_string()],
            dashboard_domain: "mydns.local".to_string(),
            db_path: db_path.clone(),
            jwt_secret: mydns::config::generate_secret(64),
            admin_username: "admin".to_string(),
            admin_password: "changeme123".to_string(),
            resolver_mode: ResolverMode::Forwarding,
            resolver_priority: ResolverPriority::CloudflareFirst,
            cloudflare_dns: "1.1.1.1:53".parse().unwrap(),
            router_dns: None,
            run_as_user: "nobody".to_string(),
            run_as_group: "nobody".to_string(),
            // No allowed_zones on restart — zones are loaded from DB.
            allowed_zones: vec![],
            root_hints: vec![],
        };

        // Re-open the same DB (pool must be a fresh connection to the same file).
        let pool = mydns::db::init(&db_path)
            .await
            .expect("Failed to reopen test database on restart");

        let upstream = dns::upstream::UpstreamResolver::from_config(
            cfg.resolver_mode.clone(),
            cfg.resolver_priority.clone(),
            cfg.cloudflare_dns,
            cfg.router_dns,
            cfg.root_hints.clone(),
        )
        .expect("Failed to create UpstreamResolver on restart");

        let (log_tx, _) = tokio::sync::broadcast::channel(256);
        let cancel = CancellationToken::new();
        let zone_trie = ZoneTrie::from_zones(
            &mydns::db::records::list_zone_names(&pool)
                .await
                .expect("Failed to load zone names on restart"),
        );
        let record_index = RecordIndex::load_from_db(&pool)
            .await
            .expect("Failed to load RecordIndex on restart");
        let state = AppState::new(
            pool.clone(),
            cfg,
            upstream,
            log_tx,
            cancel.clone(),
            record_index,
            zone_trie,
        );

        let server_state = Arc::clone(&state);
        let server_cancel = cancel.clone();

        let handle = tokio::spawn(async move {
            let _ = dns::server::run(server_state, server_cancel).await;
        });

        // Wait until TCP socket is ready.
        for _ in 0..50 {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        self.addr = addr;
        self.pool = pool;
        self.cancel = cancel;
        self.handle = handle;
    }
}

impl Drop for TestDnsServer {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Finds an available ephemeral port on 127.0.0.1.
pub async fn get_ephemeral_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral socket");
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    port
}
