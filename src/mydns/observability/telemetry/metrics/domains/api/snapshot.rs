//! Read-only data representations produced from API aggregators.
//!
//! Snapshots are detached from the live aggregators. They can therefore be
//! read, serialized, persisted, exported, or passed to higher-level aggregation
//! without exposing the aggregators' synchronization or mutation state.

use std::collections::BTreeMap;

use crate::observability::telemetry::metrics::types::DistributionSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct ApiPerformanceSnapshot {
    pub request_latency: DistributionSnapshot,
    pub handler_latency: DistributionSnapshot,
    pub request_count: u64,
    pub request_concurrency: f64,
    pub request_size: DistributionSnapshot,
    pub response_size: DistributionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiOperationalSnapshot {
    pub requests: u64,
    pub responses: u64,

    pub request_size_bytes: u64,
    pub response_size_bytes: u64,

    pub authentication_successes: u64,
    pub authentication_failures: u64,

    pub errors: u64,

    pub method_counts: BTreeMap<String, u64>,
    pub route_counts: BTreeMap<String, u64>,
    pub status_counts: BTreeMap<String, u64>,
    pub error_category_counts: BTreeMap<String, u64>,
}
