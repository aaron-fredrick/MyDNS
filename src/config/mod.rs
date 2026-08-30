use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
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

#[derive(Debug, Deserialize, Default)]
struct TomlConfigFile {
    server: Option<TomlServerSection>,
    database: Option<TomlDatabaseSection>,
    auth: Option<TomlAuthSection>,
    resolver: Option<TomlResolverSection>,
    zones: Option<TomlZonesSection>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlServerSection {
    bind_host: Option<IpAddr>,
    dns_port: Option<u16>,
    http_host: Option<IpAddr>,
    http_port: Option<u16>,
    run_as_user: Option<String>,
    run_as_group: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlDatabaseSection {
    path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlAuthSection {
    admin_username: Option<String>,
    admin_password: Option<String>,
    jwt_secret: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlResolverSection {
    mode: Option<ResolverMode>,
    priority: Option<ResolverPriority>,
    cloudflare_dns: Option<SocketAddr>,
    router_dns: Option<SocketAddr>,
    /// Optional list of root hint addresses (e.g. ["198.41.0.4:53", ...]).
    /// When omitted the built-in IANA defaults are used.
    root_hints: Option<Vec<SocketAddr>>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlZonesSection {
    authoritative: Option<Vec<String>>,
    allowed: Option<Vec<String>>,
    dashboard_domain: Option<String>,
    cors_domains: Option<Vec<String>>,
}

impl AppConfig {
    /// Loads configuration from `config.toml`, falling back to `config.ini` if absent.
    pub fn from_config_file() -> anyhow::Result<Self> {
        if Path::new("config.toml").exists() {
            Self::from_toml_file(Path::new("config.toml"))
        } else if Path::new("config.ini").exists() {
            Self::from_ini_file(Path::new("config.ini"))
        } else {
            Err(anyhow::anyhow!(
                "Configuration file not found. Please create config.toml (see config.toml.example)"
            ))
        }
    }

    /// Loads configuration from a TOML file.
    pub fn from_toml_file(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
        Self::from_toml_str(&contents)
    }

    /// Parses configuration from a TOML string.
    pub fn from_toml_str(contents: &str) -> anyhow::Result<Self> {
        let parsed: TomlConfigFile = toml::from_str(contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {}", e))?;

        let auth = parsed.auth.unwrap_or_default();
        let admin_username = auth
            .admin_username
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!("Missing required config.toml field: [auth].admin_username")
            })?;
        let admin_password = auth
            .admin_password
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!("Missing required config.toml field: [auth].admin_password")
            })?;

        let server = parsed.server.unwrap_or_default();
        let database = parsed.database.unwrap_or_default();
        let resolver = parsed.resolver.unwrap_or_default();
        let zones = parsed.zones.unwrap_or_default();

        let allowed_zones = zones
            .authoritative
            .or(zones.allowed)
            .unwrap_or_default()
            .into_iter()
            .map(|z| z.trim_end_matches('.').to_lowercase())
            .filter(|z| !z.is_empty())
            .collect();

        let cors_domains = zones
            .cors_domains
            .unwrap_or_else(|| vec!["mydns.local".to_string()]);

        Ok(Self {
            bind_host: server.bind_host.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            dns_port: server.dns_port.unwrap_or(53),
            http_host: server.http_host.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            http_port: server.http_port.unwrap_or(8080),
            cors_domains,
            dashboard_domain: zones
                .dashboard_domain
                .unwrap_or_else(|| "mydns.local".to_string()),
            db_path: database.path.unwrap_or_else(|| "mydns.db".to_string()),
            jwt_secret: auth.jwt_secret.unwrap_or_default(),
            admin_username,
            admin_password,
            resolver_mode: resolver.mode.unwrap_or_default(),
            resolver_priority: resolver.priority.unwrap_or_default(),
            cloudflare_dns: resolver
                .cloudflare_dns
                .unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53)),
            router_dns: resolver.router_dns,
            root_hints: resolver.root_hints.unwrap_or_default(),
            run_as_user: server.run_as_user.unwrap_or_else(|| "nobody".to_string()),
            run_as_group: server.run_as_group.unwrap_or_else(|| "nobody".to_string()),
            allowed_zones,
        })
    }

