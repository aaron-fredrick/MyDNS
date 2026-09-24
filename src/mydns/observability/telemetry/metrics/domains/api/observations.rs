//! API metric observations.

#[derive(Debug, Clone, PartialEq)]
pub struct ApiRequestObservation<'a> {
    pub method: &'a str,
    pub route: &'a str,
    pub status_code: u16,
    pub latency_ms: f64,
}
