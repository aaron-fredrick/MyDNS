//! Measurements produced by the DNS domain.
//!
//! The measurements below mirror the measurement dimensions already represented
//! by the DNS metrics implementation. They are definitions only; aggregation,
//! bucket assignment, retention, and persistence remain separate concerns.

#[derive(Debug, Clone, PartialEq)]
pub enum OperationalMeasurement<'a> {
    Query {
        record_type: &'a str,
        transport: &'a str,
    },
    Response {
        response_code: &'a str,
    },
    Blocked {
        reason: &'a str,
    },
    Resolution {
        outcome: &'a str,
        path: &'a str,
    },
    Cache {
        hit: bool,
    },
    CacheEviction,
    Upstream {
        successful: bool,
        timeout: bool,
        retry: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceMeasurement {
    ResponseLatency { latency_ms: f64 },
    UpstreamLatency { latency_ms: f64 },
}
