//! Detached snapshot of a scalar gauge.

use serde::{Deserialize, Serialize};

use crate::observability::telemetry::metrics::types::SnapshotTypeTrait;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GaugeSnapshot {
    pub value: f64,
}

impl SnapshotTypeTrait for GaugeSnapshot {
    /// A gauge represents current state, so merging replaces the value with the
    /// snapshot supplied by the caller.
    fn merge(&mut self, other: &Self) {
        self.value = other.value;
    }
}
