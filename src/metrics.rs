use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 20_000;
const HISTORY: Duration = Duration::from_secs(24 * 60 * 60);

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
        let mut samples = self
            .upstream_samples
            .lock()
            .expect("metrics upstream sample lock poisoned");
        samples.push_back((Instant::now(), latency_ms));
        trim(&mut samples);
    }

    pub fn record_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, response_ms: f64) {
        let mut samples = self
            .response_samples
            .lock()
            .expect("metrics response sample lock poisoned");
        samples.push_back((Instant::now(), response_ms));
        trim(&mut samples);
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let now = Instant::now();
        let queries = self.queries.load(Ordering::Relaxed);
        let upstream_requests = self.upstream_requests.load(Ordering::Relaxed);
        let upstream_successes = self.upstream_successes.load(Ordering::Relaxed);
        let availability = if upstream_requests == 0 {
            100.0
        } else {
            upstream_successes as f64 / upstream_requests as f64 * 100.0
        };

        let mut response_samples = self
            .response_samples
            .lock()
            .expect("metrics response sample lock poisoned");
        trim(&mut response_samples);
        let response_values: Vec<f64> = response_samples.iter().map(|(_, value)| *value).collect();
        let requests_per_minute = response_samples
            .iter()
            .filter(|(at, _)| now.duration_since(*at) <= Duration::from_secs(60))
            .count() as u64;
        drop(response_samples);

        let mut upstream_samples = self
            .upstream_samples
            .lock()
            .expect("metrics upstream sample lock poisoned");
        trim(&mut upstream_samples);
        let upstream_values: Vec<f64> = upstream_samples.iter().map(|(_, value)| *value).collect();
        drop(upstream_samples);

        serde_json::json!({
            "uptime_secs": self.started.elapsed().as_secs(),
            "requests_per_minute": requests_per_minute,
            "queries_total": queries,
            "upstream": { "requests": upstream_requests, "successes": upstream_successes, "failures": self.upstream_failures.load(Ordering::Relaxed), "availability_pct": round(availability), "latency_ms": percentile_stats(&upstream_values) },
            "response_time_ms": percentile_stats(&response_values),
            "cache_evictions": self.cache_evictions.load(Ordering::Relaxed),
            "dns_errors": self.errors.load(Ordering::Relaxed),
            "query_types": self.query_types.lock().expect("metrics query type lock poisoned").clone(),
            "resolution_outcomes": self.outcomes.lock().expect("metrics outcome lock poisoned").clone()
        })
    }
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

fn percentile_stats(values: &[f64]) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::json!({"avg_ms": 0.0, "p50_ms": 0.0, "p95_ms": 0.0, "p99_ms": 0.0});
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    serde_json::json!({"avg_ms": round(avg), "p50_ms": round(percentile(&sorted, 0.50)), "p95_ms": round(percentile(&sorted, 0.95)), "p99_ms": round(percentile(&sorted, 0.99))})
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
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
