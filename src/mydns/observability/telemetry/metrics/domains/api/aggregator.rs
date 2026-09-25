//! Domain-level aggregation for API measurements.
//!
//! This module maps API measurements into aggregated API metric state.
//! Time buckets, retention, and persistence remain outside the domain aggregator.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::types::{BoundedCounts, MergeableHistogram};

use super::measurements::{OperationalMeasurement, PerformanceMeasurement};

const LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiAggregationSnapshot {
    // Operational
    pub requests: u64,
    pub responses: u64,
    pub authentication_successes: u64,
    pub authentication_failures: u64,
    pub errors: u64,
    pub method_counts: BoundedCounts,
    pub route_counts: BoundedCounts,
    pub status_counts: BoundedCounts,
    pub error_category_counts: BoundedCounts,

    // Performance
    pub request_latency: MergeableHistogram,
    pub handler_latency: MergeableHistogram,
}

pub struct ApiAggregator {
    state: Mutex<ApiAggregationSnapshot>,
}

impl ApiAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_operational(&self, measurement: OperationalMeasurement<'_>) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            OperationalMeasurement::Request { method, route } => {
                state.requests += 1;
                state.method_counts.record(&method.trim().to_ascii_uppercase());
                state.route_counts.record(route.trim());
            }
            OperationalMeasurement::Response { status_code } => {
                state.responses += 1;
                state.status_counts.record(&status_code.to_string());
            }
            OperationalMeasurement::Authentication { successful } => {
                if successful {
                    state.authentication_successes += 1;
                } else {
                    state.authentication_failures += 1;
                }
            }
            OperationalMeasurement::Error { category } => {
                state.errors += 1;
                state.error_category_counts.record(category);
            }
        }
    }

    pub fn record_performance(&self, measurement: PerformanceMeasurement) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            PerformanceMeasurement::RequestLatency { latency_ms } => {
                state.request_latency.record(latency_ms);
            }
            PerformanceMeasurement::HandlerLatency { latency_ms } => {
                state.handler_latency.record(latency_ms);
            }
        }
    }

    pub fn snapshot(&self) -> ApiAggregationSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for ApiAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(ApiAggregationSnapshot {
                // Operational
                requests: 0,
                responses: 0,
                authentication_successes: 0,
                authentication_failures: 0,
                errors: 0,
                method_counts: BoundedCounts::default(),
                route_counts: BoundedCounts::default(),
                status_counts: BoundedCounts::default(),
                error_category_counts: BoundedCounts::default(),

                // Performance
                request_latency: MergeableHistogram::new(LATENCY_BOUNDS_MS),
                handler_latency: MergeableHistogram::new(LATENCY_BOUNDS_MS),
            }),
        }
    }
}
