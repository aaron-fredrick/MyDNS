//! Basic API metric aggregation.
//!
//! This is intentionally limited until the API measurement contract is finalized.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::core::{BoundedCounts, MergeableHistogram};
use super::observations::ApiRequestObservation;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiMetricsSnapshot {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub method_counts: BoundedCounts,
    pub route_counts: BoundedCounts,
    pub status_counts: BoundedCounts,
    pub latency: MergeableHistogram,
}

pub struct ApiMetricsAggregator {
    state: Mutex<ApiMetricsSnapshot>,
}

impl ApiMetricsAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ApiMetricsSnapshot {
                requests: 0,
                successes: 0,
                failures: 0,
                method_counts: BoundedCounts::default(),
                route_counts: BoundedCounts::default(),
                status_counts: BoundedCounts::default(),
                latency: MergeableHistogram::latency(),
            }),
        })
    }

    pub fn record(&self, observation: ApiRequestObservation<'_>) {
        let mut state = self.state.lock().unwrap();
        state.requests += 1;
        if observation.status_code < 500 {
            state.successes += 1;
        } else {
            state.failures += 1;
        }
        state.method_counts.record(&observation.method.trim().to_ascii_uppercase());
        state.route_counts.record(observation.route.trim());
        state.status_counts.record(&observation.status_code.to_string());
        state.latency.record(observation.latency_ms);
    }

    pub fn snapshot(&self) -> ApiMetricsSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for ApiMetricsAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(ApiMetricsSnapshot {
                requests: 0,
                successes: 0,
                failures: 0,
                method_counts: BoundedCounts::default(),
                route_counts: BoundedCounts::default(),
                status_counts: BoundedCounts::default(),
                latency: MergeableHistogram::response(),
            }),
        }
    }
}
