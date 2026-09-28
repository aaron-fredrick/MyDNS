//! Performance aggregation for API measurements.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use mydns_macros::metric_category_aggregator;

use crate::observability::telemetry::metrics::{
    domains::api::{
        measurements::{ApiMeasurement, ApiMeasurementFamily},
        snapshot::ApiPerformanceSnapshot,
    },
    traits::CategoryAggregatorTrait,
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

#[metric_category_aggregator]
pub struct ApiPerformanceAggregator {
    start_time: DateTime<Utc>,
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
            start_time: Utc::now(),
            request_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            handler_latency: Mutex::new(DistributionMetrics::new(LATENCY_BOUNDS_MS)),
            request_count: Mutex::new(ScalarCounter::new()),
            request_concurrency: Mutex::new(Gauge::default()),
            request_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
            response_size: Mutex::new(DistributionMetrics::new(SIZE_BOUNDS_BYTES)),
        }
    }

    fn record_request_latency(&self, value: f64) {
        self.request_latency.lock().unwrap().record(value);
    }
    fn record_handler_latency(&self, value: f64) {
        self.handler_latency.lock().unwrap().record(value);
    }
    fn record_request_count(&self) {
        self.request_count.lock().unwrap().increment();
    }
    fn record_request_concurrency(&self, value: u64) {
        self.request_concurrency.lock().unwrap().set(value as f64);
    }
    fn record_request_size(&self, value: u64) {
        self.request_size.lock().unwrap().record(value as f64);
    }
    fn record_response_size(&self, value: u64) {
        self.response_size.lock().unwrap().record(value as f64);
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
        match measurement {
            ApiMeasurement::RequestLatency { latency_ms } => {
                self.record_request_latency(latency_ms);
                self.record_request_count();
            }
            ApiMeasurement::HandlerLatency { latency_ms } => {
                self.record_handler_latency(latency_ms)
            }
            ApiMeasurement::RequestConcurrency { active_requests } => {
                self.record_request_concurrency(active_requests)
            }
            ApiMeasurement::RequestSize { bytes } => self.record_request_size(bytes),
            ApiMeasurement::ResponseSize { bytes } => self.record_response_size(bytes),
            _ => {}
        }
    }

    fn snapshot(&self) -> Self::Snapshot {
        let end_time = Utc::now();

        ApiPerformanceSnapshot {
            start_time: self.start_time,
            end_time,
            request_latency: (&*self.request_latency.lock().unwrap()).into(),
            handler_latency: (&*self.handler_latency.lock().unwrap()).into(),
            request_count: (&*self.request_count.lock().unwrap()).into(),
            request_concurrency: (&*self.request_concurrency.lock().unwrap()).into(),
            request_size: (&*self.request_size.lock().unwrap()).into(),
            response_size: (&*self.response_size.lock().unwrap()).into(),
        }
    }

    fn reset(&mut self) {
        self.start_time = Utc::now();
        self.request_latency.get_mut().unwrap().reset();
        self.handler_latency.get_mut().unwrap().reset();
        self.request_count.get_mut().unwrap().reset();
        self.request_concurrency.get_mut().unwrap().reset();
        self.request_size.get_mut().unwrap().reset();
        self.response_size.get_mut().unwrap().reset();
    }
}
