//! DNS metric domain.
///
/// DNS-specific observations, aggregation state, and metric representations
/// live here. Shared metric mechanics remain in the metrics core.

pub mod aggregator;
pub mod types;

pub use aggregator::MetricsAggregator as DnsMetricsAggregator;
