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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeableHistogram {
    pub bounds_ms: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum_ms: f64,
}

impl MergeableHistogram {
    pub fn new(bounds_ms: &[f64]) -> Self {
        assert!(
            bounds_ms.windows(2).all(|window| window[0] < window[1]),
            "histogram bounds must be strictly increasing"
        );

        Self {
            bounds_ms: bounds_ms.to_vec(),
            counts: vec![0; bounds_ms.len() + 1],
            count: 0,
            sum_ms: 0.0,
        }
    }

    pub fn response() -> Self {
        // 500ms+ is a candidate warning threshold and 3000ms+ a candidate critical threshold
        // for DNS response latency. 5000ms is intentionally the final finite bound: above ~5s,
        // finer tail resolution is not useful for MyDNS operational decisions.
        // * IMPORTANT: these thresholds are candidates for alert rules, not alerting itself.
        // TODO: Wire suitable warning/critical latency thresholds into the alerting capability
        // after instrumentation and real workload data establish appropriate policy.
        Self::new(&[
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
        ])
    }

    pub fn upstream() -> Self {
        // Upstream resolution can legitimately have a longer tail than local response work,
        // but 5000ms remains the practical upper bound for the same operational reason.
        Self::new(&[
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 3000.0,
            5000.0,
        ])
    }

    pub fn record(&mut self, value_ms: f64) {
        if !value_ms.is_finite() || value_ms < 0.0 {
            return;
        }

        self.count += 1;
        self.sum_ms += value_ms;

        let index = self
            .bounds_ms
            .iter()
            .position(|bound| value_ms <= *bound)
            .unwrap_or(self.bounds_ms.len());
        self.counts[index] += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        if self.bounds_ms != other.bounds_ms {
            return;
        }

        self.count += other.count;
        self.sum_ms += other.sum_ms;

        for (left, right) in self.counts.iter_mut().zip(&other.counts) {
            *left += right;
        }
    }

    pub fn mean_ms(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum_ms / self.count as f64)
    }

    /// Returns an approximate quantile using the histogram buckets.
    // TODO: Investigate a mergeable percentile/distribution algorithm or pipeline that can
    // retain enough information for more accurate p50/p95/p99 calculations. Determine whether
    // MyDNS actually needs exact percentile accuracy or whether bounded histograms are sufficient
    // for diagnostics, alerting, and long-term rollups before changing this representation.
    ///
    /// The estimate is intentionally bounded by the bucket containing the
    /// quantile. The overflow bucket has no finite upper bound, so quantiles
    /// landing there return the final configured bound.
    pub fn quantile(&self, quantile: f64) -> Option<f64> {
        if self.count == 0 || !quantile.is_finite() || !(0.0..=1.0).contains(&quantile) {
            return None;
        }

        let rank = quantile * self.count as f64;
        let mut cumulative = 0_u64;

        for (index, bucket_count) in self.counts.iter().copied().enumerate() {
            if bucket_count == 0 {
                continue;
            }

            let next = cumulative + bucket_count;
            if rank <= next as f64 {
                let lower = if index == 0 {
                    0.0
                } else {
                    self.bounds_ms[index - 1]
                };
                let upper = self
                    .bounds_ms
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| *self.bounds_ms.last().unwrap());

                let position = if bucket_count == 1 {
                    0.5
                } else {
                    ((rank - cumulative as f64) / bucket_count as f64).clamp(0.0, 1.0)
                };

                return Some(lower + (upper - lower) * position);
            }

            cumulative = next;
        }

        self.bounds_ms.last().copied()
    }
}

impl Default for MergeableHistogram {
    fn default() -> Self {
        Self::response()
    }
}

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
    fn bounded_counts_cap_categories_and_preserve_overflow() {
        let mut counts = BoundedCounts::default();

        for index in 0..40 {
            counts.record(&format!("category-{index}"));
        }

        assert_eq!(counts.as_map().len(), MAX_CATEGORIES);
        assert!(counts.get(OTHER_CATEGORY) > 0);
        assert_eq!(counts.get("category-0"), 1);
    }

    #[test]
    fn histograms_merge_and_expose_mean_and_quantiles() {
        let mut left = MergeableHistogram::response();
        left.record(10.0);
        left.record(20.0);

        let mut right = MergeableHistogram::response();
        right.record(100.0);

        left.merge(&right);

        assert_eq!(left.count, 3);
        assert_eq!(left.mean_ms(), Some(130.0 / 3.0));
        assert!(left.quantile(0.5).unwrap() >= 10.0);
        assert!(left.quantile(0.95).unwrap() <= 100.0);
        assert!(left.quantile(0.5).unwrap() <= left.quantile(0.95).unwrap());
    }

    #[test]
    fn histogram_rejects_invalid_samples() {
        let mut histogram = MergeableHistogram::response();
        histogram.record(f64::NAN);
        histogram.record(f64::INFINITY);
        histogram.record(-1.0);
        assert_eq!(histogram.count, 0);
        assert_eq!(histogram.sum_ms, 0.0);
    }

    #[test]
    fn history_bucket_merge_combines_all_primitives() {
        let timestamp = DateTime::parse_from_rfc3339("2026-09-24T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
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
