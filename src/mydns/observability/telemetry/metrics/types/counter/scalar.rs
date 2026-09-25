//! Scalar monotonic counter.
use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

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
    pub fn merge(&mut self, other: &Self) {
        self.value = self.value.saturating_add(other.value);
    }
}

impl AddAssign<u64> for ScalarCounter {
    fn add_assign(&mut self, amount: u64) {
        self.increment_by(amount);
    }
}

impl AddAssign<ScalarCounter> for ScalarCounter {
    fn add_assign(&mut self, other: ScalarCounter) {
        self.merge(&other);
    }
}
