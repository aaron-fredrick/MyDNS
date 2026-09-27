//! Detached performance snapshots produced by the API performance aggregator.

use chrono::{DateTime, Utc};
use mydns_macros::metric_category_snapshot;

use crate::observability::telemetry::metrics::{
    traits::CategorySnapshotTrait,
    types::{DistributionSnapshot, GaugeSnapshot, ScalarCounterSnapshot},
};

use serde::{Deserialize, Serialize};

#[metric_category_snapshot]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiPerformanceSnapshot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub request_latency: DistributionSnapshot,
    pub handler_latency: DistributionSnapshot,
    pub request_count: ScalarCounterSnapshot,
    pub request_concurrency: GaugeSnapshot,
    pub request_size: DistributionSnapshot,
    pub response_size: DistributionSnapshot,
}

impl CategorySnapshotTrait for ApiPerformanceSnapshot {
    fn merge(&mut self, other: &Self) {
        self.start_time = self.start_time.min(other.start_time);
        self.end_time = self.end_time.max(other.end_time);
        self.request_latency.merge(&other.request_latency);
        self.handler_latency.merge(&other.handler_latency);
        self.request_count.merge(&other.request_count);
        self.request_concurrency.merge(&other.request_concurrency);
        self.request_size.merge(&other.request_size);
        self.response_size.merge(&other.response_size);
    }
}
