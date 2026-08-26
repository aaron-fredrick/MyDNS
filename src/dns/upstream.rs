#[allow(unused_imports)]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use hickory_proto::rr::{Name, Record, RecordType};
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

use crate::config::ResolverPriority;

#[derive(Debug)]
pub enum UpstreamResolution {
    Positive(Vec<Record>, u32),
    Nodata,
    NxDomain,
    ServFail,
}

/// Attempts to detect the default gateway/router IP from the OS routing table.
///
/// Returns the gateway IP as port-53 `SocketAddr`, or `None` if detection fails.
#[allow(non_snake_case)]
pub fn detectGateway() -> Option<SocketAddr> {
    detectGatewayImpl().map(|ip| SocketAddr::new(ip, 53))
}

#[allow(non_snake_case)]
#[cfg(windows)]
fn detectGatewayImpl() -> Option<IpAddr> {
    let output = std::process::Command::new("ipconfig").output().ok()?;
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

#[allow(non_snake_case)]
#[cfg(unix)]
fn detectGatewayImpl() -> Option<IpAddr> {
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

#[allow(non_snake_case)]
#[cfg(not(any(windows, unix)))]
fn detectGatewayImpl() -> Option<IpAddr> {
    None
}

#[allow(non_snake_case)]
fn buildResolver(addr: SocketAddr) -> TokioAsyncResolver {
    let group = NameServerConfigGroup::from_ips_clear(&[addr.ip()], addr.port(), true);
    let config = ResolverConfig::from_parts(None, vec![], group);
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(3);
    opts.attempts = 2;
    TokioAsyncResolver::tokio(config, opts)
}

/// Forwards DNS queries to upstream servers in the configured priority order.
pub struct UpstreamResolver {
    cloudflare: TokioAsyncResolver,
    router: Option<TokioAsyncResolver>,
    pub priority: ResolverPriority,
    #[allow(dead_code)]
    pub cloudflare_addr: SocketAddr,
    pub router_addr: Option<SocketAddr>,
}

impl UpstreamResolver {
    #[allow(non_snake_case)]
    pub fn fromConfig(
        priority: ResolverPriority,
        cloudflare_addr: SocketAddr,
        router_addr: Option<SocketAddr>,
    ) -> anyhow::Result<Self> {
        let effective_router = router_addr.or_else(detectGateway);

        let cloudflare = buildResolver(cloudflare_addr);
        let router = effective_router.map(buildResolver);

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

    /// Queries upstream servers while preserving NXDOMAIN, NODATA and SERVFAIL.
    pub async fn resolve(&self, name: &Name, rtype: RecordType) -> UpstreamResolution {
        let (first, second) = self.orderedResolvers();

        match queryResolver(first, name, rtype).await {
            UpstreamResolution::Positive(records, ttl) => return UpstreamResolution::Positive(records, ttl),
            UpstreamResolution::NxDomain => return UpstreamResolution::NxDomain,
            UpstreamResolution::Nodata => {
                if let Some(resolver) = second {
                    match queryResolver(resolver, name, rtype).await {
                        UpstreamResolution::Positive(records, ttl) => return UpstreamResolution::Positive(records, ttl),
                        UpstreamResolution::NxDomain => return UpstreamResolution::NxDomain,
                        UpstreamResolution::Nodata => return UpstreamResolution::Nodata,
                        UpstreamResolution::ServFail => return UpstreamResolution::ServFail,
                    }
                }
                return UpstreamResolution::Nodata;
            }
            UpstreamResolution::ServFail => {
                if let Some(resolver) = second {
                    match queryResolver(resolver, name, rtype).await {
                        UpstreamResolution::Positive(records, ttl) => return UpstreamResolution::Positive(records, ttl),
                        UpstreamResolution::NxDomain => return UpstreamResolution::NxDomain,
                        UpstreamResolution::Nodata => return UpstreamResolution::Nodata,
                        UpstreamResolution::ServFail => return UpstreamResolution::ServFail,
                    }
                }
                return UpstreamResolution::ServFail;
            }
        }
    }

    #[allow(non_snake_case)]
    fn orderedResolvers(&self) -> (&TokioAsyncResolver, Option<&TokioAsyncResolver>) {
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

#[allow(non_snake_case)]
async fn queryResolver(
    resolver: &TokioAsyncResolver,
    name: &Name,
    rtype: RecordType,
) -> UpstreamResolution {
    match resolver.lookup(name.clone(), rtype).await {
        Ok(lookup) => {
            let records: Vec<Record> = lookup.records().to_vec();
            if records.is_empty() {
                return UpstreamResolution::Nodata;
            }
            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
            UpstreamResolution::Positive(records, ttl)
        }
        Err(e) if e.is_nx_domain() => {
            tracing::info!(query = %name, "Upstream returned NXDOMAIN");
            UpstreamResolution::NxDomain
        }
        Err(e) if e.is_no_records_found() => {
            tracing::info!(query = %name, "Upstream returned NODATA");
            UpstreamResolution::Nodata
        }
        Err(e) => {
            tracing::warn!(query = %name, error = %e, "Upstream lookup failed; returning SERVFAIL");
            UpstreamResolution::ServFail
        }
    }
}
