//! Detached snapshot of a scalar counter.

use serde::{Deserialize, Serialize};
use crate::observability::telemetry::metrics::types::SnapshotTypeTrait;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScalarCounterSnapshot {
    pub value: u64,
}

impl SnapshotTypeTrait for ScalarCounterSnapshot {
    type Snapshot = ScalarCounterSnapshot;

    fn merge(&mut self, other: &Self) -> Self::Snapshot {
        Self::Snapshot {
            value: self.value.saturating_add(other.value),
        }
    }
}
