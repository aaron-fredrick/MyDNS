//! Mutable metric state for API operational aggregation.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Mutex,
};

use crate::observability::telemetry::metrics::types::{BoundedCounter, ScalarCounter};

/// Mutable metric state for one API operational aggregation window.
///
/// Metric-level mutexes preserve the concurrency model of the current
/// operational aggregator. Bucket leasing/lifecycle synchronization is a
/// separate concern and will be added when the aggregator is migrated.
pub struct ApiOperationalBucket {
    /// Number of active recording leases on this bucket.
    pub(crate) users: AtomicUsize,
    /// Prevents new recording leases while the bucket is being drained.
    pub(crate) draining: AtomicBool,

    pub(crate) requests: Mutex<ScalarCounter>,
    pub(crate) responses: Mutex<ScalarCounter>,
    pub(crate) request_size_bytes: Mutex<ScalarCounter>,
    pub(crate) response_size_bytes: Mutex<ScalarCounter>,
    pub(crate) authentication_successes: Mutex<ScalarCounter>,
    pub(crate) authentication_failures: Mutex<ScalarCounter>,
    pub(crate) errors: Mutex<ScalarCounter>,
    pub(crate) method_counts: Mutex<BoundedCounter>,
    pub(crate) route_counts: Mutex<BoundedCounter>,
    pub(crate) status_counts: Mutex<BoundedCounter>,
    pub(crate) error_category_counts: Mutex<BoundedCounter>,
}

impl ApiOperationalBucket {
    pub fn new() -> Self {
        Self {
            users: AtomicUsize::new(0),
            draining: AtomicBool::new(false),
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

impl Default for ApiOperationalBucket {
    fn default() -> Self {
        Self::new()
    }
}
