//! Basic mergeable summary.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Summary { count: u64, sum: f64, min: Option<f64>, max: Option<f64> }

impl Default for Summary {
    fn default() -> Self { Self { count: 0, sum: 0.0, min: None, max: None } }
}
impl Summary {
    pub fn new() -> Self { Self::default() }
    pub fn record(&mut self, value: f64) {
        if !value.is_finite() { return; }
        self.count = self.count.saturating_add(1);
        self.sum += value;
        self.min = Some(self.min.map_or(value, |v| v.min(value)));
        self.max = Some(self.max.map_or(value, |v| v.max(value)));
    }
    pub fn count(&self) -> u64 { self.count }
    pub fn sum(&self) -> f64 { self.sum }
    pub fn mean(&self) -> Option<f64> { (self.count > 0).then_some(self.sum / self.count as f64) }
    pub fn min(&self) -> Option<f64> { self.min }
    pub fn max(&self) -> Option<f64> { self.max }
    pub fn merge(&mut self, other: &Self) {
        self.count = self.count.saturating_add(other.count);
        self.sum += other.sum;
        if let Some(v) = other.min { self.min = Some(self.min.map_or(v, |x| x.min(v))); }
        if let Some(v) = other.max { self.max = Some(self.max.map_or(v, |x| x.max(v))); }
    }
}
