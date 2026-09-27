//! Detached snapshot of a scalar counter.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScalarCounterSnapshot {
    pub value: u64,
}

impl ScalarCounterSnapshot {
    pub fn merge(&mut self, other: &Self) {
        self.value = self.value.saturating_add(other.value);
    }
}
