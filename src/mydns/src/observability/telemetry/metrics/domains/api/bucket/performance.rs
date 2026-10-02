//! Mutable metric state for API performance aggregation.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

use chrono::{DateTime, Utc};

use crate::observability::telemetry::metrics::{
    domains::api::{
        measurements::ApiMeasurement,
        snapshot::ApiPerformanceSnapshot,
    },
    types::{DistributionMetrics, Gauge, ScalarCounter, TypeTrait},
};

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
/// Metric-level mutexes protect individual metric state. Bucket lifecycle
/// synchronization is separate and is coordinated by the owning aggregator.
pub struct ApiPerformanceBucket {
    /// Start of this bucket's aggregation window.
    pub(crate) start_time: Mutex<DateTime<Utc>>,
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

// Lifecycle
impl ApiPerformanceBucket {
    pub fn new() -> Self {
        Self {
            start_time: Mutex::new(Utc::now()),
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

    /// Attempts to acquire a recording lease.
    ///
    /// A lease acquired while draining has begun is immediately rejected and
    /// released. The owning aggregator must coordinate this operation with
    /// bucket rotation so that the active-bucket swap and lease acquisition
    /// cannot race.
    pub fn try_acquire(&self) -> bool {
        self.users.fetch_add(1, Ordering::Acquire);

        if self.draining.load(Ordering::Acquire) {
            self.users.fetch_sub(1, Ordering::Release);
            false
        } else {
            true
        }
    }

    /// Releases one recording lease.
    pub fn release(&self) {
        let previous = self.users.fetch_sub(1, Ordering::Release);
        debug_assert!(previous > 0, "bucket recording lease underflow");
    }

    /// Prevents new recording leases from being acquired.
    pub fn begin_draining(&self) {
        self.draining.store(true, Ordering::Release);
    }

    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::Acquire)
    }

    pub fn users(&self) -> usize {
        self.users.load(Ordering::Acquire)
    }

    pub fn is_drained(&self) -> bool {
        self.users() == 0
    }

    /// Clears all metric state and makes the bucket available for reuse.
    ///
    /// The bucket must be detached and fully drained before it is reset.
    pub fn reset(&self) {
        assert!(
            self.is_drained(),
            "bucket must be drained before reset"
        );

        self.request_latency.lock().unwrap().reset();
        self.handler_latency.lock().unwrap().reset();
        self.request_count.lock().unwrap().reset();
        self.request_concurrency.lock().unwrap().reset();
        self.request_size.lock().unwrap().reset();
        self.response_size.lock().unwrap().reset();

        *self.start_time.lock().unwrap() = Utc::now();
        self.draining.store(false, Ordering::Release);
    }
}

// Recording
impl ApiPerformanceBucket {
    pub fn record_request_latency(&self, value: f64) {
        self.request_latency.lock().unwrap().record(value);
    }

    pub fn record_handler_latency(&self, value: f64) {
        self.handler_latency.lock().unwrap().record(value);
    }

    pub fn record_request_count(&self) {
        self.request_count.lock().unwrap().increment();
    }

    pub fn record_request_concurrency(&self, value: u64) {
        self.request_concurrency.lock().unwrap().set(value as f64);
    }

    pub fn record_request_size(&self, value: u64) {
        self.request_size.lock().unwrap().record(value as f64);
    }

    pub fn record_response_size(&self, value: u64) {
        self.response_size.lock().unwrap().record(value as f64);
    }

/// Records an API measurement relevant to performance aggregation.
    ///
    /// This is the category-local dispatcher. The domain-level aggregator is
    /// still responsible for broadcasting the measurement to both categories.
    pub fn record(&self, measurement: ApiMeasurement<'_>) {
        match measurement {
            ApiMeasurement::RequestLatency { latency_ms } => {
                self.record_request_latency(latency_ms);
                self.record_request_count();
            }
            ApiMeasurement::HandlerLatency { latency_ms } => {
                self.record_handler_latency(latency_ms);
            }
            ApiMeasurement::RequestConcurrency { active_requests } => {
                self.record_request_concurrency(active_requests);
            }
            ApiMeasurement::RequestSize { bytes } => {
                self.record_request_size(bytes);
            }
            ApiMeasurement::ResponseSize { bytes } => {
                self.record_response_size(bytes);
            }
            _ => {}
        }
    }
}

// Snapshot
impl ApiPerformanceBucket {
    /// Creates a snapshot of a detached, drained bucket.
    ///
    /// Rotation/draining is responsible for ensuring no recording operations
    /// remain before this method is called.
    pub fn snapshot(&self) -> ApiPerformanceSnapshot {
        assert!(
            self.is_drained(),
            "bucket must be drained before snapshot"
        );

        ApiPerformanceSnapshot {
            start_time: *self.start_time.lock().unwrap(),
            end_time: Utc::now(),
            request_latency: (&*self.request_latency.lock().unwrap()).into(),
            handler_latency: (&*self.handler_latency.lock().unwrap()).into(),
            request_count: (&*self.request_count.lock().unwrap()).into(),
            request_concurrency: (&*self.request_concurrency.lock().unwrap()).into(),
            request_size: (&*self.request_size.lock().unwrap()).into(),
            response_size: (&*self.response_size.lock().unwrap()).into(),
        }
    }
}

impl Default for ApiPerformanceBucket {
    fn default() -> Self {
        Self::new()
    }
}
