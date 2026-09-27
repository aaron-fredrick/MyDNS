//! Reusable distribution metric aggregation.

use serde::{Deserialize, Serialize};

use super::{histogram::snapshot::HistogramSnapshot, summary::snapshot::TDigestSummarySnapshot};
use super::{Histogram, TDigestSummary};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionMetrics {
    pub histogram: Histogram,
    pub summary: TDigestSummary,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionSnapshot {
    pub histogram: HistogramSnapshot,
    pub summary: TDigestSummarySnapshot,
}

impl DistributionSnapshot {
    pub fn merge(&mut self, other: &Self) {
        self.histogram.merge(&other.histogram);
        self.summary.merge(&other.summary);
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
