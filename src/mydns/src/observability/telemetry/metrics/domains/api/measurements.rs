//! Measurements produced by the API domain.
//!
//! These definitions describe the API measurement contract. They are intentionally
//! independent from aggregation, time buckets, retention, and persistence.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiMeasurement<'a> {
    Request { method: &'a str, route: &'a str },
    Response { status_code: u16 },

    RequestLatency { latency_ms: f64 },
    HandlerLatency { latency_ms: f64 },

    RequestSize { bytes: u64 },
    ResponseSize { bytes: u64 },

    RequestConcurrency { active_requests: u64 },

    Authentication { successful: bool },
    Error { category: &'a str },
}

// TODO: Revisit measurement ownership/lifetimes when the asynchronous metric
// collection pipeline is implemented. Borrowed values are appropriate while
// measurements are consumed synchronously by the aggregators.
