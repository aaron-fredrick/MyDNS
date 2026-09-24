//! Database metric observations.

#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseOperationObservation<'a> {
    pub operation: &'a str,
    pub success: bool,
    pub latency_ms: f64,
}
