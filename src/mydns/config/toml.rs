use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use serde::Deserialize;

use super::types::{AppConfig, ResolverMode, ResolverPriority};

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlSystemSection {
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlConfigFile {
    system: Option<TomlSystemSection>,
    server: Option<TomlServerSection>,
    database: Option<TomlDatabaseSection>,
    auth: Option<TomlAuthSection>,
    resolver: Option<TomlResolverSection>,
    zones: Option<TomlZonesSection>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlServerSection {
    bind_host: Option<IpAddr>,
    dns_port: Option<u16>,
    http_host: Option<IpAddr>,
    http_port: Option<u16>,
    run_as_user: Option<String>,
    run_as_group: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlDatabaseSection {
    path: Option<String>,
    observability_path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlAuthSection {
    admin_username: Option<String>,
    admin_password: Option<String>,
    jwt_secret: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlResolverSection {
    mode: Option<ResolverMode>,
    priority: Option<ResolverPriority>,
    cloudflare_dns: Option<SocketAddr>,
    router_dns: Option<SocketAddr>,
    /// Optional list of root hint addresses (e.g. ["198.41.0.4:53", ...]).
    /// When omitted the built-in IANA defaults are used.
    root_hints: Option<Vec<SocketAddr>>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct TomlZonesSection {
    authoritative: Option<Vec<String>>,
    allowed: Option<Vec<String>>,
    dashboard_domain: Option<String>,
    cors_domains: Option<Vec<String>>,
}

impl AppConfig {
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

        let system = parsed.system.unwrap_or_default();
        let timezone = system.timezone.unwrap_or_else(|| "UTC".to_string());
        timezone.parse::<chrono_tz::Tz>().map_err(|e| {
            anyhow::anyhow!("Invalid system.timezone '{}': {}", timezone, e)
        })?;
        let server = parsed.server.unwrap_or_default();
        let database = parsed.database.unwrap_or_default();
        let resolver = parsed.resolver.unwrap_or_default();
        let zones = parsed.zones.unwrap_or_default();

        let mut allowed_zones: Vec<String> = zones
            .authoritative
            .or(zones.allowed)
            .unwrap_or_default()
            .into_iter()
            .map(|z| z.trim().trim_end_matches('.').to_lowercase())
            .filter(|z| !z.is_empty())
            .collect();

        if allowed_zones.is_empty() {
            allowed_zones.push("home.arpa".to_string());
        }

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
            observability_db_path: database.observability_path.unwrap_or_else(|| "observability.db".to_string()),
            timezone,
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
}
