//! Mutable metric state for API performance aggregation.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Mutex,
};

use crate::observability::telemetry::metrics::types::{DistributionMetrics, Gauge, ScalarCounter};

const LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
];

const SIZE_BOUNDS_BYTES: &[f64] = &[
    64.0,
    256.0,
    1024.0,
    4096.0,
    16_384.0,
    65_536.0,
    262_144.0,
    1_048_576.0,
    4_194_304.0,
    16_777_216.0,
];

/// Mutable metric state for one API performance aggregation window.
///
/// Metric-level mutexes preserve the concurrency model of the current
/// performance aggregator. Bucket leasing/lifecycle synchronization is a
/// separate concern and will be added when the aggregator is migrated.
pub struct ApiPerformanceBucket {
    /// Number of active recording leases on this bucket.
    pub(crate) users: AtomicUsize,
    /// Prevents new recording leases while the bucket is being drained.
    pub(crate) draining: AtomicBool,

    pub(crate) request_latency: Mutex<DistributionMetrics>,
    pub(crate) handler_latency: Mutex<DistributionMetrics>,
    pub(crate) request_count: Mutex<ScalarCounter>,
    pub(crate) request_concurrency: Mutex<Gauge>,
    pub(crate) request_size: Mutex<DistributionMetrics>,
    pub(crate) response_size: Mutex<DistributionMetrics>,
}

impl ApiPerformanceBucket {
    pub fn new() -> Self {
        Self {
            users: AtomicUsize::new(0),
            draining: AtomicBool::new(false),
            request_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            handler_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            request_count: Mutex::new(ScalarCounter::new()),
            request_concurrency: Mutex::new(Gauge::default()),
            request_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
            response_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
        }
    }
}

impl Default for ApiPerformanceBucket {
    fn default() -> Self {
        Self::new()
    }
}
