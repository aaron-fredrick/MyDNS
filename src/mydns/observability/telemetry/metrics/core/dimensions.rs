//! Generic bounded categorical metric dimensions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_CATEGORIES: usize = 32;
const OTHER_CATEGORY: &str = "other";

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
            *existing += count;
            return;
        }

        if self.values.len() < MAX_CATEGORIES.saturating_sub(1) {
            self.values.insert(value.to_owned(), count);
        } else {
            *self.values.entry(OTHER_CATEGORY.to_owned()).or_insert(0) += count;
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
