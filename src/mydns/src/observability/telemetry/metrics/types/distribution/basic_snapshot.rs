//! Detached snapshot of a distribution metric.

use serde::{Deserialize, Serialize};

use super::super::{HistogramSnapshot, TDigestSummarySnapshot};

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
