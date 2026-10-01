//! Mutable metric state for API operational aggregation.

use crate::observability::telemetry::metrics::types::{BoundedCounter, ScalarCounter};

/// Mutable metric state for one API operational aggregation window.
///
/// This contains only metric state. Window timing and bucket lifecycle are
/// deliberately handled outside the API-specific metric state.
pub struct ApiOperationalBucket {
    pub(crate) requests: ScalarCounter,
    pub(crate) responses: ScalarCounter,
    pub(crate) request_size_bytes: ScalarCounter,
    pub(crate) response_size_bytes: ScalarCounter,
    pub(crate) authentication_successes: ScalarCounter,
    pub(crate) authentication_failures: ScalarCounter,
    pub(crate) errors: ScalarCounter,
    pub(crate) method_counts: BoundedCounter,
    pub(crate) route_counts: BoundedCounter,
    pub(crate) status_counts: BoundedCounter,
    pub(crate) error_category_counts: BoundedCounter,
}

impl ApiOperationalBucket {
    pub fn new() -> Self {
        Self {
            requests: ScalarCounter::new(),
            responses: ScalarCounter::new(),
            request_size_bytes: ScalarCounter::new(),
            response_size_bytes: ScalarCounter::new(),
            authentication_successes: ScalarCounter::new(),
            authentication_failures: ScalarCounter::new(),
            errors: ScalarCounter::new(),
            method_counts: BoundedCounter::default(),
            route_counts: BoundedCounter::default(),
            status_counts: BoundedCounter::default(),
            error_category_counts: BoundedCounter::default(),
        }
    }
}

impl Default for ApiOperationalBucket {
    fn default() -> Self {
        Self::new()
    }
}
