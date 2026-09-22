use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

#[cfg(unix)]
use std::net::Ipv4Addr;

use hickory_proto::rr::{Name, Record, RecordType};
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

use super::{UpstreamResolution, UpstreamResolver};

pub(super) fn detect_gateway() -> Option<SocketAddr> {
    detect_gateway_impl().map(|ip| SocketAddr::new(ip, 53))
}

#[cfg(windows)]
fn detect_gateway_impl() -> Option<IpAddr> {
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

#[cfg(unix)]
fn detect_gateway_impl() -> Option<IpAddr> {
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

#[cfg(not(any(windows, unix)))]
fn detect_gateway_impl() -> Option<IpAddr> {
    None
}

pub(super) fn build_resolver(addr: SocketAddr) -> anyhow::Result<TokioResolver> {
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

impl UpstreamResolver {
    #[tracing::instrument(
        name = "resolve_forwarding",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, has_fallback = second.is_some()),
        skip(self)
    )]
    pub(super) async fn resolve_forwarding(
        &self,
        first: &TokioResolver,
        second: Option<&TokioResolver>,
        name: &Name,
        rtype: RecordType,
    ) -> UpstreamResolution {
        match query_resolver(first, name, rtype).await {
            UpstreamResolution::Positive(records, ttl) => {
                UpstreamResolution::Positive(records, ttl)
            }
            UpstreamResolution::NxDomain => UpstreamResolution::NxDomain,
            UpstreamResolution::Nodata => {
                if let Some(resolver) = second {
                    query_resolver(resolver, name, rtype).await
                } else {
                    UpstreamResolution::Nodata
                }
            }
            UpstreamResolution::ServFail => {
                if let Some(resolver) = second {
                    query_resolver(resolver, name, rtype).await
                } else {
                    UpstreamResolution::ServFail
                }
            }
        }
    }

    pub(super) fn ordered_resolvers(&self) -> (&TokioResolver, Option<&TokioResolver>) {
        match self.priority {
            crate::config::ResolverPriority::CloudflareFirst => {
                (&self.cloudflare, self.router.as_ref())
            }
            crate::config::ResolverPriority::RouterFirst => {
                if let Some(router) = &self.router {
                    (router, Some(&self.cloudflare))
                } else {
                    (&self.cloudflare, None)
                }
            }
        }
    }
}

pub(super) async fn query_resolver(
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
                UpstreamResolution::NxDomain
            } else if e.is_no_records_found() {
                UpstreamResolution::Nodata
            } else {
                UpstreamResolution::ServFail
            }
        }
    }
}
