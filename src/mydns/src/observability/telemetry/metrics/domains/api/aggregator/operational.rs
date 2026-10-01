//! Operational aggregation for API measurements.

use crate::observability::telemetry::metrics::{
    domains::api::{
        bucket::ApiOperationalBucket,
        measurements::{ApiMeasurement, ApiMeasurementFamily},
        snapshot::ApiOperationalSnapshot,
    },
    traits::CategoryAggregatorTrait,
};

/// Coordinates API operational aggregation over the active bucket.
///
/// Metric state and measurement dispatch belong to ApiOperationalBucket.
/// The aggregator owns the bucket boundary and the recording lease around
/// each delegated measurement. Rotation will later replace the active bucket
/// with a reserve bucket without changing the bucket's metric contract.
pub struct ApiOperationalAggregator {
    active: ApiOperationalBucket,
}

impl ApiOperationalAggregator {
    pub fn new() -> Self {
        Self {
            active: ApiOperationalBucket::new(),
        }
    }
}

impl Default for ApiOperationalAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryAggregatorTrait<ApiMeasurementFamily> for ApiOperationalAggregator {
    type Snapshot = ApiOperationalSnapshot;

    fn record(&self, measurement: ApiMeasurement<'_>) {
        assert!(
            self.active.try_acquire(),
            "active API operational bucket unexpectedly rejected a recording lease"
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
