//! Detached snapshot of a t-digest statistical summary.

use serde::{Deserialize, Serialize};
use tdigest::TDigest;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TDigestSummarySnapshot {
    pub count: u64,
    pub sum: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    digest: TDigest,
}

impl TDigestSummarySnapshot {
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.count == 0 || !q.is_finite() || !(0.0..=1.0).contains(&q) {
            return None;
        }

        Some(self.digest.estimate_quantile(q))
    }

    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
    }

    pub fn merge(&mut self, other: &Self) {
        self.count = self.count.saturating_add(other.count);
        self.sum += other.sum;

        if let Some(value) = other.min {
            self.min = Some(self.min.map_or(value, |current| current.min(value)));
        }
        if let Some(value) = other.max {
            self.max = Some(self.max.map_or(value, |current| current.max(value)));
        }
        if other.count > 0 {
            self.digest = TDigest::merge_digests(vec![self.digest.clone(), other.digest.clone()]);
        }
    }
}
