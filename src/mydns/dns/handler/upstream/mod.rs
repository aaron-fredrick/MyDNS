use super::{DnsHandler, ResolutionResult};
use crate::config::{ResolverMode, ResolverPriority};
use crate::observability::Metrics;
use hickory_proto::rr::{Name, Record, RecordType};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod forward;
mod recursive;

const MAX_RECURSION_DEPTH: usize = 10;

#[derive(Debug)]
pub enum UpstreamResolution {
    Positive(Vec<Record>, u32),
    Nodata,
    NxDomain,
    ServFail,
}

pub struct UpstreamResolver {
    cloudflare: hickory_resolver::TokioResolver,
    router: Option<hickory_resolver::TokioResolver>,
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
        let effective_router = router_addr.or_else(forward::detect_gateway);
        let cloudflare = forward::build_resolver(cloudflare_addr)?;
        let router = effective_router.map(forward::build_resolver).transpose()?;
        let root_hints = if root_hints.is_empty() {
            crate::config::default_root_hints()
        } else {
            root_hints
        };

        if let Some(addr) = effective_router {
            tracing::debug!(%addr, "Router/gateway DNS detected");
        } else {
            tracing::warn!("Could not detect gateway DNS; router fallback unavailable");
        }

        if mode == ResolverMode::Recursive {
            let hint_count = if root_hints.is_empty() {
                13
            } else {
                root_hints.len()
            };
            tracing::debug!(%mode, hint_count, "Recursive resolver configured with root hints");
            tracing::warn!("Recursive mode is active. DNSSEC validation is not implemented. This mode is suitable for trusted networks only.");
        } else {
            tracing::debug!(%mode, "Resolver engine configured");
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

    #[tracing::instrument(
        name = "resolve",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, mode = ?self.mode),
        skip(self)
    )]
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
}

impl DnsHandler {
    #[tracing::instrument(
        name = "query_upstream",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, client = %src),
        skip(self)
    )]
    pub(crate) async fn query_upstream(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> ResolutionResult {
        let fqdn = format!("{}.", name);
        let parsed_name = match fqdn.parse::<Name>() {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(query = %name, error = %e, "Invalid DNS name; returning SERVFAIL");
                return ResolutionResult::ServFail;
            }
        };
        let upstream = self.state.upstream.read().await;
        let addr = if upstream.mode == ResolverMode::Recursive {
            "recursive (root hints)".to_string()
        } else if let Some(addr) = upstream.router_addr {
            format!("{} (router)", addr)
        } else {
            format!("{} (cloudflare)", upstream.cloudflare_addr)
        };
        tracing::debug!(upstream_addr = %addr, "Querying upstream");
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            upstream.resolve(&parsed_name, rtype),
        )
        .await;
        match result {
            Ok(UpstreamResolution::Positive(records, _ttl)) => {
                let values = records
                    .iter()
                    .map(|r| r.data.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::debug!(client = %src, query = %name, r#type = %rtype, value = %values, upstream = %addr, "Upstream resolve hit");
                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM] client={} query={} type={} value=[{}] server={}",
                    src, name, rtype, values, addr
                ));
                ResolutionResult::Positive(records, false)
            }
            Ok(UpstreamResolution::Nodata) => ResolutionResult::Nodata(false),
            Ok(UpstreamResolution::NxDomain) => ResolutionResult::NxDomain(false),
            Ok(UpstreamResolution::ServFail) => ResolutionResult::ServFail,
            Err(_) => {
                tracing::warn!(client = %src, query = %name, r#type = %rtype, "Upstream resolve timed out; returning SERVFAIL");
                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM TIMEOUT] client={} query={} type={} server={}",
                    src, name, rtype, addr
                ));
                ResolutionResult::ServFail
            }
        }
    }
}
