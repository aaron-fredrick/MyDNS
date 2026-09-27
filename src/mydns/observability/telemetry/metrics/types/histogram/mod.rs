//! Histogram metric types.
pub mod fixed;
pub mod snapshot;
pub use fixed::Histogram;
pub use snapshot::HistogramSnapshot;
