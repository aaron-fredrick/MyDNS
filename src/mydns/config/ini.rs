use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::str::FromStr;

use super::types::{AppConfig, ResolverMode, ResolverPriority};

impl AppConfig {
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
            allowed_zones: {
                let mut zones = values
                    .get("allowed_zones")
                    .map(|v| {
                        v.split(',')
                            .map(str::trim)
                            .filter(|z| !z.is_empty())
                            .map(|z| z.trim_end_matches('.').to_lowercase())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if zones.is_empty() {
                    zones.push("home.arpa".to_string());
                }
                zones
            },
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

pub(crate) fn parse_ini(
    contents: &str,
) -> anyhow::Result<std::collections::HashMap<String, String>> {
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
