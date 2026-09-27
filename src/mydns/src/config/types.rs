use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The 13 IANA root server IPv4 addresses (A–M), each on port 53.
/// Used as default root hints when `resolver.root_hints` is not set in config.toml
/// and the resolver is operating in `recursive` mode.
///
/// Source: <https://www.iana.org/domains/root/servers>
pub const IANA_ROOT_HINTS: &[(&str, &str)] = &[
    ("a.root-servers.net", "198.41.0.4"),
    ("b.root-servers.net", "199.9.14.201"),
    ("c.root-servers.net", "192.33.4.12"),
    ("d.root-servers.net", "199.7.91.13"),
    ("e.root-servers.net", "192.203.230.10"),
    ("f.root-servers.net", "192.5.5.241"),
    ("g.root-servers.net", "192.112.36.4"),
    ("h.root-servers.net", "198.97.190.53"),
    ("i.root-servers.net", "192.36.148.17"),
    ("j.root-servers.net", "192.58.128.30"),
    ("k.root-servers.net", "193.0.14.129"),
    ("l.root-servers.net", "199.7.83.42"),
    ("m.root-servers.net", "202.12.27.33"),
];

/// Returns the 13 IANA root server addresses as `SocketAddr` (port 53).
/// These are the fallback used when `root_hints` is not specified in config.toml.
pub fn default_root_hints() -> Vec<SocketAddr> {
    IANA_ROOT_HINTS
        .iter()
        .filter_map(|(_, ip)| ip.parse::<IpAddr>().ok())
        .map(|ip| SocketAddr::new(ip, 53))
        .collect()
}

/// Mode of operation for non-authoritative DNS resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResolverMode {
    /// Forward queries to upstream DNS forwarders (Cloudflare, Router gateway, etc.). (Default)
    #[default]
    Forwarding,
    /// Full iterative recursive resolution starting from root servers.
    Recursive,
}

impl std::fmt::Display for ResolverMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forwarding => write!(f, "forwarding"),
            Self::Recursive => write!(f, "recursive"),
        }
    }
}

impl FromStr for ResolverMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "forwarding" | "forward" => Ok(Self::Forwarding),
            "recursive" | "recurse" => Ok(Self::Recursive),
            other => Err(anyhow::anyhow!("Unknown resolver mode: '{}'", other)),
        }
    }
}

/// Controls which upstream DNS server is tried first on a cache/DB miss in forwarding mode.
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

/// Runtime configuration loaded from `config.toml` (with fallback to `config.ini`).
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Address the DNS server binds to. Defaults to localhost.
    pub bind_host: IpAddr,
    /// UDP/TCP port the DNS server binds to. Requires elevated privileges for port 53.
    pub dns_port: u16,
    /// HTTP bind address. Defaults to localhost.
    pub http_host: IpAddr,
    /// HTTP port for the management dashboard.
    pub http_port: u16,
    /// Domains allowed as dashboard CORS origins. Defaults to `mydns.local`.
    pub cors_domains: Vec<String>,
    /// The special dashboard hostname that resolves to the MyDNS server itself. Defaults to `mydns.local`.
    pub dashboard_domain: String,
    /// Path to the SQLite database file.
    pub db_path: String,
    /// Path to the SQLite observability database file.
    pub observability_db_path: String,
    /// System timezone for period boundaries (e.g. "UTC" or "Asia/Colombo").
    pub timezone: String,
    /// HMAC secret used to sign/verify JWTs.
    pub jwt_secret: String,
    /// Dashboard admin username (required).
    pub admin_username: String,
    /// Dashboard admin plaintext password (hashed on first run; not stored in plain text).
    pub admin_password: String,
    /// Resolver mode: `forwarding` or `recursive`.
    pub resolver_mode: ResolverMode,
    /// Which upstream server to try first on a cache/DB miss in forwarding mode.
    pub resolver_priority: ResolverPriority,
    /// Cloudflare Public DNS address.
    pub cloudflare_dns: SocketAddr,
    /// Router/gateway DNS address (auto-detected on startup, can be overridden).
    pub router_dns: Option<SocketAddr>,
    /// Root hint server addresses used in recursive mode.
    /// Defaults to the 13 IANA root server IP addresses when empty.
    pub root_hints: Vec<SocketAddr>,
    /// Target Unix user to run as after binding privileged sockets.
    pub run_as_user: String,
    /// Target Unix group to run as after binding privileged sockets.
    pub run_as_group: String,
    /// Authoritative DNS zones managed locally (e.g. ["home.local", "mydns.local"]).
    /// If empty, queries check DB and fall through to resolver. If non-empty, queries matching
    /// these zones are strictly authoritative and never forwarded.
    pub allowed_zones: Vec<String>,
}
