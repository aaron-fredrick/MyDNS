//! Bounded categorical counter.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::bounded_snapshot::BoundedCounterSnapshot;

const MAX_CATEGORIES: usize = 32;
const OTHER_CATEGORY: &str = "other";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BoundedCounter {
    values: BTreeMap<String, u64>,
}
impl BoundedCounter {
    pub fn increment(&mut self, value: &str) {
        self.increment_by(value, 1)
    }
    pub fn increment_by(&mut self, value: &str, count: u64) {
        let value = value.trim();
        if value.is_empty() || count == 0 {
            return;
        }
        if let Some(existing) = self.values.get_mut(value) {
            *existing = existing.saturating_add(count);
            return;
        }
        if self.values.len() < MAX_CATEGORIES.saturating_sub(1) {
            self.values.insert(value.to_owned(), count)
        } else {
            let other = self.values.entry(OTHER_CATEGORY.to_owned()).or_insert(0);
            *other = other.saturating_add(count)
        }
    }
    pub fn get(&self, value: &str) -> u64 {
        self.values.get(value).copied().unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn as_map(&self) -> &BTreeMap<String, u64> {
        &self.values
    }
}
impl From<&BoundedCounter> for BoundedCounterSnapshot {
    fn from(metric: &BoundedCounter) -> Self {
        Self {
            values: metric.values.clone(),
        }
    }
}
