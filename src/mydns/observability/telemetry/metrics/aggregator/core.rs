//! Shared contract for metric collection aggregators.

pub trait MetricAggregatorTrait {
    type Measurement;
    type Snapshot;

    fn record(&self, measurement: Self::Measurement);
    fn snapshot(&self) -> Self::Snapshot;
}
