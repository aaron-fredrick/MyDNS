//! Mergeable statistical summary with t-digest quantile estimation.

use serde::{Deserialize, Serialize};
use tdigest::TDigest;

const DEFAULT_TDIGEST_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TDigestSummary {
    count: u64,
    sum: f64,
    min: Option<f64>,
    max: Option<f64>,
    digest: TDigest,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: None,
            max: None,
            digest: TDigest::new_with_size(DEFAULT_TDIGEST_SIZE),
        }
    }
}

impl TDigestSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }

        self.count = self.count.saturating_add(1);
        self.sum += value;
        self.min = Some(self.min.map_or(value, |current| current.min(value)));
        self.max = Some(self.max.map_or(value, |current| current.max(value)));
        self.digest = self.digest.merge_unsorted(vec![value]);
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
    }

    pub fn min(&self) -> Option<f64> {
        self.min
    }

    pub fn max(&self) -> Option<f64> {
        self.max
    }

    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.count == 0 || !q.is_finite() || !(0.0..=1.0).contains(&q) {
            return None;
        }

        Some(self.digest.estimate_quantile(q))
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
