//! Detached snapshot of a scalar counter.

use serde::{Deserialize, Serialize};

use crate::observability::telemetry::metrics::types::SnapshotTypeTrait;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScalarCounterSnapshot {
    pub value: u64,
}

impl SnapshotTypeTrait for ScalarCounterSnapshot {
    fn merge(&mut self, other: &Self) {
        self.value = self.value.saturating_add(other.value);
    }
}
