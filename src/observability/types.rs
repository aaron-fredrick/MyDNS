use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct LatencyStats {
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            avg_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UpstreamStats {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub availability_pct: f64,
    pub latency: LatencyStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub requests_per_minute: u64,
    pub queries_total: u64,
    pub upstream: UpstreamStats,
    pub response_time: LatencyStats,
    pub cache_evictions: u64,
    pub dns_errors: u64,
    pub query_types: HashMap<String, u64>,
    pub resolution_outcomes: HashMap<String, u64>,
}
