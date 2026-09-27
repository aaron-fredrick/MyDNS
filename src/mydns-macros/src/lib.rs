//! Procedural macros used by the MyDNS workspace.
//!
//! This crate is intentionally kept separate from the runtime mydns crate.
//! Macro implementations provide compile-time structural contracts for the
//! runtime observability infrastructure.

mod metric_category_aggregator;

pub use metric_category_aggregator::metric_category_aggregator;
