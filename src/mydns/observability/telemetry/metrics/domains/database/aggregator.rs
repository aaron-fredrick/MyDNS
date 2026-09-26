//! Domain-level aggregation for database measurements.

use std::sync::{Arc, Mutex};

use crate::observability::telemetry::metrics::types::{BoundedCounter, MergeableHistogram};

use super::measurements::{OperationalMeasurement, PerformanceMeasurement};

const LATENCY_BOUNDS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseOperationalSnapshot {
    pub operations: u64,
    pub operation_successes: u64,
    pub operation_failures: u64,
    pub connections: u64,
    pub connection_successes: u64,
    pub connection_failures: u64,
    pub operation_counts: BoundedCounter,
}

pub struct DatabaseOperationalAggregator {
    state: Mutex<DatabaseOperationalSnapshot>,
}

impl DatabaseOperationalAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, measurement: OperationalMeasurement<'_>) {
        let mut state = self.state.lock().unwrap();

        match measurement {
            OperationalMeasurement::Operation { operation, successful } => {
                state.operations += 1;
                state.operation_counts.record(&operation.trim().to_ascii_lowercase());

                if successful { state.operation_successes += 1; }
                else { state.operation_failures += 1; }
            }
            OperationalMeasurement::Connection { successful } => {
                state.connections += 1;
                if successful { state.connection_successes += 1; }
                else { state.connection_failures += 1; }
            }
        }
    }

    pub fn snapshot(&self) -> DatabaseOperationalSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DatabaseOperationalAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DatabaseOperationalSnapshot {
                operations: 0,
                operation_successes: 0,
                operation_failures: 0,
                connections: 0,
                connection_successes: 0,
                connection_failures: 0,
                operation_counts: BoundedCounter::default(),
            }),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabasePerformanceSnapshot {
    pub operation_latency: MergeableHistogram,
    pub connection_latency: MergeableHistogram,
}

pub struct DatabasePerformanceAggregator {
    state: Mutex<DatabasePerformanceSnapshot>,
}

impl DatabasePerformanceAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, measurement: PerformanceMeasurement) {
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

    pub fn snapshot(&self) -> DatabasePerformanceSnapshot {
        self.state.lock().unwrap().clone()
    }
}

impl Default for DatabasePerformanceAggregator {
    fn default() -> Self {
        Self {
            state: Mutex::new(DatabasePerformanceSnapshot {
                operation_latency: MergeableHistogram::new(LATENCY_BOUNDS_MS),
                connection_latency: MergeableHistogram::new(LATENCY_BOUNDS_MS),
            }),
        }
    }
}
