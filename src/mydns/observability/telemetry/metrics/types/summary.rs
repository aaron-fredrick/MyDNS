//! Summary metric type.
//!
//! This type keeps mergeable descriptive statistics. Quantile estimation is
//! intentionally not part of this initial implementation; histograms provide
//! the current distribution-oriented metric type for MyDNS.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    count: u64,
    sum: f64,
    min: Option<f64>,
    max: Option<f64>,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: None,
            max: None,
        }
    }
}

impl Summary {
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

    pub fn merge(&mut self, other: &Self) {
        self.count = self.count.saturating_add(other.count);
        self.sum += other.sum;

        if let Some(value) = other.min {
            self.min = Some(self.min.map_or(value, |current| current.min(value)));
        }

        if let Some(value) = other.max {
            self.max = Some(self.max.map_or(value, |current| current.max(value)));
        }
    }
}
