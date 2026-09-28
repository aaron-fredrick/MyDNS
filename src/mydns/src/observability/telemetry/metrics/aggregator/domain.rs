//! Generic composition for domain metric aggregators.

use crate::observability::telemetry::metrics::traits::CategoryAggregatorTrait;

/// Composes a domain's performance and operational metric aggregators.
pub struct DomainAggregator<P, O> {
    pub performance: P,
    pub operational: O,
}

impl<P, O> DomainAggregator<P, O> {
    pub fn new(performance: P, operational: O) -> Self {
        Self {
            performance,
            operational,
        }
    }
}

impl<P, O> DomainAggregator<P, O>
where
    P: CategoryAggregatorTrait,
    O: CategoryAggregatorTrait,
{
    pub fn record(&self, measurement: P::Measurement<'_>) {
        self.performance.record(measurement);
        self.operational.record(measurement);
    }
}
