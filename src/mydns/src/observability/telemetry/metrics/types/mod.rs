//! Reusable metric type implementations.

/// Common behavior for live metric types.
pub trait TypeTrait {
    fn reset(&mut self);
}

/// Common behavior for detached metric snapshot types.
pub trait SnapshotTypeTrait {
    fn merge(&mut self, other: &Self);
}

pub mod counter;
pub use counter::{BoundedCounter, BoundedCounterSnapshot, ScalarCounter, ScalarCounterSnapshot};
pub mod distribution;
pub use distribution::{DistributionMetrics, DistributionSnapshot};
pub mod gauge;
pub use gauge::{Gauge, GaugeSnapshot};
pub mod histogram;
pub use histogram::{Histogram, HistogramSnapshot};
pub mod summary;
pub use summary::{TDigestSummary, TDigestSummarySnapshot};
