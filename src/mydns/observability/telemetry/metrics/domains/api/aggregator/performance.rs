//! Performance aggregation for API measurements.
use crate::observability::telemetry::metrics::{
    domains::api::{measurements::ApiPerformanceMeasurement, snapshot::ApiPerformanceSnapshot},
    types::{DistributionMetrics, Gauge, ScalarCounter},
};
use std::sync::Mutex;
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
    request_latency: Mutex<DistributionMetrics>,
    handler_latency: Mutex<DistributionMetrics>,
    request_count: Mutex<ScalarCounter>,
    request_concurrency: Mutex<Gauge>,
    request_size: Mutex<DistributionMetrics>,
    response_size: Mutex<DistributionMetrics>,
}
impl ApiPerformanceAggregator {
    pub fn new() -> Self {
        Self {
            request_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            handler_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            request_count: Mutex::new(ScalarCounter::new()),
            request_concurrency: Mutex::new(Gauge::default()),
            request_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
            response_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
        }
    }
    pub fn record(&self, m: ApiPerformanceMeasurement) {
        match m {
            ApiPerformanceMeasurement::RequestLatency { latency_ms } => {
                self.record_request_latency(latency_ms)
            }
            ApiPerformanceMeasurement::HandlerLatency { latency_ms } => {
                self.record_handler_latency(latency_ms)
            }
            ApiPerformanceMeasurement::RequestCount => self.record_request_count(),
            ApiPerformanceMeasurement::RequestConcurrency { active_requests } => {
                self.record_request_concurrency(active_requests)
            }
            ApiPerformanceMeasurement::RequestSize { bytes } => self.record_request_size(bytes),
            ApiPerformanceMeasurement::ResponseSize { bytes } => self.record_response_size(bytes),
        }
    }
    pub fn snapshot(&self) -> ApiPerformanceSnapshot {
        ApiPerformanceSnapshot {
            request_latency: self.request_latency.lock().unwrap().clone().into(),
            handler_latency: self.handler_latency.lock().unwrap().clone().into(),
            request_count: (&*self.request_count.lock().unwrap()).into(),
            request_concurrency: (&*self.request_concurrency.lock().unwrap()).into(),
            request_size: self.request_size.lock().unwrap().clone().into(),
            response_size: self.response_size.lock().unwrap().clone().into(),
        }
    }
    fn record_request_latency(&self, v: f64) {
        self.request_latency.lock().unwrap().record(v)
    }
    fn record_handler_latency(&self, v: f64) {
        self.handler_latency.lock().unwrap().record(v)
    }
    fn record_request_count(&self) {
        self.request_count.lock().unwrap().increment()
    }
    fn record_request_concurrency(&self, v: u64) {
        self.request_concurrency.lock().unwrap().set(v as f64)
    }
    fn record_request_size(&self, v: u64) {
        self.request_size.lock().unwrap().record(v as f64)
    }
    fn record_response_size(&self, v: u64) {
        self.response_size.lock().unwrap().record(v as f64)
    }
}
impl Default for ApiPerformanceAggregator {
    fn default() -> Self {
        Self::new()
    }
}
