//! API-level composition of performance and operational aggregators.

use crate::observability::telemetry::metrics::{
    aggregator::DomainAggregator,
    domains::api::measurements::ApiMeasurementFamily,
};

use super::{ApiOperationalAggregator, ApiPerformanceAggregator};

pub type ApiAggregator =
    DomainAggregator<ApiMeasurementFamily, ApiPerformanceAggregator, ApiOperationalAggregator>;
