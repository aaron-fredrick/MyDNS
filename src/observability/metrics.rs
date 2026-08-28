use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::types::{LatencyStats, MetricsSnapshot, UpstreamStats};

const MAX_SAMPLES: usize = 20_000;
const HISTORY: Duration = Duration::from_secs(24 * 60 * 60);

/// Backend-owned operational metrics. Storage is bounded by count and age.
pub struct Metrics {
    started: Instant,
    queries: AtomicU64,
    upstream_requests: AtomicU64,
    upstream_successes: AtomicU64,
    upstream_failures: AtomicU64,
    cache_evictions: AtomicU64,
    errors: AtomicU64,
    query_types: Mutex<HashMap<String, u64>>,
    outcomes: Mutex<HashMap<String, u64>>,
    response_samples: Mutex<VecDeque<(Instant, f64)>>,
    upstream_samples: Mutex<VecDeque<(Instant, f64)>>,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_query(&self, query_type: &str) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        increment(&self.query_types, query_type);
    }

    pub fn record_outcome(&self, outcome: &str) {
        increment(&self.outcomes, outcome);
        if matches!(outcome, "SERVFAIL" | "REFUSED" | "OTHER") {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_upstream_start(&self) {
        self.upstream_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_upstream_result(&self, success: bool) {
        if success {
            self.upstream_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.upstream_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_upstream_latency(&self, latency_ms: f64) {
        record_sample(&self.upstream_samples, latency_ms);
    }

    pub fn record_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, response_ms: f64) {
        record_sample(&self.response_samples, response_ms);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let now = Instant::now();
        let upstream_requests = self.upstream_requests.load(Ordering::Relaxed);
        let upstream_successes = self.upstream_successes.load(Ordering::Relaxed);
        let availability = if upstream_requests == 0 {
            100.0
        } else {
            upstream_successes as f64 / upstream_requests as f64 * 100.0
        };
        let response_values = current_values(&self.response_samples);
        let requests_per_minute = current_count(
            &self.response_samples,
            now,
            Duration::from_secs(60),
        );
        let upstream_values = current_values(&self.upstream_samples);

        MetricsSnapshot {
            uptime_secs: self.started.elapsed().as_secs(),
            requests_per_minute,
            queries_total: self.queries.load(Ordering::Relaxed),
            upstream: UpstreamStats {
                requests: upstream_requests,
                successes: upstream_successes,
                failures: self.upstream_failures.load(Ordering::Relaxed),
                availability_pct: round(availability),
                latency: percentile_stats(&upstream_values),
            },
            response_time: percentile_stats(&response_values),
            cache_evictions: self.cache_evictions.load(Ordering::Relaxed),
            dns_errors: self.errors.load(Ordering::Relaxed),
            query_types: self
                .query_types
                .lock()
                .expect("metrics query type lock poisoned")
                .clone(),
            resolution_outcomes: self
                .outcomes
                .lock()
                .expect("metrics outcome lock poisoned")
                .clone(),
        }
    }
}

fn record_sample(samples: &Mutex<VecDeque<(Instant, f64)>>, value: f64) {
    if !value.is_finite() || value < 0.0 {
        return;
    }
    let mut samples = samples.lock().expect("metrics sample lock poisoned");
    samples.push_back((Instant::now(), value));
    trim(&mut samples);
}

fn current_values(samples: &Mutex<VecDeque<(Instant, f64)>>) -> Vec<f64> {
    let mut samples = samples.lock().expect("metrics sample lock poisoned");
    trim(&mut samples);
    samples.iter().map(|(_, value)| *value).collect()
}

fn current_count(
    samples: &Mutex<VecDeque<(Instant, f64)>>,
    now: Instant,
    window: Duration,
) -> u64 {
    let samples = samples.lock().expect("metrics sample lock poisoned");
    samples
        .iter()
        .filter(|(at, _)| now.duration_since(*at) <= window)
        .count() as u64
}

fn trim(samples: &mut VecDeque<(Instant, f64)>) {
    while samples.len() > MAX_SAMPLES
        || samples
            .front()
            .is_some_and(|(at, _)| at.elapsed() > HISTORY)
    {
        samples.pop_front();
    }
}

fn increment(map: &Mutex<HashMap<String, u64>>, key: &str) {
    let mut map = map.lock().expect("metrics counter lock poisoned");
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn percentile_stats(values: &[f64]) -> LatencyStats {
    if values.is_empty() {
        return LatencyStats::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    LatencyStats {
        avg_ms: round(sorted.iter().sum::<f64>() / sorted.len() as f64),
        p50_ms: round(percentile(&sorted, 0.50)),
        p95_ms: round(percentile(&sorted, 0.95)),
        p99_ms: round(percentile(&sorted, 0.99)),
    }
}

/// Uses linear interpolation between adjacent ordered observations.
/// This is deterministic, monotonic, and avoids the off-by-one behaviour of
/// rounding an index for small samples.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let position = (sorted.len() - 1) as f64 * p;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let weight = position - lower as f64;
    sorted[lower] + (sorted[upper] - sorted[lower]) * weight
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            queries: AtomicU64::new(0),
            upstream_requests: AtomicU64::new(0),
            upstream_successes: AtomicU64::new(0),
            upstream_failures: AtomicU64::new(0),
            cache_evictions: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            query_types: Mutex::new(HashMap::new()),
            outcomes: Mutex::new(HashMap::new()),
            response_samples: Mutex::new(VecDeque::with_capacity(MAX_SAMPLES)),
            upstream_samples: Mutex::new(VecDeque::with_capacity(MAX_SAMPLES)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_starts_empty_and_healthy() {
        let snapshot = Metrics::default().snapshot();
        assert_eq!(snapshot.queries_total, 0);
        assert_eq!(snapshot.requests_per_minute, 0);
        assert_eq!(snapshot.upstream.availability_pct, 100.0);
        assert_eq!(snapshot.response_time.avg_ms, 0.0);
    }

    #[test]
    fn records_queries_and_outcomes() {
        let metrics = Metrics::default();
        metrics.record_query("A");
        metrics.record_query("A");
        metrics.record_query("AAAA");
        metrics.record_outcome("NOERROR");
        metrics.record_outcome("SERVFAIL");
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.queries_total, 3);
        assert_eq!(snapshot.query_types["A"], 2);
        assert_eq!(snapshot.query_types["AAAA"], 1);
        assert_eq!(snapshot.resolution_outcomes["NOERROR"], 1);
        assert_eq!(snapshot.dns_errors, 1);
    }

    #[test]
    fn calculates_latency_percentiles() {
        let metrics = Metrics::default();
        for value in 1..=100 {
            metrics.record_latency(value as f64);
        }
        let stats = &metrics.snapshot().response_time;
        assert_eq!(stats.avg_ms, 50.5);
        assert_eq!(stats.p50_ms, 50.5);
        assert_eq!(stats.p95_ms, 95.05);
        assert_eq!(stats.p99_ms, 99.01);
    }

    #[test]
    fn percentile_handles_single_and_small_samples() {
        assert_eq!(percentile(&[42.0], 0.99), 42.0);
        assert_eq!(percentile(&[10.0, 20.0], 0.50), 15.0);
        assert_eq!(percentile(&[10.0, 20.0], 0.95), 19.5);
    }

    #[test]
    fn calculates_upstream_availability() {
        let metrics = Metrics::default();
        for _ in 0..8 {
            metrics.record_upstream_start();
            metrics.record_upstream_result(true);
        }
        for _ in 0..2 {
            metrics.record_upstream_start();
            metrics.record_upstream_result(false);
        }
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.upstream.requests, 10);
        assert_eq!(snapshot.upstream.successes, 8);
        assert_eq!(snapshot.upstream.failures, 2);
        assert_eq!(snapshot.upstream.availability_pct, 80.0);
    }

    #[test]
    fn ignores_invalid_latency_samples() {
        let metrics = Metrics::default();
        metrics.record_latency(-1.0);
        metrics.record_latency(f64::NAN);
        metrics.record_latency(12.0);
        assert_eq!(metrics.snapshot().response_time.avg_ms, 12.0);
    }
}
