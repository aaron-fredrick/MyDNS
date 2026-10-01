//! Performance aggregation for API measurements.

use crate::observability::telemetry::metrics::{
    domains::api::{
        bucket::ApiPerformanceBucket,
        measurements::{ApiMeasurement, ApiMeasurementFamily},
        snapshot::ApiPerformanceSnapshot,
    },
    traits::CategoryAggregatorTrait,
};

/// Coordinates API performance aggregation over the active bucket.
///
/// Metric state and measurement dispatch belong to ApiPerformanceBucket.
/// The aggregator owns the bucket boundary and the recording lease around
/// each delegated measurement. Rotation will later replace the active bucket
/// with a reserve bucket without changing the bucket's metric contract.
pub struct ApiPerformanceAggregator {
    active: ApiPerformanceBucket,
}

impl ApiPerformanceAggregator {
    pub fn new() -> Self {
        Self {
            active: ApiPerformanceBucket::new(),
        }
    }
}

impl Default for ApiPerformanceAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryAggregatorTrait<ApiMeasurementFamily> for ApiPerformanceAggregator {
    type Snapshot = ApiPerformanceSnapshot;

    fn record(&self, measurement: ApiMeasurement<'_>) {
        assert!(
            self.active.try_acquire(),
            "active API performance bucket unexpectedly rejected a recording lease"
        );

        self.active.record(measurement);
        self.active.release();
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.active.snapshot()
    }

    fn reset(&self) {
        self.active.reset();
    }
}
