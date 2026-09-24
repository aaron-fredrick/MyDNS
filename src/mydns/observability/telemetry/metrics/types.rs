use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_CATEGORIES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundedCounts { values: BTreeMap<String, u64> }

impl BoundedCounts {
    pub fn record(&mut self, value: &str) {
        let value = value.trim();
        if value.is_empty() { return; }
        if let Some(count) = self.values.get_mut(value) { *count += 1; return; }
        if self.values.len() < MAX_CATEGORIES.saturating_sub(1) {
            self.values.insert(value.to_owned(), 1);
        } else {
            *self.values.entry("other".to_owned()).or_insert(0) += 1;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeableHistogram {
    pub bounds_ms: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum_ms: f64,
}

impl MergeableHistogram {
    pub fn new(bounds_ms: &[f64]) -> Self {
        Self { bounds_ms: bounds_ms.to_vec(), counts: vec![0; bounds_ms.len() + 1], count: 0, sum_ms: 0.0 }
    }
    pub fn response() -> Self {
        Self::new(&[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
    }
    pub fn upstream() -> Self {
        Self::new(&[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0])
    }
    pub fn record(&mut self, value_ms: f64) {
        if !value_ms.is_finite() || value_ms < 0.0 { return; }
        self.count += 1;
        self.sum_ms += value_ms;
        let index = self.bounds_ms.iter().position(|bound| value_ms <= *bound).unwrap_or(self.bounds_ms.len());
        self.counts[index] += 1;
    }
    pub fn merge(&mut self, other: &Self) {
        if self.bounds_ms != other.bounds_ms { return; }
        self.count += other.count;
        self.sum_ms += other.sum_ms;
        for (left, right) in self.counts.iter_mut().zip(&other.counts) { *left += right; }
    }
}
impl Default for MergeableHistogram { fn default() -> Self { Self::response() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalPeriodSnapshot {
    pub timezone: String,
    pub start_utc: DateTime<Utc>, pub end_utc: DateTime<Utc>,
    pub queries: u64, pub responses: u64, pub blocked: u64,
    pub blocked_reason_counts: BoundedCounts,
    pub record_type_counts: BoundedCounts, pub transport_counts: BoundedCounts,
    pub response_code_counts: BoundedCounts, pub resolution_outcome_counts: BoundedCounts,
    pub resolution_path_counts: BoundedCounts,
    pub cache_hits: u64, pub cache_misses: u64, pub cache_evictions: u64,
    pub upstream_requests: u64, pub upstream_successes: u64, pub upstream_failures: u64,
    pub upstream_timeouts: u64, pub upstream_retries: u64,
    pub response_latency: MergeableHistogram, pub upstream_latency: MergeableHistogram,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryBucket {
    pub timestamp: DateTime<Utc>, pub resolution_seconds: u32,
    pub request_count: u64, pub response_count: u64, pub blocked_count: u64,
    pub blocked_reason_counts: BoundedCounts,
    pub record_type_counts: BoundedCounts, pub transport_counts: BoundedCounts,
    pub response_code_counts: BoundedCounts, pub resolution_outcome_counts: BoundedCounts,
    pub resolution_path_counts: BoundedCounts,
    pub cache_hits: u64, pub cache_misses: u64, pub cache_evictions: u64,
    pub upstream_requests: u64, pub upstream_successes: u64, pub upstream_failures: u64,
    pub upstream_timeouts: u64, pub upstream_retries: u64,
    pub response_latency: MergeableHistogram, pub upstream_latency: MergeableHistogram,
}

impl HistoryBucket {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp, resolution_seconds: 60, request_count: 0, response_count: 0, blocked_count: 0,
            blocked_reason_counts: BoundedCounts::default(),
            record_type_counts: BoundedCounts::default(), transport_counts: BoundedCounts::default(),
            response_code_counts: BoundedCounts::default(), resolution_outcome_counts: BoundedCounts::default(),
            resolution_path_counts: BoundedCounts::default(), cache_hits: 0, cache_misses: 0,
            cache_evictions: 0, upstream_requests: 0, upstream_successes: 0, upstream_failures: 0,
            upstream_timeouts: 0, upstream_retries: 0,
            response_latency: MergeableHistogram::response(), upstream_latency: MergeableHistogram::upstream(),
        }
    }
}
