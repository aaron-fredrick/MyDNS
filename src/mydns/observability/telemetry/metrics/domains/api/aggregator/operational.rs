//! Operational aggregation for API measurements.

use std::sync::Mutex;

use crate::observability::telemetry::metrics::types::{BoundedCounter, ScalarCounter};

pub struct ApiOperationalAggregator {
    // AtomicU64 is a better fit for these scalar counters; Mutex is used here
    // until the aggregator's atomic synchronization strategy is introduced.
    pub requests: Mutex<ScalarCounter>,
    pub responses: Mutex<ScalarCounter>,

    pub request_size_bytes: Mutex<ScalarCounter>,
    pub response_size_bytes: Mutex<ScalarCounter>,

    pub authentication_successes: Mutex<ScalarCounter>,
    pub authentication_failures: Mutex<ScalarCounter>,

    pub errors: Mutex<ScalarCounter>,

    pub method_counts: Mutex<BoundedCounter>,
    pub route_counts: Mutex<BoundedCounter>,
    pub status_counts: Mutex<BoundedCounter>,
    pub error_category_counts: Mutex<BoundedCounter>,
}
