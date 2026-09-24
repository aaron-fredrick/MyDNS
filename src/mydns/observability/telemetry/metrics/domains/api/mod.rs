//! API metric domain.
//!
//! The initial implementation records request volume, status classes, dimensions,
//! and latency. The detailed measurement contract can be expanded after API
//! instrumentation semantics are finalized.

pub mod metrics;
pub mod observations;

pub use metrics::{ApiMetricsAggregator, ApiMetricsSnapshot};
pub use observations::ApiRequestObservation;
