//! Reusable distribution metric aggregation.

use serde::{Deserialize, Serialize};

use super::super::{Histogram, TDigestSummary};
use super::basic_snapshot::DistributionSnapshot;
use crate::observability::telemetry::metrics::types::TypeTrait;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionMetrics {
    histogram: Histogram,
    summary: TDigestSummary,
}

impl DistributionMetrics {
    pub fn new(bounds: &[f64]) -> Self {
        Self {
            histogram: Histogram::new(bounds),
            summary: TDigestSummary::new(),
        }
    }

    pub fn record(&mut self, value: f64) {
        self.histogram.record(value);
        self.summary.record(value);
    }
}

impl TypeTrait for DistributionMetrics {
    fn reset(&mut self) {
        self.histogram.reset();
        self.summary.reset();
    }
}

impl From<&DistributionMetrics> for DistributionSnapshot {
    fn from(metric: &DistributionMetrics) -> Self {
        Self {
            histogram: (&metric.histogram).into(),
            summary: (&metric.summary).into(),
        }
    }
}