    /// Legacy INI configuration parser for backwards compatibility.
    pub fn from_ini_file(path: &Path) -> anyhow::Result<Self> {
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
            dashboard_domain: values
                .get("dashboard_domain")
                .cloned()
                .unwrap_or_else(|| "mydns.local".to_string()),
            db_path: values
                .get("db_path")
                .cloned()
                .unwrap_or_else(|| "mydns.db".to_string()),
            jwt_secret: values.get("jwt_secret").cloned().unwrap_or_default(),
            admin_username,
            admin_password,
            resolver_mode: parse_value(&values, "resolver_mode", ResolverMode::default())?,
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
            run_as_user: values
                .get("run_as_user")
                .cloned()
                .unwrap_or_else(|| "nobody".to_string()),
            run_as_group: values
                .get("run_as_group")
                .cloned()
                .unwrap_or_else(|| "nobody".to_string()),
            allowed_zones: values
                .get("allowed_zones")
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|z| !z.is_empty())
                        .map(|z| z.trim_end_matches('.').to_lowercase())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            root_hints: values
                .get("root_hints")
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.parse::<SocketAddr>().ok())
                        .collect()
                })
                .unwrap_or_default(),
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
        None => Err(anyhow::anyhow!("Missing required config value: {}", key)),
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

pub fn generate_secret(len: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml_full() {
        let toml_str = r#"
[server]
bind_host = "0.0.0.0"
dns_port = 5353
http_host = "127.0.0.1"
http_port = 9090
run_as_user = "dnsuser"
run_as_group = "dnsgroup"

[database]
path = "custom.db"

[auth]
admin_username = "superuser"
admin_password = "secretpassword"
jwt_secret = "customjwtsecret"

[resolver]
mode = "recursive"
priority = "router_first"
cloudflare_dns = "1.0.0.1:53"
router_dns = "192.168.1.1:53"
root_hints = ["198.41.0.4:53", "199.9.14.201:53"]

[zones]
authoritative = ["home.local", "lab.local"]
dashboard_domain = "dashboard.local"
cors_domains = ["dashboard.local", "app.local"]
"#;

        let cfg = AppConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(cfg.bind_host, "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.dns_port, 5353);
        assert_eq!(cfg.http_host, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.http_port, 9090);
        assert_eq!(cfg.run_as_user, "dnsuser");
        assert_eq!(cfg.run_as_group, "dnsgroup");
        assert_eq!(cfg.db_path, "custom.db");
        assert_eq!(cfg.admin_username, "superuser");
        assert_eq!(cfg.admin_password, "secretpassword");
        assert_eq!(cfg.jwt_secret, "customjwtsecret");
        assert_eq!(cfg.resolver_mode, ResolverMode::Recursive);
        assert_eq!(cfg.resolver_priority, ResolverPriority::RouterFirst);
        assert_eq!(cfg.cloudflare_dns, "1.0.0.1:53".parse().unwrap());
        assert_eq!(cfg.router_dns, Some("192.168.1.1:53".parse().unwrap()));
        assert_eq!(cfg.root_hints.len(), 2);
        assert_eq!(
            cfg.root_hints[0],
            "198.41.0.4:53".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(cfg.allowed_zones, vec!["home.local", "lab.local"]);
        assert_eq!(cfg.dashboard_domain, "dashboard.local");
        assert_eq!(cfg.cors_domains, vec!["dashboard.local", "app.local"]);
    }

    #[test]
    fn test_parse_toml_minimal_defaults() {
        let toml_str = r#"
[auth]
admin_username = "admin"
admin_password = "password"
"#;

        let cfg = AppConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(cfg.bind_host, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.dns_port, 53);
        assert_eq!(cfg.http_port, 8080);
        assert_eq!(cfg.db_path, "mydns.db");
        assert_eq!(cfg.resolver_mode, ResolverMode::Forwarding);
        assert_eq!(cfg.resolver_priority, ResolverPriority::CloudflareFirst);
        assert_eq!(cfg.allowed_zones.len(), 0);
        // When not set, root_hints should be empty (resolved to IANA defaults at runtime).
        assert_eq!(cfg.root_hints.len(), 0);
    }

    #[test]
    fn test_default_root_hints_has_13_entries() {
        let hints = default_root_hints();
        assert_eq!(hints.len(), 13);
        // All should use port 53.
        assert!(hints.iter().all(|a| a.port() == 53));
        // The well-known A root server IP.
        assert!(hints.iter().any(|a| a.ip().to_string() == "198.41.0.4"));
    }

    #[test]
    fn test_parse_toml_missing_credentials_fails() {
        let toml_str = r#"
[server]
dns_port = 53
"#;
        assert!(AppConfig::from_toml_str(toml_str).is_err());
    }
}
