//! Detached performance snapshots produced by the API performance aggregator.

use crate::observability::telemetry::metrics::types::{
    DistributionSnapshot, GaugeSnapshot, ScalarCounterSnapshot,
};
use serde::{Deserialize, Serialize};

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
