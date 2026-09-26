//! DNS metric domain.

pub mod aggregator;
pub mod measurements;

pub use aggregator::{DnsOperationalAggregator, DnsPerformanceAggregator};
