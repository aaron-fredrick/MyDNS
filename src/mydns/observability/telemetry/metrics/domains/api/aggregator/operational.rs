//! Operational aggregation for API measurements.

use crate::observability::telemetry::metrics::types::{BoundedCounter, ScalarCounter};

pub struct ApiOperationalAggregator {
    pub requests: ScalarCounter,
    pub responses: ScalarCounter,

    pub request_size_bytes: ScalarCounter,
    pub response_size_bytes: ScalarCounter,

    pub authentication_successes: ScalarCounter,
    pub authentication_failures: ScalarCounter,

    pub errors: ScalarCounter,

    pub method_counts: BoundedCounter,
    pub route_counts: BoundedCounter,
    pub status_counts: BoundedCounter,
    pub error_category_counts: BoundedCounter,
}
