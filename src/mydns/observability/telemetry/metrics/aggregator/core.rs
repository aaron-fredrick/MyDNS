//! Shared contract for metric collection aggregators.

pub trait MetricAggregatorTrait {
    type Measurement<'a>
    where
        Self: 'a;
    type Snapshot;

    fn record(&self, measurement: Self::Measurement<'_>);
    fn snapshot(&self) -> Self::Snapshot;
}
