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

pub fn detect_gateway() -> Option<SocketAddr> {
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

fn build_resolver(addr: SocketAddr) -> anyhow::Result<TokioResolver> {
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

/// Sends a single non-recursive DNS query via UDP and returns the raw response message.
/// Returns `None` on timeout or any I/O error.
async fn raw_dns_query(
    server: SocketAddr,
    name: &Name,
    rtype: RecordType,
) -> Option<hickory_proto::op::Message> {
    use hickory_proto::op::{Message, MessageType, OpCode, Query as DnsQuery};

    let mut msg = Message::new(rand::random::<u16>(), MessageType::Query, OpCode::Query);
    msg.metadata.recursion_desired = false;
    msg.add_query(DnsQuery::query(name.clone(), rtype));

    let bytes = msg.to_vec().ok()?;

    let bind_addr = if server.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = tokio::net::UdpSocket::bind(bind_addr).await.ok()?;
    socket.send_to(&bytes, server).await.ok()?;

    let mut recv_buf = vec![0u8; 4096];
    let (len, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut recv_buf))
        .await
        .ok()?
        .ok()?;

    Message::from_vec(&recv_buf[..len]).ok()
}

pub struct UpstreamResolver {
    cloudflare: TokioResolver,
    router: Option<TokioResolver>,
    pub mode: ResolverMode,
    pub priority: ResolverPriority,
    pub cloudflare_addr: SocketAddr,
    pub router_addr: Option<SocketAddr>,
    pub root_hints: Vec<SocketAddr>,
    metrics: Option<Arc<Metrics>>,
}

impl UpstreamResolver {
    pub fn from_config(
        mode: ResolverMode,
        priority: ResolverPriority,
        cloudflare_addr: SocketAddr,
        router_addr: Option<SocketAddr>,
        root_hints: Vec<SocketAddr>,
    ) -> anyhow::Result<Self> {
        let effective_router = router_addr.or_else(detect_gateway);
        let cloudflare = build_resolver(cloudflare_addr)?;
        let router = effective_router.map(build_resolver).transpose()?;
        let root_hints = if root_hints.is_empty() {
            default_root_hints()
        } else {
            root_hints
        };

        if let Some(addr) = effective_router {
            tracing::info!(%addr, "Router/gateway DNS detected");
        } else {
            tracing::warn!("Could not detect gateway DNS; router fallback unavailable");
        }

        if mode == ResolverMode::Recursive {
            let hint_count = if root_hints.is_empty() {
                13
            } else {
                root_hints.len()
            };
            tracing::info!(%mode, hint_count, "Recursive resolver configured with root hints");
        } else {
            tracing::info!(%mode, "Resolver engine configured");
        }

        Ok(Self {
            cloudflare,
            router,
            mode,
            priority,
            cloudflare_addr,
            router_addr: effective_router,
            root_hints,
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
            self.resolve_iterative(name, rtype).await
        } else {
            let (first, second) = self.ordered_resolvers();
            self.resolve_forwarding(first, second, name, rtype).await
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

    async fn resolve_forwarding(
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

    fn ordered_resolvers(&self) -> (&TokioResolver, Option<&TokioResolver>) {
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

    async fn resolve_iterative(&self, name: &Name, rtype: RecordType) -> UpstreamResolution {
        use colored::Colorize;
        use hickory_proto::op::ResponseCode;
        use hickory_proto::rr::RData;

        let mut current_servers = self.root_hints.clone();
        eprintln!(
            "{} Starting iterative recursion for {} (type {}) with {} root hints",
            "[RECURSE START]".magenta().bold(),
            name.to_string().cyan().bold(),
            rtype.to_string().yellow().bold(),
            current_servers.len()
        );

        for depth in 0..10usize {
            let mut next_servers: Vec<SocketAddr> = Vec::new();
            let mut ns_names: Vec<Name> = Vec::new();
            let mut got_response = false;

            let level_tag = match depth {
                0 => "ROOT",
                1 => "TLD",
                _ => "AUTH/DELEGATION",
            };

            let step_indent = "  ".repeat(depth);
            let res_indent = format!("{}  ↳", step_indent);

            // Try up to 3 servers at this delegation level
            for (idx, &server) in current_servers.iter().take(3).enumerate() {
                eprintln!(
                    "{}{} Querying {} (candidate {}/{}) for {}",
                    step_indent,
                    format!("[RECURSE step {} | {}]", depth, level_tag)
                        .magenta()
                        .bold(),
                    server.to_string().cyan().bold(),
                    idx + 1,
                    current_servers.len().min(3),
                    name.to_string().bold()
                );

                let Some(response) = raw_dns_query(server, name, rtype).await else {
                    eprintln!(
                        "{} {} Server {} timed out, trying next server...",
                        res_indent,
                        "[RECURSE TIMEOUT]".yellow().bold(),
                        server.to_string().cyan().bold()
                    );
                    continue;
                };
                got_response = true;

                // Authoritative answer
                if !response.answers.is_empty() {
                    let records: Vec<Record> = response.answers.clone();
                    let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
                    let values = records
                        .iter()
                        .map(|r| r.data.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    eprintln!(
                        "{} {} Authoritative answer from {}: {} (TTL={}s)",
                        res_indent,
                        "[RECURSE ANSWER]".green().bold(),
                        server.to_string().cyan().bold(),
                        format!("[{}]", values).green().bold(),
                        ttl
                    );
                    return UpstreamResolution::Positive(records, ttl);
                }

                match response.metadata.response_code {
                    ResponseCode::NXDomain => {
                        eprintln!(
                            "{} {} Server {} returned NXDOMAIN for {}",
                            res_indent,
                            "[RECURSE NXDOMAIN]".red().bold(),
                            server.to_string().cyan().bold(),
                            name.to_string().bold()
                        );
                        return UpstreamResolution::NxDomain;
                    }
                    ResponseCode::NoError => {}
                    _ => continue,
                }

                // Check if this is an authoritative NODATA (NoError, 0 answers, SOA in authority, no NS)
                let has_soa = response
                    .authorities
                    .iter()
                    .any(|r| matches!(r.data, RData::SOA(_)));
                let has_ns = response
                    .authorities
                    .iter()
                    .any(|r| matches!(r.data, RData::NS(_)));

                if has_soa && !has_ns {
                    eprintln!(
                        "{} {} Server {} returned NODATA for {} (type {})",
                        res_indent,
                        "[RECURSE NODATA]".yellow().bold(),
                        server.to_string().cyan().bold(),
                        name.to_string().bold(),
                        rtype
                    );
                    return UpstreamResolution::Nodata;
                }

                // Extract glue A/AAAA records from the additional section
                for rec in &response.additionals {
                    match &rec.data {
                        RData::A(a) => {
                            next_servers.push(SocketAddr::new(IpAddr::V4(a.0), 53));
                        }
                        RData::AAAA(aaaa) => {
                            next_servers.push(SocketAddr::new(IpAddr::V6(aaaa.0), 53));
                        }
                        _ => {}
                    }
                }

                // No glue: save NS hostnames for resolution
                if next_servers.is_empty() {
                    for rec in &response.authorities {
                        if let RData::NS(ns) = &rec.data {
                            ns_names.push(ns.0.clone());
                        }
                    }
                }

                if !next_servers.is_empty() {
                    let glue_str = next_servers
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    eprintln!(
                        "{} {} Server {} referred with {} glue IPs: {}",
                        res_indent,
                        "[RECURSE REFERRAL]".blue().bold(),
                        server.to_string().cyan().bold(),
                        next_servers.len(),
                        format!("[{}]", glue_str).cyan().bold()
                    );
                    break;
                }
            }

            let step_indent = "  ".repeat(depth);
            let res_indent = format!("{}  ↳", step_indent);

            if !got_response {
                eprintln!(
                    "{} {} All iterative servers timed out at step {} for query {}",
                    res_indent,
                    "[RECURSE FAILED]".red().bold(),
                    depth,
                    name
                );
                return UpstreamResolution::ServFail;
            }

            // No glue in the additional section — resolve the NS hostnames via
            // the forwarding resolver (Cloudflare) and use those IPs.
            if next_servers.is_empty() {
                let ns_str = ns_names
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                eprintln!(
                    "{} {} Un-glued referral to NS [{}]; resolving NS IP addresses via upstream...",
                    res_indent,
                    "[RECURSE RESOLVE-NS]".blue().bold(),
                    ns_str
                );

                for ns_name in ns_names.iter().take(2) {
                    if let UpstreamResolution::Positive(a_records, _) =
                        query_resolver(&self.cloudflare, ns_name, RecordType::A).await
                    {
                        for rec in &a_records {
                            if let RData::A(a) = &rec.data {
                                next_servers.push(SocketAddr::new(IpAddr::V4(a.0), 53));
                            }
                        }
                    }
                    if !next_servers.is_empty() {
                        break;
                    }
                }
            }

            if next_servers.is_empty() {
                eprintln!(
                    "{} {} Referral with no resolvable next-hop servers at step {} for {}",
                    res_indent,
                    "[RECURSE FAILED]".red().bold(),
                    depth,
                    name
                );
                return UpstreamResolution::ServFail;
            }

            current_servers = next_servers;
        }

        eprintln!(
            "{} Iterative resolution exceeded maximum depth for {}",
            "[RECURSE LIMIT]".red().bold(),
            name
        );
        UpstreamResolution::ServFail
    }
}

async fn query_resolver(
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
