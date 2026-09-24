use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::observability::telemetry::metrics::core::{BoundedCounts, MergeableHistogram};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalPeriodSnapshot {
    pub timezone: String,
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
    pub queries: u64,
    pub responses: u64,
    pub blocked: u64,
    pub blocked_reason_counts: BoundedCounts,
    pub record_type_counts: BoundedCounts,
    pub transport_counts: BoundedCounts,
    pub response_code_counts: BoundedCounts,
    pub resolution_outcome_counts: BoundedCounts,
    pub resolution_path_counts: BoundedCounts,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub upstream_requests: u64,
    pub upstream_successes: u64,
    pub upstream_failures: u64,
    pub upstream_timeouts: u64,
    pub upstream_retries: u64,
    pub response_latency: MergeableHistogram,
    pub upstream_latency: MergeableHistogram,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryBucket {
    pub timestamp: DateTime<Utc>,
    pub resolution_seconds: u32,
    pub request_count: u64,
    pub response_count: u64,
    pub blocked_count: u64,
    pub blocked_reason_counts: BoundedCounts,
    pub record_type_counts: BoundedCounts,
    pub transport_counts: BoundedCounts,
    pub response_code_counts: BoundedCounts,
    pub resolution_outcome_counts: BoundedCounts,
    pub resolution_path_counts: BoundedCounts,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub upstream_requests: u64,
    pub upstream_successes: u64,
    pub upstream_failures: u64,
    pub upstream_timeouts: u64,
    pub upstream_retries: u64,
    pub response_latency: MergeableHistogram,
    pub upstream_latency: MergeableHistogram,
}

impl HistoryBucket {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self::with_resolution(timestamp, 60)
    }

    pub fn with_resolution(timestamp: DateTime<Utc>, resolution_seconds: u32) -> Self {
        Self {
            timestamp,
            resolution_seconds,
            request_count: 0,
            response_count: 0,
            blocked_count: 0,
            blocked_reason_counts: BoundedCounts::default(),
            record_type_counts: BoundedCounts::default(),
            transport_counts: BoundedCounts::default(),
            response_code_counts: BoundedCounts::default(),
            resolution_outcome_counts: BoundedCounts::default(),
            resolution_path_counts: BoundedCounts::default(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            upstream_requests: 0,
            upstream_successes: 0,
            upstream_failures: 0,
            upstream_timeouts: 0,
            upstream_retries: 0,
            response_latency: MergeableHistogram::response(),
            upstream_latency: MergeableHistogram::upstream(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.request_count == 0
            && self.response_count == 0
            && self.blocked_count == 0
            && self.cache_hits == 0
            && self.cache_misses == 0
            && self.cache_evictions == 0
            && self.upstream_requests == 0
            && self.upstream_successes == 0
            && self.upstream_failures == 0
            && self.upstream_timeouts == 0
            && self.upstream_retries == 0
            && self.blocked_reason_counts.is_empty()
            && self.record_type_counts.is_empty()
            && self.transport_counts.is_empty()
            && self.response_code_counts.is_empty()
            && self.resolution_outcome_counts.is_empty()
            && self.resolution_path_counts.is_empty()
            && self.response_latency.count == 0
            && self.upstream_latency.count == 0
    }

    pub fn merge(&mut self, other: &Self) {
        if self.resolution_seconds != other.resolution_seconds {
            return;
        }

        self.request_count += other.request_count;
        self.response_count += other.response_count;
        self.blocked_count += other.blocked_count;
        self.blocked_reason_counts
            .merge(&other.blocked_reason_counts);
        self.record_type_counts.merge(&other.record_type_counts);
        self.transport_counts.merge(&other.transport_counts);
        self.response_code_counts.merge(&other.response_code_counts);
        self.resolution_outcome_counts
            .merge(&other.resolution_outcome_counts);
        self.resolution_path_counts
            .merge(&other.resolution_path_counts);
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;
        self.cache_evictions += other.cache_evictions;
        self.upstream_requests += other.upstream_requests;
        self.upstream_successes += other.upstream_successes;
        self.upstream_failures += other.upstream_failures;
        self.upstream_timeouts += other.upstream_timeouts;
        self.upstream_retries += other.upstream_retries;
        self.response_latency.merge(&other.response_latency);
        self.upstream_latency.merge(&other.upstream_latency);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_bucket_merge_combines_all_primitives() {
        let timestamp = DateTime::parse_from_rfc3339("2026-09-24T00:00:00Z").unwrap().with_timezone(&Utc);
        let mut left = HistoryBucket::new(timestamp);
        left.request_count = 2;
        left.record_type_counts.record("A");
        let mut right = HistoryBucket::new(timestamp);
        right.request_count = 3;
        right.record_type_counts.record("A");
        right.record_type_counts.record("AAAA");
        left.merge(&right);
        assert_eq!(left.request_count, 5);
        assert_eq!(left.record_type_counts.get("A"), 2);
        assert_eq!(left.record_type_counts.get("AAAA"), 1);
    }
}
