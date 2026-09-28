//! Detached snapshot of a bounded categorical counter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::observability::telemetry::metrics::types::SnapshotTypeTrait;

const MAX_CATEGORIES: usize = 32;
const OTHER_CATEGORY: &str = "other";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BoundedCounterSnapshot {
    pub values: BTreeMap<String, u64>,
}

impl SnapshotTypeTrait for BoundedCounterSnapshot {
    fn merge(&mut self, other: &Self) {
        for (value, count) in &other.values {
            if let Some(existing) = self.values.get_mut(value) {
                *existing = existing.saturating_add(*count);
                continue;
            }

            if self.values.len() < MAX_CATEGORIES.saturating_sub(1) {
                self.values.insert(value.clone(), *count);
            } else {
                let other_count = self.values.entry(OTHER_CATEGORY.to_owned()).or_insert(0);
                *other_count = other_count.saturating_add(*count);
            }
        }
    }
}
