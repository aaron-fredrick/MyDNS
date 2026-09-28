//! Generic composition for domain metric aggregators.

use std::marker::PhantomData;

use crate::observability::telemetry::metrics::traits::{
    CategoryAggregatorTrait, MeasurementFamily,
};

/// Composes a domain's shared measurement contract with its category aggregators.
///
/// The measurement family belongs to the domain composition rather than to an
/// individual category. This keeps domain producers independent from the
/// performance/operational split.
pub struct DomainAggregator<M, P, O> {
    pub performance: P,
    pub operational: O,
    _measurement: PhantomData<fn() -> M>,
}

impl<M, P, O> DomainAggregator<M, P, O>
where
    M: MeasurementFamily,
{
    pub fn new(performance: P, operational: O) -> Self {
        Self {
            performance,
            operational,
            _measurement: PhantomData,
        }
    }
}

impl<M, P, O> DomainAggregator<M, P, O>
where
    M: MeasurementFamily,
    P: CategoryAggregatorTrait<M>,
    O: CategoryAggregatorTrait<M>,
{
    /// Broadcasts one domain measurement to every category aggregator.
    ///
    /// Each category decides which measurements are relevant to it.
    pub fn record(&self, measurement: M::Measurement<'_>) {
        self.performance.record(measurement.clone());
        self.operational.record(measurement);
    }
}
