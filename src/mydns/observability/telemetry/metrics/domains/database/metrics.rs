//! Basic database metric aggregation.
//!
//! This is intentionally limited until the database measurement contract is finalized.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::core::{BoundedCounts, MergeableHistogram};
use super::observations::DatabaseOperationObservation;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseMetricsSnapshot {
    pub operations: u64,
    pub successes: u64,
    pub failures: u64,
    pub operation_counts: BoundedCounts,
    pub latency: MergeableHistogram,
}

pub struct DatabaseMetricsAggregator {
    state: Mutex<DatabaseMetricsSnapshot>,
}

impl DatabaseMetricsAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, observation: DatabaseOperationObservation<'_>) {
        let mut state = self.state.lock().unwrap();
        state.operations += 1;
        if observation.success {
            state.successes += 1;
        } else {
            state.failures += 1;
        }
        state.operation_counts.record(&observation.operation.trim().to_ascii_lowercase());
        state.latency.record(observation.latency_ms);
    }

    pub fn snapshot(&self) -> DatabaseMetricsSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DatabaseMetricsAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DatabaseMetricsSnapshot {
                operations: 0,
                successes: 0,
                failures: 0,
                operation_counts: BoundedCounts::default(),
                latency: MergeableHistogram::response(),
            }),
        }
    }
}
