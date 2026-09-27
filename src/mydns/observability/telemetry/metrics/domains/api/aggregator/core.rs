trait MetricAggregator {
    type Measurement;
    type Snapshot;

    fn record(&self, measurement: Self::Measurement);
    fn snapshot(&self) -> Self::Snapshot;
}
