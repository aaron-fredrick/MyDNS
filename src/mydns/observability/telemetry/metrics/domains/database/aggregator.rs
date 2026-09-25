//! Domain-level aggregation for database measurements.
//!
//! This module maps database measurements into aggregated database metric state.
//! Time buckets, retention, and persistence remain outside the domain aggregator.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::core::{BoundedCounts, MergeableHistogram};

use super::measurements::{OperationalMeasurement, PerformanceMeasurement};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseAggregationSnapshot {
    // Operational
    pub operations: u64,
    pub operation_successes: u64,
    pub operation_failures: u64,
    pub connections: u64,
    pub connection_successes: u64,
    pub connection_failures: u64,
    pub operation_counts: BoundedCounts,

    // Performance
    pub operation_latency: MergeableHistogram,
    pub connection_latency: MergeableHistogram,
}

pub struct DatabaseAggregator {
    state: Mutex<DatabaseAggregationSnapshot>,
}

impl DatabaseAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_operational(&self, measurement: OperationalMeasurement<'_>) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            OperationalMeasurement::Operation {
                operation,
                successful,
            } => {
                state.operations += 1;
                state.operation_counts.record(&operation.trim().to_ascii_lowercase());

                if successful {
                    state.operation_successes += 1;
                } else {
                    state.operation_failures += 1;
                }
            }
            OperationalMeasurement::Connection { successful } => {
                state.connections += 1;

                if successful {
                    state.connection_successes += 1;
                } else {
                    state.connection_failures += 1;
                }
            }
        }
    }

    pub fn record_performance(&self, measurement: PerformanceMeasurement) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            PerformanceMeasurement::OperationLatency { latency_ms } => {
                state.operation_latency.record(latency_ms);
            }
            PerformanceMeasurement::ConnectionLatency { latency_ms } => {
                state.connection_latency.record(latency_ms);
            }
        }
    }

    pub fn snapshot(&self) -> DatabaseAggregationSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DatabaseAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DatabaseAggregationSnapshot {
                // Operational
                operations: 0,
                operation_successes: 0,
                operation_failures: 0,
                connections: 0,
                connection_successes: 0,
                connection_failures: 0,
                operation_counts: BoundedCounts::default(),

                // Performance
                operation_latency: MergeableHistogram::latency(),
                connection_latency: MergeableHistogram::latency(),
            }),
        }
    }
}
