//! Operational aggregation for API measurements.

use std::sync::{Arc, Condvar, Mutex};

use arc_swap::ArcSwap;

use crate::observability::telemetry::metrics::{
    domains::api::{
        bucket::ApiOperationalBucket,
        measurements::{ApiMeasurement, ApiMeasurementFamily},
        snapshot::ApiOperationalSnapshot,
    },
    traits::CategoryAggregatorTrait,
};

/// Coordinates API operational aggregation across a double-buffered bucket set.
///
/// The active bucket is published through ArcSwap so recording can cheaply
/// obtain the current bucket. A short coordination lock makes active-bucket
/// selection and recording-lease acquisition atomic with respect to rotation.
/// The lock is never held while metric state is being mutated.
pub struct ApiOperationalAggregator {
    active: ArcSwap<ApiOperationalBucket>,
    reserve: Mutex<Option<Arc<ApiOperationalBucket>>>,
    coordination: Mutex<()>,
    rotation: Mutex<()>,
    drain_wait: Mutex<()>,
    drain_complete: Condvar,
}

struct OperationalLease<'a> {
    bucket: Arc<ApiOperationalBucket>,
    drain_complete: &'a Condvar,
}

impl Drop for OperationalLease<'_> {
    fn drop(&mut self) {
        self.bucket.release();
        self.drain_complete.notify_all();
    }
}

impl ApiOperationalAggregator {
    pub fn new() -> Self {
        Self {
            active: ArcSwap::from_pointee(ApiOperationalBucket::new()),
            reserve: Mutex::new(Some(Arc::new(ApiOperationalBucket::new()))),
            coordination: Mutex::new(()),
            rotation: Mutex::new(()),
            drain_wait: Mutex::new(()),
            drain_complete: Condvar::new(),
        }
    }

    fn acquire(&self) -> OperationalLease<'_> {
        let coordination = self.coordination.lock().unwrap();
        let bucket = self.active.load_full();

        assert!(
            bucket.try_acquire(),
            "active API operational bucket rejected a lease while protected by coordination"
        );

        drop(coordination);

        OperationalLease {
            bucket,
            drain_complete: &self.drain_complete,
        }
    }

    /// Rotates the active bucket and returns a snapshot of the detached bucket.
    ///
    /// New recordings are admitted to the replacement bucket immediately after
    /// the active pointer is swapped. Existing recordings finish on the detached
    /// bucket before it is snapshotted, reset, and returned to the reserve slot.
    pub fn rotate(&self) -> ApiOperationalSnapshot {
        let _rotation = self.rotation.lock().unwrap();

        let next = {
            let mut reserve = self.reserve.lock().unwrap();
            let bucket = reserve
                .take()
                .expect("API operational reserve bucket missing");

            bucket.reset();
            bucket
        };

        let old = {
            let _coordination = self.coordination.lock().unwrap();
            let old = self.active.load_full();

            old.begin_draining();
            self.active.store(next);

            old
        };

        let mut wait = self.drain_wait.lock().unwrap();
        while !old.is_drained() {
            wait = self.drain_complete.wait(wait).unwrap();
        }
        drop(wait);

        let snapshot = old.snapshot();
        old.reset();

        *self.reserve.lock().unwrap() = Some(old);

        snapshot
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
        let lease = self.acquire();
        lease.bucket.record(measurement);
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.rotate()
    }
}
