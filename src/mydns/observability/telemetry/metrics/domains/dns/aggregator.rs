//! Domain-level aggregation for DNS measurements.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::types::{BoundedCounter, MergeableHistogram};

use super::measurements::{OperationalMeasurement, PerformanceMeasurement};

const RESPONSE_LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
];

const UPSTREAM_LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 3000.0, 5000.0,
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsOperationalSnapshot {
    pub queries: u64,
    pub responses: u64,
    pub blocked: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub upstream_requests: u64,
    pub upstream_successes: u64,
    pub upstream_failures: u64,
    pub upstream_timeouts: u64,
    pub upstream_retries: u64,
    pub blocked_reason_counts: BoundedCounter,
    pub record_type_counts: BoundedCounter,
    pub transport_counts: BoundedCounter,
    pub response_code_counts: BoundedCounter,
    pub resolution_outcome_counts: BoundedCounter,
    pub resolution_path_counts: BoundedCounter,
}

pub struct DnsOperationalAggregator {
    state: Mutex<DnsOperationalSnapshot>,
}

impl DnsOperationalAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, measurement: OperationalMeasurement<'_>) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            OperationalMeasurement::Query { record_type, transport } => {
                state.queries += 1;
                state.record_type_counts.record(&record_type.trim().to_ascii_uppercase());
                state.transport_counts.record(&transport.trim().to_ascii_lowercase());
            }
            OperationalMeasurement::Response { response_code } => {
                state.responses += 1;
                state.response_code_counts.record(&response_code.trim().to_ascii_uppercase());
            }
            OperationalMeasurement::Blocked { reason } => {
                state.blocked += 1;
                state.blocked_reason_counts.record(reason);
            }
            OperationalMeasurement::Resolution { outcome, path } => {
                state.resolution_outcome_counts.record(outcome);
                state.resolution_path_counts.record(path);
            }
            OperationalMeasurement::Cache { hit } => {
                if hit { state.cache_hits += 1; } else { state.cache_misses += 1; }
            }
            OperationalMeasurement::CacheEviction => state.cache_evictions += 1,
            OperationalMeasurement::Upstream { successful, timeout, retry } => {
                state.upstream_requests += 1;
                if successful { state.upstream_successes += 1; } else { state.upstream_failures += 1; }
                if timeout { state.upstream_timeouts += 1; }
                if retry { state.upstream_retries += 1; }
            }
        }
    }

    pub fn snapshot(&self) -> DnsOperationalSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DnsOperationalAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DnsOperationalSnapshot {
                queries: 0,
                responses: 0,
                blocked: 0,
                cache_hits: 0,
                cache_misses: 0,
                cache_evictions: 0,
                upstream_requests: 0,
                upstream_successes: 0,
                upstream_failures: 0,
                upstream_timeouts: 0,
                upstream_retries: 0,
                blocked_reason_counts: BoundedCounter::default(),
                record_type_counts: BoundedCounter::default(),
                transport_counts: BoundedCounter::default(),
                response_code_counts: BoundedCounter::default(),
                resolution_outcome_counts: BoundedCounter::default(),
                resolution_path_counts: BoundedCounter::default(),
            }),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsPerformanceSnapshot {
    pub response_latency: MergeableHistogram,
    pub upstream_latency: MergeableHistogram,
}

pub struct DnsPerformanceAggregator {
    state: Mutex<DnsPerformanceSnapshot>,
}

impl DnsPerformanceAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, measurement: PerformanceMeasurement) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            PerformanceMeasurement::ResponseLatency { latency_ms } => {
                state.response_latency.record(latency_ms);
            }
            PerformanceMeasurement::UpstreamLatency { latency_ms } => {
                state.upstream_latency.record(latency_ms);
            }
        }
    }

    pub fn snapshot(&self) -> DnsPerformanceSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DnsPerformanceAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DnsPerformanceSnapshot {
                response_latency: MergeableHistogram::new(RESPONSE_LATENCY_BOUNDS_MS),
                upstream_latency: MergeableHistogram::new(UPSTREAM_LATENCY_BOUNDS_MS),
            }),
        }
    }
}
