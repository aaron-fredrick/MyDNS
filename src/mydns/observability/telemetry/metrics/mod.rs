//! Metrics telemetry.
//!
//! Existing metrics remain at observability::metrics for now.
//! This module is the future telemetry-facing abstraction boundary.

pub mod aggregator;
pub mod db;
pub mod persistence;
pub mod types;
