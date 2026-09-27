//! Detached snapshot of a fixed-bound histogram.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistogramSnapshot {
    pub bounds: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum: f64,
}

impl HistogramSnapshot {
    pub fn merge(&mut self, other: &Self) {
        assert_eq!(self.bounds, other.bounds, "histogram bounds must match");
        assert_eq!(self.counts.len(), other.counts.len(), "histogram bucket counts must match");

        self.count = self.count.saturating_add(other.count);
        self.sum += other.sum;

        for (left, right) in self.counts.iter_mut().zip(&other.counts) {
            *left = left.saturating_add(*right);
        }
    }
}
