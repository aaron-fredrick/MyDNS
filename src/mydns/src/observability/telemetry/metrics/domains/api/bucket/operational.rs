//! Mutable metric state for API operational aggregation.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

use chrono::{DateTime, Utc};

use crate::observability::telemetry::metrics::{
    domains::api::{
        measurements::ApiMeasurement,
        snapshot::ApiOperationalSnapshot,
    },
    types::{BoundedCounter, ScalarCounter, TypeTrait},
};

/// Mutable metric state for one API operational aggregation window.
///
/// Metric-level mutexes protect individual metric state. Bucket lifecycle
/// synchronization is separate and is coordinated by the owning aggregator.
pub struct ApiOperationalBucket {
    /// Start of this bucket's aggregation window.
    pub(crate) start_time: Mutex<DateTime<Utc>>,
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

// Lifecycle
impl ApiOperationalBucket {
    pub fn new() -> Self {
        Self {
            start_time: Mutex::new(Utc::now()),
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

        self.requests.lock().unwrap().reset();
        self.responses.lock().unwrap().reset();
        self.request_size_bytes.lock().unwrap().reset();
        self.response_size_bytes.lock().unwrap().reset();
        self.authentication_successes.lock().unwrap().reset();
        self.authentication_failures.lock().unwrap().reset();
        self.errors.lock().unwrap().reset();
        self.method_counts.lock().unwrap().reset();
        self.route_counts.lock().unwrap().reset();
        self.status_counts.lock().unwrap().reset();
        self.error_category_counts.lock().unwrap().reset();

        *self.start_time.lock().unwrap() = Utc::now();
        self.draining.store(false, Ordering::Release);
    }
}

// Recording
impl ApiOperationalBucket {
    pub fn record_request(&self, method: &str, route: &str) {
        self.requests.lock().unwrap().increment();
        self.method_counts.lock().unwrap().increment(method);
        self.route_counts.lock().unwrap().increment(route);
    }

    pub fn record_response(&self, status_code: u16) {
        self.responses.lock().unwrap().increment();
        self.status_counts
            .lock()
            .unwrap()
            .increment(&status_code.to_string());
    }

    pub fn record_request_size(&self, bytes: u64) {
        self.request_size_bytes
            .lock()
            .unwrap()
            .increment_by(bytes);
    }

    pub fn record_response_size(&self, bytes: u64) {
        self.response_size_bytes
            .lock()
            .unwrap()
            .increment_by(bytes);
    }

    pub fn record_authentication(&self, successful: bool) {
        if successful {
            self.authentication_successes.lock().unwrap().increment();
        } else {
            self.authentication_failures.lock().unwrap().increment();
        }
    }

    pub fn record_error(&self, category: &str) {
        self.errors.lock().unwrap().increment();
        self.error_category_counts
            .lock()
            .unwrap()
            .increment(category);
    }
}

/// Records an API measurement relevant to operational aggregation.
    ///
    /// This is the category-local dispatcher. The domain-level aggregator is
    /// still responsible for broadcasting the measurement to both categories.
    pub fn record(&self, measurement: ApiMeasurement<'_>) {
        match measurement {
            ApiMeasurement::Request { method, route } => {
                self.record_request(method, route);
            }
            ApiMeasurement::Response { status_code } => {
                self.record_response(status_code);
            }
            ApiMeasurement::RequestSize { bytes } => {
                self.record_request_size(bytes);
            }
            ApiMeasurement::ResponseSize { bytes } => {
                self.record_response_size(bytes);
            }
            ApiMeasurement::Authentication { successful } => {
                self.record_authentication(successful);
            }
            ApiMeasurement::Error { category } => {
                self.record_error(category);
            }
            _ => {}
        }
    }
}

// Snapshot
impl ApiOperationalBucket {
    /// Creates a snapshot of a detached, drained bucket.
    ///
    /// Rotation/draining is responsible for ensuring no recording operations
    /// remain before this method is called.
    pub fn snapshot(&self) -> ApiOperationalSnapshot {
        assert!(
            self.is_drained(),
            "bucket must be drained before snapshot"
        );

        ApiOperationalSnapshot {
            start_time: *self.start_time.lock().unwrap(),
            end_time: Utc::now(),
            requests: (&*self.requests.lock().unwrap()).into(),
            responses: (&*self.responses.lock().unwrap()).into(),
            request_size_bytes: (&*self.request_size_bytes.lock().unwrap()).into(),
            response_size_bytes: (&*self.response_size_bytes.lock().unwrap()).into(),
            authentication_successes: (&*self.authentication_successes.lock().unwrap()).into(),
            authentication_failures: (&*self.authentication_failures.lock().unwrap()).into(),
            errors: (&*self.errors.lock().unwrap()).into(),
            method_counts: (&*self.method_counts.lock().unwrap()).into(),
            route_counts: (&*self.route_counts.lock().unwrap()).into(),
            status_counts: (&*self.status_counts.lock().unwrap()).into(),
            error_category_counts: (&*self.error_category_counts.lock().unwrap()).into(),
        }
    }
}

impl Default for ApiOperationalBucket {
    fn default() -> Self {
        Self::new()
    }
}
