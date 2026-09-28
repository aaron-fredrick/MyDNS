//! Metric aggregation primitives.

pub mod bucket;
pub use bucket::MetricBucket;

pub mod domain;
pub use domain::DomainAggregator;

pub mod period;
pub use period::MetricPeriod;
