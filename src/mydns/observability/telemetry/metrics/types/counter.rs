//! Counter metric types.
//!
//! Counters represent monotonically increasing event totals. BoundedCounts
//! provides a categorical counter when a bounded set of values is required.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_CATEGORIES: usize = 32;
const OTHER_CATEGORY: &str = "other";

/// A scalar monotonically increasing counter.
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

/// A bounded categorical counter.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BoundedCounts {
    values: BTreeMap<String, u64>,
}

impl BoundedCounts {
    pub fn record(&mut self, value: &str) {
        self.record_n(value, 1);
    }

    pub fn record_n(&mut self, value: &str, count: u64) {
        let value = value.trim();
        if value.is_empty() || count == 0 {
            return;
        }

        if let Some(existing) = self.values.get_mut(value) {
            *existing = existing.saturating_add(count);
            return;
        }

        if self.values.len() < MAX_CATEGORIES.saturating_sub(1) {
            self.values.insert(value.to_owned(), count);
        } else {
            let other = self.values.entry(OTHER_CATEGORY.to_owned()).or_insert(0);
            *other = other.saturating_add(count);
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for (value, count) in &other.values {
            self.record_n(value, *count);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_counter_accumulates() {
        let mut counter = ScalarCounter::new();
        counter.increment();
        counter.increment_by(4);
        assert_eq!(counter.value(), 5);
    }

    #[test]
    fn bounded_counts_remains_ordered_and_bounded() {
        let mut counts = BoundedCounts::default();
        counts.record("POST");
        counts.record("GET");

        assert_eq!(counts.get("POST"), 1);
        assert_eq!(counts.as_map().keys().next().unwrap(), "GET");
    }
}
