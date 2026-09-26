//! Performance aggregation for API measurements.

use std::sync::Mutex;

use crate::observability::telemetry::metrics::{
    domains::api::measurements::ApiPerformanceMeasurement,
    types::{DistributionMetrics, Gauge, ScalarCounter},
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

pub struct ApiPerformanceAggregator {
    pub request_latency: Mutex<DistributionMetrics>,
    pub handler_latency: Mutex<DistributionMetrics>,

    // AtomicU64 is a better fit for this scalar counter; Mutex is used here
    // until the aggregator's atomic synchronization strategy is introduced.
    pub request_frequency: Mutex<ScalarCounter>,

    pub request_concurrency: Mutex<Gauge>,

    pub request_size: Mutex<DistributionMetrics>,
    pub response_size: Mutex<DistributionMetrics>,
}

impl ApiPerformanceAggregator {
    pub fn new() -> Self {
        Self {
            request_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            handler_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            request_frequency: Mutex::new(ScalarCounter::new()),
            request_concurrency: Mutex::new(Gauge::default()),
            request_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
            response_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
        }
    }

    pub fn record(&self, measurement: ApiPerformanceMeasurement) {
        match measurement {
            ApiPerformanceMeasurement::RequestLatency { latency_ms } => {
                self.record_request_latency(latency_ms);
            }
            ApiPerformanceMeasurement::HandlerLatency { latency_ms } => {
                self.record_handler_latency(latency_ms);
            }
            ApiPerformanceMeasurement::RequestFrequency => {
                self.record_request_frequency();
            }
            ApiPerformanceMeasurement::RequestConcurrency { active_requests } => {
                self.record_request_concurrency(active_requests);
            }
            ApiPerformanceMeasurement::RequestSize { bytes } => {
                self.record_request_size(bytes);
            }
            ApiPerformanceMeasurement::ResponseSize { bytes } => {
                self.record_response_size(bytes);
            }
        }
    }

    fn record_request_latency(&self, latency_ms: f64) {
        let mut metric = self.request_latency.lock().unwrap();
        metric.record(latency_ms);
    }

    fn record_handler_latency(&self, latency_ms: f64) {
        let mut metric = self.handler_latency.lock().unwrap();
        metric.record(latency_ms);
    }

    fn record_request_frequency(&self) {
        let mut metric = self.request_frequency.lock().unwrap();
        metric.increment();
    }

    fn record_request_concurrency(&self, active_requests: u64) {
        let mut metric = self.request_concurrency.lock().unwrap();
        metric.set(active_requests as f64);
    }

    fn record_request_size(&self, bytes: u64) {
        let mut metric = self.request_size.lock().unwrap();
        metric.record(bytes as f64);
    }

    fn record_response_size(&self, bytes: u64) {
        let mut metric = self.response_size.lock().unwrap();
        metric.record(bytes as f64);
    }
}
