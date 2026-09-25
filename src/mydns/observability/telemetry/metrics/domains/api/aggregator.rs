//! Domain-level aggregation for API measurements.
//!
//! This module maps API measurements into aggregated API metric state.
//! Time buckets, retention, and persistence remain outside the domain aggregator.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::core::{BoundedCounts, MergeableHistogram};

use super::measurements::{OperationalMeasurement, PerformanceMeasurement};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiAggregationSnapshot {
    pub requests: u64,
    pub responses: u64,
    pub authentication_successes: u64,
    pub authentication_failures: u64,
    pub errors: u64,
    pub method_counts: BoundedCounts,
    pub route_counts: BoundedCounts,
    pub status_counts: BoundedCounts,
    pub error_category_counts: BoundedCounts,
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
                requests: 0,
                responses: 0,
                authentication_successes: 0,
                authentication_failures: 0,
                errors: 0,
                method_counts: BoundedCounts::default(),
                route_counts: BoundedCounts::default(),
                status_counts: BoundedCounts::default(),
                error_category_counts: BoundedCounts::default(),
                request_latency: MergeableHistogram::latency(),
                handler_latency: MergeableHistogram::latency(),
            }),
        }
    }
}
