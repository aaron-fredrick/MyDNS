//! Reusable distribution metric aggregation.

use serde::{Deserialize, Serialize};

use super::{Histogram, TDigestSummary};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionMetrics {
    pub histogram: Histogram,
    pub summary: TDigestSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionSnapshot {
    pub histogram: Histogram,
    pub summary: TDigestSummary,
}

impl From<&DistributionMetrics> for DistributionSnapshot {
    fn from(metric: &DistributionMetrics) -> Self {
        Self {
            histogram: metric.histogram.clone(),
            summary: metric.summary.clone(),
        }
    }
}

impl From<DistributionMetrics> for DistributionSnapshot {
    fn from(metric: DistributionMetrics) -> Self {
        Self {
            histogram: metric.histogram,
            summary: metric.summary,
        }
    }
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

    pub fn merge(&mut self, other: &Self) {
        self.histogram.merge(&other.histogram);
        self.summary.merge(&other.summary);
    }
}
