//! Measurements produced by the API domain.
//!
//! These definitions describe the API measurement contract. They are intentionally
//! independent from aggregation, time buckets, retention, and persistence.

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceMeasurement {
    RequestLatency { latency_ms: f64 },
    HandlerLatency { latency_ms: f64 },

    RequestFrequency,

    RequestConcurrency { active_requests: u64 },
    
    RequestSize { bytes: u64 },
    ResponseSize { bytes: u64 },
}


#[derive(Debug, Clone, PartialEq)]
pub enum OperationalMeasurement<'a> {
    Request { method: &'a str, route: &'a str },
    Response { status_code: u16 },
    Authentication { successful: bool },
    Error { category: &'a str },
}