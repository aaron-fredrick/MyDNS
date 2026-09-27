//! Histogram metric types.
pub mod fixed;
pub mod fixed_snapshot;
pub use fixed::Histogram;
pub use fixed_snapshot::HistogramSnapshot;
