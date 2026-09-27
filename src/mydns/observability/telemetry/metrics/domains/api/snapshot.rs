//! Detached snapshots produced by API aggregators.
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::observability::telemetry::metrics::types::{BoundedCounterSnapshot, DistributionSnapshot, GaugeSnapshot, ScalarCounterSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiPerformanceSnapshot {
    pub request_latency: DistributionSnapshot,
    pub handler_latency: DistributionSnapshot,
    pub request_count: ScalarCounterSnapshot,
    pub request_concurrency: GaugeSnapshot,
    pub request_size: DistributionSnapshot,
    pub response_size: DistributionSnapshot,
}
impl ApiPerformanceSnapshot {
    pub fn merge(&mut self, other: &Self) {
        self.request_latency.merge(&other.request_latency);
        self.handler_latency.merge(&other.handler_latency);
        self.request_count.merge(&other.request_count);
        self.request_concurrency.merge(&other.request_concurrency);
        self.request_size.merge(&other.request_size);
        self.response_size.merge(&other.response_size);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiOperationalSnapshot {
    pub requests: ScalarCounterSnapshot,
    pub responses: ScalarCounterSnapshot,
    pub request_size_bytes: ScalarCounterSnapshot,
    pub response_size_bytes: ScalarCounterSnapshot,
    pub authentication_successes: ScalarCounterSnapshot,
    pub authentication_failures: ScalarCounterSnapshot,
    pub errors: ScalarCounterSnapshot,
    pub method_counts: BoundedCounterSnapshot,
    pub route_counts: BoundedCounterSnapshot,
    pub status_counts: BoundedCounterSnapshot,
    pub error_category_counts: BoundedCounterSnapshot,
}
impl ApiOperationalSnapshot {
    pub fn merge(&mut self, other: &Self) {
        self.requests.merge(&other.requests);
        self.responses.merge(&other.responses);
        self.request_size_bytes.merge(&other.request_size_bytes);
        self.response_size_bytes.merge(&other.response_size_bytes);
        self.authentication_successes.merge(&other.authentication_successes);
        self.authentication_failures.merge(&other.authentication_failures);
        self.errors.merge(&other.errors);
        self.method_counts.merge(&other.method_counts);
        self.route_counts.merge(&other.route_counts);
        self.status_counts.merge(&other.status_counts);
        self.error_category_counts.merge(&other.error_category_counts);
    }
}
