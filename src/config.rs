use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Controls which upstream DNS server is tried first on a cache/DB miss.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResolverPriority {
    /// Try Cloudflare (1.1.1.1) first, then the router gateway. (Default)
    #[default]
    CloudflareFirst,
    /// Try the router gateway first, then Cloudflare. Useful for ISP-specific domains.
    RouterFirst,
}

impl std::fmt::Display for ResolverPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CloudflareFirst => write!(f, "cloudflare_first"),
            Self::RouterFirst => write!(f, "router_first"),
        }
    }
}

impl FromStr for ResolverPriority {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "cloudflare_first" => Ok(Self::CloudflareFirst),
            "router_first" => Ok(Self::RouterFirst),
            other => Err(anyhow::anyhow!("Unknown resolver priority: '{}'", other)),
        }
    }
}

/// Runtime-configurable application settings.
///
/// Values are loaded from environment variables with sensible defaults.
/// Runtime-mutable fields (resolver priority, upstream IPs) live inside
/// [`crate::state::AppState`] behind an `Arc<RwLock<AppConfig>>`.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// UDP/TCP port the DNS server binds to. Requires elevated privileges for port 53.
    pub dns_port: u16,
    /// HTTP port for the management dashboard.
    pub http_port: u16,
    /// Path to the SQLite database file.
    pub db_path: String,
    /// HMAC secret used to sign/verify JWTs.
    pub jwt_secret: String,
    /// Dashboard admin username (seeded into DB on first run).
    pub admin_username: String,
    /// Dashboard admin plaintext password (hashed on first run; not stored in plain text).
    pub admin_password: String,
    /// Which upstream server to try first on a cache/DB miss.
    pub resolver_priority: ResolverPriority,
    /// Cloudflare Public DNS address.
    pub cloudflare_dns: SocketAddr,
    /// Router/gateway DNS address (auto-detected on startup, can be overridden).
    pub router_dns: Option<SocketAddr>,
}

impl AppConfig {
    #[allow(non_snake_case)]
    /// Builds configuration from environment variables, falling back to sensible defaults.
    pub fn fromEnv() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| generateSecret(64));

        Self {
            dns_port: envParse("DNS_PORT", 53),
            http_port: envParse("HTTP_PORT", 8080),
            db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "mydns.db".to_string()),
            jwt_secret,
            admin_username: std::env::var("ADMIN_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            admin_password: std::env::var("ADMIN_PASSWORD")
                .unwrap_or_else(|_| "changeme123".to_string()),
            resolver_priority: std::env::var("RESOLVER_PRIORITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            cloudflare_dns: std::env::var("CLOUDFLARE_DNS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53)),
            router_dns: std::env::var("ROUTER_DNS")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
fn envParse<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[allow(non_snake_case)]
pub fn generateSecret(len: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
