//! Measurements produced by the database domain.
//!
//! These definitions describe the database measurement contract. They are
//! intentionally independent from aggregation, time buckets, retention,
//! and persistence.

#[derive(Debug, Clone, PartialEq)]
pub enum OperationalMeasurement<'a> {
    Operation {
        operation: &'a str,
        successful: bool,
    },
    Connection {
        successful: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceMeasurement {
    OperationLatency { latency_ms: f64 },
    ConnectionLatency { latency_ms: f64 },
}
