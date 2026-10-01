//! Mutable metric state for API performance aggregation.

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
/// This contains only metric state. Window timing and bucket lifecycle are
/// deliberately handled outside the API-specific metric state.
pub struct ApiPerformanceBucket {
    pub(crate) request_latency: DistributionMetrics,
    pub(crate) handler_latency: DistributionMetrics,
    pub(crate) request_count: ScalarCounter,
    pub(crate) request_concurrency: Gauge,
    pub(crate) request_size: DistributionMetrics,
    pub(crate) response_size: DistributionMetrics,
}

impl ApiPerformanceBucket {
    pub fn new() -> Self {
        Self {
            request_latency: DistributionMetrics::new(LATENCY_BOUNDS_MS),
            handler_latency: DistributionMetrics::new(LATENCY_BOUNDS_MS),
            request_count: ScalarCounter::new(),
            request_concurrency: Gauge::default(),
            request_size: DistributionMetrics::new(SIZE_BOUNDS_BYTES),
            response_size: DistributionMetrics::new(SIZE_BOUNDS_BYTES),
        }
    }
}

impl Default for ApiPerformanceBucket {
    fn default() -> Self {
        Self::new()
    }
}
