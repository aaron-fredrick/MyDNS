//! Reusable metric type implementations.
//!
//! Metric types describe how observations are represented and aggregated.
//! Domain aggregators decide what each metric measures.

pub mod counter;
pub use counter::{BoundedCounts, ScalarCounter};

pub mod gauge;
pub use gauge::Gauge;

pub mod histogram;
pub use histogram::MergeableHistogram;

pub mod summary;
pub use summary::Summary;
