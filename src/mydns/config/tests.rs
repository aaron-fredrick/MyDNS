use super::*;
use crate::config::ini::parse_ini;
use std::net::{IpAddr, SocketAddr};

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
    assert_eq!(cfg.allowed_zones.len(), 1);
    assert_eq!(cfg.allowed_zones[0], "home.arpa");
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
fn resolver_mode_parses_aliases_and_rejects_unknown_values() {
    assert_eq!(
        "forward".parse::<ResolverMode>().unwrap(),
        ResolverMode::Forwarding
    );
    assert_eq!(
        " FORWARDING ".parse::<ResolverMode>().unwrap(),
        ResolverMode::Forwarding
    );
    assert_eq!(
        "recurse".parse::<ResolverMode>().unwrap(),
        ResolverMode::Recursive
    );
    assert!("authoritative".parse::<ResolverMode>().is_err());
}

#[test]
fn resolver_priority_requires_supported_spelling() {
    assert_eq!(
        "cloudflare_first".parse::<ResolverPriority>().unwrap(),
        ResolverPriority::CloudflareFirst
    );
    assert_eq!(
        "router_first".parse::<ResolverPriority>().unwrap(),
        ResolverPriority::RouterFirst
    );
    assert!("Cloudflare_First".parse::<ResolverPriority>().is_err());
}

#[test]
fn resolver_enums_display_as_config_values() {
    assert_eq!(ResolverMode::Forwarding.to_string(), "forwarding");
    assert_eq!(ResolverMode::Recursive.to_string(), "recursive");
    assert_eq!(
        ResolverPriority::CloudflareFirst.to_string(),
        "cloudflare_first"
    );
    assert_eq!(ResolverPriority::RouterFirst.to_string(), "router_first");
}

#[test]
fn toml_normalizes_and_defaults_allowed_zones() {
    let cfg = AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "admin"
admin_password = "password"

[zones]
authoritative = ["Example.COM.", "  ", "LAB.local..."]
"#,
    )
    .unwrap();

    assert_eq!(cfg.allowed_zones, vec!["example.com", "lab.local"]);
}

#[test]
fn toml_rejects_blank_credentials() {
    assert!(AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "   "
admin_password = "password"
"#
    )
    .is_err());

    assert!(AppConfig::from_toml_str(
        r#"
[auth]
admin_username = "admin"
admin_password = "   "
"#
    )
    .is_err());
}

#[test]
fn ini_parser_ignores_comments_sections_and_normalizes_keys() {
    let values = parse_ini(
        r#"
# comment
; another comment
[server]
BIND_HOST = "127.0.0.1"
dns_port = 5353

"#,
    )
    .unwrap();

    assert_eq!(values.get("bind_host").unwrap(), "127.0.0.1");
    assert_eq!(values.get("dns_port").unwrap(), "5353");
}

#[test]
fn ini_parser_rejects_malformed_lines_and_empty_keys() {
    assert!(parse_ini("not-a-pair").is_err());
    assert!(parse_ini(" = value").is_err());
}

#[test]
fn generate_secret_returns_requested_length() {
    assert_eq!(generate_secret(0).len(), 0);
    assert_eq!(generate_secret(32).len(), 32);
}

#[test]
fn test_parse_toml_missing_credentials_fails() {
    let toml_str = r#"
[server]
dns_port = 53
"#;
    assert!(AppConfig::from_toml_str(toml_str).is_err());
}
