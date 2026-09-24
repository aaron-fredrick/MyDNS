//! Database metric domain.
//!
//! The initial implementation records operation volume, success/failure,
//! operation dimensions, and latency. Detailed database semantics can be
//! expanded after instrumentation is finalized.

pub mod metrics;
pub mod observations;

pub use metrics::{DatabaseMetricsAggregator, DatabaseMetricsSnapshot};
pub use observations::DatabaseOperationObservation;
