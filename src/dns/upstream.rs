#[allow(unused_imports)]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;


use hickory_proto::rr::{Name, RecordType, Record};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

use crate::config::ResolverPriority;

/// Attempts to detect the default gateway/router IP from the OS routing table.
///
/// Returns the gateway IP as port-53 `SocketAddr`, or `None` if detection fails.
pub fn detect_gateway() -> Option<SocketAddr> {
    detect_gateway_impl().map(|ip| SocketAddr::new(ip, 53))
}

#[cfg(windows)]
fn detect_gateway_impl() -> Option<IpAddr> {
    let output = std::process::Command::new("ipconfig")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains("Default Gateway") {
            if let Some(raw) = line.split(':').nth(1) {
                if let Ok(ip) = raw.trim().parse::<IpAddr>() {
                    if !ip.is_unspecified() {
                        return Some(ip);
                    }
                }
            }
        }
    }
    None
}

#[cfg(unix)]
fn detect_gateway_impl() -> Option<IpAddr> {
    let content = std::fs::read_to_string("/proc/net/route").ok()?;
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 3 && fields[1] == "00000000" {
            let gw_hex = fields[2];
            let bytes = u32::from_str_radix(gw_hex, 16).ok()?;
            let octets = bytes.to_le_bytes();
            return Some(IpAddr::V4(Ipv4Addr::from(octets)));
        }
    }
    None
}

#[cfg(not(any(windows, unix)))]
fn detect_gateway_impl() -> Option<IpAddr> {
    None
}

// ── resolver construction ─────────────────────────────────────────────────────

fn build_resolver(addr: SocketAddr) -> TokioAsyncResolver {
    let group = NameServerConfigGroup::from_ips_clear(&[addr.ip()], addr.port(), true);
    let config = ResolverConfig::from_parts(None, vec![], group);
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(3);
    opts.attempts = 2;
    TokioAsyncResolver::tokio(config, opts)
}

// ── public struct ─────────────────────────────────────────────────────────────

/// Forwards DNS queries to upstream servers in the configured priority order.
///
/// Resolution order depends on [`ResolverPriority`]:
/// - `CloudflareFirst` → tries `1.1.1.1`, then the router gateway.
/// - `RouterFirst`     → tries the router gateway, then `1.1.1.1`.
///
/// Successful responses are returned to the caller; caching at the correct TTL
/// is done in [`crate::dns::handler`].
pub struct UpstreamResolver {
    cloudflare: TokioAsyncResolver,
    router: Option<TokioAsyncResolver>,
    pub priority: ResolverPriority,
    #[allow(dead_code)]
    pub cloudflare_addr: SocketAddr,
    pub router_addr: Option<SocketAddr>,
}

impl UpstreamResolver {
    /// Builds the resolver chain from config.  Gateway detection is attempted
    /// here if `router_addr` is `None` in the config.
    pub fn from_config(
        priority: ResolverPriority,
        cloudflare_addr: SocketAddr,
        router_addr: Option<SocketAddr>,
    ) -> anyhow::Result<Self> {
        let effective_router = router_addr.or_else(detect_gateway);

        let cloudflare = build_resolver(cloudflare_addr);
        let router = effective_router.map(build_resolver);

        if let Some(addr) = effective_router {
            tracing::info!(%addr, "Router/gateway DNS detected");
        } else {
            tracing::warn!("Could not detect gateway DNS; router fallback unavailable");
        }

        Ok(Self {
            cloudflare,
            router,
            priority,
            cloudflare_addr,
            router_addr: effective_router,
        })
    }

    /// Queries upstream servers in priority order. Returns the first successful
    /// response as a tuple of `(records, ttl_seconds)`, or `None` if all fail.
    pub async fn resolve(
        &self,
        name: &Name,
        rtype: RecordType,
    ) -> Option<(Vec<Record>, u32)> {
        let (first, second) = self.ordered_resolvers();

        if let Some(result) = query_resolver(first, name, rtype).await {
            return Some(result);
        }
        if let Some(resolver) = second {
            return query_resolver(resolver, name, rtype).await;
        }
        None
    }

    /// Returns (primary, secondary) resolver references in priority order.
    fn ordered_resolvers(&self) -> (&TokioAsyncResolver, Option<&TokioAsyncResolver>) {
        match self.priority {
            ResolverPriority::CloudflareFirst => (&self.cloudflare, self.router.as_ref()),
            ResolverPriority::RouterFirst => {
                if let Some(router) = &self.router {
                    (router, Some(&self.cloudflare))
                } else {
                    (&self.cloudflare, None)
                }
            }
        }
    }
}

/// Issues a lookup against a single resolver, returning `(records, min_ttl)`.
async fn query_resolver(
    resolver: &TokioAsyncResolver,
    name: &Name,
    rtype: RecordType,
) -> Option<(Vec<Record>, u32)> {
    match resolver.lookup(name.clone(), rtype).await {
        Ok(lookup) => {
            let records: Vec<Record> = lookup.records().to_vec();
            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
            Some((records, ttl))
        }
        Err(e) => {
            tracing::warn!(query = %name, error = %e, "Upstream lookup failed");
            None
        }
    }
}
