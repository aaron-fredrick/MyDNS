//! API-level composition of performance and operational aggregators.

use crate::observability::telemetry::metrics::aggregator::DomainAggregator;

use super::{ApiOperationalAggregator, ApiPerformanceAggregator};

pub type ApiAggregator = DomainAggregator<ApiPerformanceAggregator, ApiOperationalAggregator>;
