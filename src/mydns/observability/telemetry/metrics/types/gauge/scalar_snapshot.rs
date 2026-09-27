//! Detached snapshot of a scalar gauge.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GaugeSnapshot {
    pub value: f64,
}

impl GaugeSnapshot {
    /// A gauge represents current state, so merging replaces the value with the
    /// snapshot supplied by the caller.
    pub fn merge(&mut self, other: &Self) {
        self.value = other.value;
    }
}
