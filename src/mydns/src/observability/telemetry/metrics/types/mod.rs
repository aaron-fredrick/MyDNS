//! Reusable metric type implementations.

/// Resets a metric to its initial empty state.
pub trait TypeTrait {
    fn reset(&mut self);
}

pub trait SnapshotTypeTrait {
    type Snapshot;

    fn merge(&mut self, other: &Self) -> Self::Snapshot;
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
