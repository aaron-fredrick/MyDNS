//! Scalar monotonic counter.
use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

use super::ScalarCounterSnapshot;
use crate::observability::telemetry::metrics::types::Reset;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScalarCounter {
    value: u64,
}

impl ScalarCounter {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn increment(&mut self) {
        self.increment_by(1);
    }
    pub fn increment_by(&mut self, amount: u64) {
        self.value = self.value.saturating_add(amount);
    }
    pub fn value(&self) -> u64 {
        self.value
    }
}

impl Reset for ScalarCounter {
    fn reset(&mut self) {
        self.value = 0;
    }
}

impl From<&ScalarCounter> for ScalarCounterSnapshot {
    fn from(metric: &ScalarCounter) -> Self {
        Self {
            value: metric.value,
        }
    }
}

impl AddAssign<u64> for ScalarCounter {
    fn add_assign(&mut self, amount: u64) {
        self.increment_by(amount);
    }
}
