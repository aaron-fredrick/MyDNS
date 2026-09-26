//! Operational aggregation for API measurements.

use std::sync::Mutex;

use crate::observability::telemetry::metrics::{
    domains::api::measurements::ApiOperationalMeasurement,
    types::{BoundedCounter, ScalarCounter},
};

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

    pub fn record(&self, measurement: ApiOperationalMeasurement<'_>) {
        match measurement {
            ApiOperationalMeasurement::Request { method, route } => {
                self.record_request(method, route);
            }
            ApiOperationalMeasurement::Response { status_code } => {
                self.record_response(status_code);
            }
            ApiOperationalMeasurement::RequestSize { bytes } => {
                self.record_request_size(bytes);
            }
            ApiOperationalMeasurement::ResponseSize { bytes } => {
                self.record_response_size(bytes);
            }
            ApiOperationalMeasurement::Authentication { successful } => {
                self.record_authentication(successful);
            }
            ApiOperationalMeasurement::Error { category } => {
                self.record_error(category);
            }
        }
    }

    fn record_request(&self, method: &str, route: &str) {
        self.requests.lock().unwrap().increment();

        // BoundedCounter's key/update API is kept behind this method so its
        // concrete storage semantics remain isolated from measurement dispatch.
        self.method_counts.lock().unwrap().increment(method);
        self.route_counts.lock().unwrap().increment(route);
    }

    fn record_response(&self, status_code: u16) {
        self.responses.lock().unwrap().increment();
        self.status_counts
            .lock()
            .unwrap()
            .increment(&status_code.to_string());
    }

    fn record_request_size(&self, bytes: u64) {
        self.request_size_bytes.lock().unwrap().increment_by(bytes);
    }

    fn record_response_size(&self, bytes: u64) {
        self.response_size_bytes.lock().unwrap().increment_by(bytes);
    }

    fn record_authentication(&self, successful: bool) {
        if successful {
            self.authentication_successes.lock().unwrap().increment();
        } else {
            self.authentication_failures.lock().unwrap().increment();
        }
    }

    fn record_error(&self, category: &str) {
        self.errors.lock().unwrap().increment();
        self.error_category_counts
            .lock()
            .unwrap()
            .increment(category);
    }
}
