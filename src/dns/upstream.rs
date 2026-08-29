use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::net::Ipv4Addr;

use hickory_proto::rr::{Name, Record, RecordType};
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

use crate::config::{default_root_hints, ResolverMode, ResolverPriority};
use crate::observability::Metrics;

#[derive(Debug)]
pub enum UpstreamResolution {
    Positive(Vec<Record>, u32),
    Nodata,
    NxDomain,
    ServFail,
}

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
            let bytes = u32::from_str_radix(fields[2], 16).ok()?;
            return Some(IpAddr::V4(Ipv4Addr::from(bytes.to_le_bytes())));
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

#[allow(non_snake_case)]
fn buildRecursiveResolver(root_hints: &[SocketAddr]) -> anyhow::Result<TokioResolver> {
    let hints: Vec<SocketAddr> = if root_hints.is_empty() {
        default_root_hints()
    } else {
        root_hints.to_vec()
    };

    let name_servers: Vec<NameServerConfig> = hints
        .iter()
        .map(|addr| {
            let mut udp = ConnectionConfig::udp();
            udp.port = addr.port();
            let mut tcp = ConnectionConfig::tcp();
            tcp.port = addr.port();
            NameServerConfig::new(addr.ip(), true, vec![udp, tcp])
        })
        .collect();

    let config = ResolverConfig::from_parts(None, vec![], name_servers);
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(4);
    opts.attempts = 3;
    opts.recursion_desired = true;
    let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
    *builder.options_mut() = opts;
    Ok(builder.build()?)
}

pub struct UpstreamResolver {
    cloudflare: TokioResolver,
    router: Option<TokioResolver>,
    recursive: Option<TokioResolver>,
    pub mode: ResolverMode,
    pub priority: ResolverPriority,
    pub cloudflare_addr: SocketAddr,
    pub router_addr: Option<SocketAddr>,
    metrics: Option<Arc<Metrics>>,
}

impl UpstreamResolver {
    #[allow(non_snake_case)]
    pub fn fromConfig(
        mode: ResolverMode,
        priority: ResolverPriority,
        cloudflare_addr: SocketAddr,
        router_addr: Option<SocketAddr>,
        root_hints: Vec<SocketAddr>,
    ) -> anyhow::Result<Self> {
        let effective_router = router_addr.or_else(detectGateway);
        let cloudflare = buildResolver(cloudflare_addr)?;
        let router = effective_router.map(buildResolver).transpose()?;
        let recursive = if mode == ResolverMode::Recursive {
            Some(buildRecursiveResolver(&root_hints)?)
        } else {
            None
        };

        if let Some(addr) = effective_router {
            tracing::info!(%addr, "Router/gateway DNS detected");
        } else {
            tracing::warn!("Could not detect gateway DNS; router fallback unavailable");
        }

        if mode == ResolverMode::Recursive {
            let hint_count = if root_hints.is_empty() { 13 } else { root_hints.len() };
            tracing::info!(%mode, hint_count, "Recursive resolver configured with root hints");
        } else {
            tracing::info!(%mode, "Resolver engine configured");
        }

        Ok(Self {
            cloudflare,
            router,
            recursive,
            mode,
            priority,
            cloudflare_addr,
            router_addr: effective_router,
            metrics: None,
        })
    }

    pub fn attach_metrics(&mut self, metrics: Arc<Metrics>) {
        self.metrics = Some(metrics);
    }

    pub async fn resolve(&self, name: &Name, rtype: RecordType) -> UpstreamResolution {
        let started = Instant::now();
        if let Some(metrics) = &self.metrics {
            metrics.record_upstream_start();
        }

        let result = if self.mode == ResolverMode::Recursive {
            if let Some(rec) = &self.recursive {
                queryResolver(rec, name, rtype).await
            } else {
                let (first, second) = self.orderedResolvers();
                self.resolveForwarding(first, second, name, rtype).await
            }
        } else {
            let (first, second) = self.orderedResolvers();
            self.resolveForwarding(first, second, name, rtype).await
        };

        if let Some(metrics) = &self.metrics {
            metrics.record_upstream_result(matches!(
                &result,
                UpstreamResolution::Positive(_, _)
                    | UpstreamResolution::Nodata
                    | UpstreamResolution::NxDomain
            ));
            metrics.record_upstream_latency(started.elapsed().as_secs_f64() * 1000.0);
        }
        result
    }

    #[allow(non_snake_case)]
    async fn resolveForwarding(
        &self,
        first: &TokioResolver,
        second: Option<&TokioResolver>,
        name: &Name,
        rtype: RecordType,
    ) -> UpstreamResolution {
        match queryResolver(first, name, rtype).await {
            UpstreamResolution::Positive(records, ttl) => {
                UpstreamResolution::Positive(records, ttl)
            }
            UpstreamResolution::NxDomain => UpstreamResolution::NxDomain,
            UpstreamResolution::Nodata => {
                if let Some(resolver) = second {
                    queryResolver(resolver, name, rtype).await
                } else {
                    UpstreamResolution::Nodata
                }
            }
            UpstreamResolution::ServFail => {
                if let Some(resolver) = second {
                    queryResolver(resolver, name, rtype).await
                } else {
                    UpstreamResolution::ServFail
                }
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
                tracing::info!(query=%name, "Upstream returned NXDOMAIN");
                UpstreamResolution::NxDomain
            } else if e.is_no_records_found() {
                tracing::info!(query=%name, "Upstream returned NODATA");
                UpstreamResolution::Nodata
            } else {
                tracing::warn!(query=%name, error=%e, "Upstream lookup failed; returning SERVFAIL");
                UpstreamResolution::ServFail
            }
        }
    }
}
