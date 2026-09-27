//! Detached operational snapshots produced by the API operational aggregator.

use crate::observability::telemetry::metrics::{
    CategorySnapshotTrait,
    types::{
    BoundedCounterSnapshot, ScalarCounterSnapshot,
},
};
use serde::{Deserialize, Serialize};

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

impl CategorySnapshotTrait for ApiOperationalSnapshot {
    pub fn merge(&mut self, other: &Self) {
        self.requests.merge(&other.requests);
        self.responses.merge(&other.responses);
        self.request_size_bytes.merge(&other.request_size_bytes);
        self.response_size_bytes.merge(&other.response_size_bytes);
        self.authentication_successes
            .merge(&other.authentication_successes);
        self.authentication_failures
            .merge(&other.authentication_failures);
        self.errors.merge(&other.errors);
        self.method_counts.merge(&other.method_counts);
        self.route_counts.merge(&other.route_counts);
        self.status_counts.merge(&other.status_counts);
        self.error_category_counts
            .merge(&other.error_category_counts);
    }
}
