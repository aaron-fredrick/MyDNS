//! Shared contracts for metric collection.

/// Defines the measurement type family owned by a metric domain.
///
/// Rust cannot pass a lifetime-parameterized type directly as a generic type
/// parameter, so domains expose that family through this GAT.
pub trait MeasurementFamily {
    type Measurement<'a>: Clone
    where
        Self: 'a;
}

pub trait CategoryAggregatorTrait<M>
where
    M: MeasurementFamily,
{
    type Snapshot;

    fn record(&self, measurement: M::Measurement<'_>);
    fn snapshot(&self) -> Self::Snapshot;
}

pub trait CategorySnapshotTrait {
    fn merge(&mut self, other: &Self);
}
