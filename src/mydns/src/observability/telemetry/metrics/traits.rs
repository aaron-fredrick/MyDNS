//! Shared contracts for metric collection.

pub trait CategoryAggregatorTrait {
    type Measurement<'a>
    where
        Self: 'a;
    type Snapshot;

    fn record(&self, measurement: Self::Measurement<'_>);
    fn snapshot(&self) -> Self::Snapshot;
    fn reset(&mut self);
}
