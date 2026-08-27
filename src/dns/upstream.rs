use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

#[cfg(unix)]
use std::net::Ipv4Addr;

use hickory_proto::rr::{Name, Record, RecordType};
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

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
fn buildResolver(addr: SocketAddr) -> anyhow::Result<TokioResolver> {
    // Hickory 0.26 separates the server IP from connection ports. The old
    // udp_and_tcp(addr.ip()) form silently used port 53, which meant configured
    // non-standard upstream ports (including test/mock resolvers) were ignored.
    let mut udp = ConnectionConfig::udp();
    udp.port = addr.port();
    let mut tcp = ConnectionConfig::tcp();
    tcp.port = addr.port();

    let config = ResolverConfig::from_parts(
        None,
        vec![],
        vec![NameServerConfig::new(addr.ip(), true, vec![udp, tcp])],
    );
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(3);
    opts.attempts = 2;

    let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
    *builder.options_mut() = opts;
    Ok(builder.build()?)
}

/// Forwards DNS queries to upstream servers in the configured priority order.
pub struct UpstreamResolver {
    cloudflare: TokioResolver,
    router: Option<TokioResolver>,
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

        let cloudflare = buildResolver(cloudflare_addr)?;
        let router = effective_router.map(buildResolver).transpose()?;

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
            UpstreamResolution::Positive(records, ttl) => {
                UpstreamResolution::Positive(records, ttl)
            }
            UpstreamResolution::NxDomain => UpstreamResolution::NxDomain,
            UpstreamResolution::Nodata => {
                if let Some(resolver) = second {
                    match queryResolver(resolver, name, rtype).await {
                        UpstreamResolution::Positive(records, ttl) => {
                            return UpstreamResolution::Positive(records, ttl)
                        }
                        UpstreamResolution::NxDomain => return UpstreamResolution::NxDomain,
                        UpstreamResolution::Nodata => return UpstreamResolution::Nodata,
                        UpstreamResolution::ServFail => return UpstreamResolution::ServFail,
                    }
                }
                UpstreamResolution::Nodata
            }
            UpstreamResolution::ServFail => {
                if let Some(resolver) = second {
                    match queryResolver(resolver, name, rtype).await {
                        UpstreamResolution::Positive(records, ttl) => {
                            return UpstreamResolution::Positive(records, ttl)
                        }
                        UpstreamResolution::NxDomain => return UpstreamResolution::NxDomain,
                        UpstreamResolution::Nodata => return UpstreamResolution::Nodata,
                        UpstreamResolution::ServFail => return UpstreamResolution::ServFail,
                    }
                }
                UpstreamResolution::ServFail
            }
        }
    }

    #[allow(non_snake_case)]
    fn orderedResolvers(&self) -> (&TokioResolver, Option<&TokioResolver>) {
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
    resolver: &TokioResolver,
    name: &Name,
    rtype: RecordType,
) -> UpstreamResolution {
    match resolver.lookup(name.clone(), rtype).await {
        Ok(lookup) => {
            let records: Vec<Record> = lookup.answers().to_vec();
            if records.is_empty() {
                return UpstreamResolution::Nodata;
            }
            let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
            UpstreamResolution::Positive(records, ttl)
        }
        Err(e) => {
            if e.is_nx_domain() {
                tracing::info!(query = %name, "Upstream returned NXDOMAIN");
                UpstreamResolution::NxDomain
            } else if e.is_no_records_found() {
                tracing::info!(query = %name, "Upstream returned NODATA");
                UpstreamResolution::Nodata
            } else {
                tracing::warn!(query = %name, error = %e, "Upstream lookup failed; returning SERVFAIL");
                UpstreamResolution::ServFail
            }
        }
    }
}
