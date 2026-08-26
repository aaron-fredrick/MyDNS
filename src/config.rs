use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
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

/// Runtime configuration loaded from `config.ini`.
///
/// `.env` is intentionally limited to debug builds and is loaded by `main` only
/// as a developer convenience. Production configuration comes from `config.ini`.
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
    /// Path to the SQLite database file.
    pub db_path: String,
    /// HMAC secret used to sign/verify JWTs.
    pub jwt_secret: String,
    /// Dashboard admin username (required in config.ini).
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
    /// Loads configuration from `config.ini`, applying safe defaults for optional values.
    ///
    /// Admin credentials are deliberately mandatory and are never defaulted. This
    /// prevents a production deployment from silently starting with known credentials.
    pub fn from_config_file() -> anyhow::Result<Self> {
        let path = Path::new("config.ini");
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
        let values = parse_ini(&contents)?;

        let admin_username = required(&values, "admin_username")?;
        let admin_password = required(&values, "admin_password")?;

        let cors_domains = values
            .get("cors_domains")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|domain| !domain.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|domains| !domains.is_empty())
            .unwrap_or_else(|| vec!["mydns.local".to_string()]);

        Ok(Self {
            bind_host: parse_value(&values, "bind_host", IpAddr::V4(Ipv4Addr::LOCALHOST))?,
            dns_port: parse_value(&values, "dns_port", 53)?,
            http_host: parse_value(&values, "http_host", IpAddr::V4(Ipv4Addr::LOCALHOST))?,
            http_port: parse_value(&values, "http_port", 8080)?,
            cors_domains,
            db_path: values
                .get("db_path")
                .cloned()
                .unwrap_or_else(|| "mydns.db".to_string()),
            // Empty means "not explicitly configured". main.rs then restores a
            // persisted secret or generates and persists one for first startup.
            jwt_secret: values.get("jwt_secret").cloned().unwrap_or_default(),
            admin_username,
            admin_password,
            resolver_priority: parse_value(
                &values,
                "resolver_priority",
                ResolverPriority::default(),
            )?,
            cloudflare_dns: parse_value(
                &values,
                "cloudflare_dns",
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53),
            )?,
            router_dns: values
                .get("router_dns")
                .map(|v| v.parse())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid router_dns: {}", e))?,
        })
    }
}

fn parse_ini(contents: &str) -> anyhow::Result<std::collections::HashMap<String, String>> {
    let mut values = std::collections::HashMap::new();

    for (line_number, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with('[')
        {
            continue;
        }

        let (key, value) = line.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid config.ini line {}: expected key=value",
                line_number + 1
            )
        })?;
        let key = key.trim().to_lowercase();
        let value = value.trim().trim_matches('"').to_string();

        if key.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid config.ini line {}: empty key",
                line_number + 1
            ));
        }
        values.insert(key, value);
    }

    Ok(values)
}

fn required(
    values: &std::collections::HashMap<String, String>,
    key: &str,
) -> anyhow::Result<String> {
    match values.get(key).map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(value) => Ok(value.to_string()),
        None => Err(anyhow::anyhow!(
            "Missing required config.ini value: {}",
            key
        )),
    }
}

fn parse_value<T: FromStr>(
    values: &std::collections::HashMap<String, String>,
    key: &str,
    default: T,
) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    match values.get(key) {
        Some(value) => value
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid {}: {}", key, e)),
        None => Ok(default),
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

pub fn generate_secret(len: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
