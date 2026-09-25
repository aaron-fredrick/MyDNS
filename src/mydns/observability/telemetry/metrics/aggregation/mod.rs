//! Temporal aggregation primitives used by metric storage and rollups.

pub mod bucket;
pub use bucket::MetricBucket;

pub mod period;
pub use period::MetricPeriod;
