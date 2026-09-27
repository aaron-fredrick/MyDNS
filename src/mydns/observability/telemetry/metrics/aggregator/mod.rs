//! Metric aggregation primitives.

mod core;
pub use core::MetricAggregatorTrait;

pub mod bucket;
pub use bucket::MetricBucket;

pub mod period;
pub use period::MetricPeriod;
