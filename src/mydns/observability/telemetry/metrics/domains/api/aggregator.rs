//! Domain-level aggregation for API measurements.

use crate::observability::telemetry::metrics::types::{
    BoundedCounter, Histogram, ScalarCounter,
};

const LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
];

pub struct ApiPerformanceAggregator {
    pub request_latency: Histogram,
    pub handler_latency: Histogram,
}

pub struct ApiOperationalAggregator {
    pub requests: ScalarCounter,
    pub responses: ScalarCounter,
    pub authentication_successes: ScalarCounter,
    pub authentication_failures: ScalarCounter,
    pub errors: ScalarCounter,

    pub method_counts: BoundedCounter,
    pub route_counts: BoundedCounter,
    pub status_counts: BoundedCounter,
    pub error_category_counts: BoundedCounter,
}
