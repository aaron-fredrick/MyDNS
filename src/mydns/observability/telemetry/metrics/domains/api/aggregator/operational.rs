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

impl ApiOperationalAggregator {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(ScalarCounter::new()),
            responses: Mutex::new(ScalarCounter::new()),

            request_size_bytes: Mutex::new(ScalarCounter::new()),
            response_size_bytes: Mutex::new(ScalarCounter::new()),

            authentication_successes: Mutex::new(ScalarCounter::new()),
            authentication_failures: Mutex::new(ScalarCounter::new()),

            errors: Mutex::new(ScalarCounter::new()),

            method_counts: Mutex::new(BoundedCounter::default()),
            route_counts: Mutex::new(BoundedCounter::default()),
            status_counts: Mutex::new(BoundedCounter::default()),
            error_category_counts: Mutex::new(BoundedCounter::default()),
        }
    }
}
