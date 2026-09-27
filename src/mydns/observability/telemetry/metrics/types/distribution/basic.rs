//! Reusable distribution metric aggregation.

use serde::{Deserialize, Serialize};

use super::{Histogram, TDigestSummary};
use super::basic_snapshot::DistributionSnapshot;

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

impl From<&DistributionMetrics> for DistributionSnapshot {
    fn from(metric: &DistributionMetrics) -> Self {
        Self {
            histogram: (&metric.histogram).into(),
            summary: (&metric.summary).into(),
        }
    }
}
