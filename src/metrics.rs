use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 20_000;
const HISTORY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy)]
pub struct LatencySample {
    pub at: Instant,
    pub response_ms: f64,
    pub upstream_ms: Option<f64>,
}

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
    samples: Mutex<VecDeque<LatencySample>>,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started: Instant::now(),
            queries: AtomicU64::new(0),
            upstream_requests: AtomicU64::new(0),
            upstream_successes: AtomicU64::new(0),
            upstream_failures: AtomicU64::new(0),
            cache_evictions: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            query_types: Mutex::new(HashMap::new()),
            outcomes: Mutex::new(HashMap::new()),
            samples: Mutex::new(VecDeque::with_capacity(MAX_SAMPLES)),
        })
    }

    pub fn record_query(&self, query_type: &str) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        increment(&self.query_types, query_type);
    }

    pub fn record_outcome(&self, outcome: &str) {
        increment(&self.outcomes, outcome);
        if outcome != "NOERROR" {
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

    pub fn record_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, response_ms: f64, upstream_ms: Option<f64>) {
        let mut samples = self.samples.lock().expect("metrics sample lock poisoned");
        samples.push_back(LatencySample { at: Instant::now(), response_ms, upstream_ms });
        while samples.len() > MAX_SAMPLES || samples.front().is_some_and(|s| s.at.elapsed() > HISTORY) {
            samples.pop_front();
        }
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let now = Instant::now();
        let queries = self.queries.load(Ordering::Relaxed);
        let hits = 0u64; // Filled by the existing cache counters at the API boundary.
        let misses = 0u64;
        let upstream_requests = self.upstream_requests.load(Ordering::Relaxed);
        let upstream_successes = self.upstream_successes.load(Ordering::Relaxed);
        let upstream_failures = self.upstream_failures.load(Ordering::Relaxed);
        let availability = if upstream_requests == 0 { 100.0 } else { upstream_successes as f64 / upstream_requests as f64 * 100.0 };

        let mut samples = self.samples.lock().expect("metrics sample lock poisoned");
        while samples.front().is_some_and(|s| s.at.elapsed() > HISTORY) { samples.pop_front(); }
        let response: Vec<f64> = samples.iter().map(|s| s.response_ms).collect();
        let upstream: Vec<f64> = samples.iter().filter_map(|s| s.upstream_ms).collect();
        let response_stats = percentile_stats(&response);
        let upstream_stats = percentile_stats(&upstream);
        let requests_per_minute = samples.iter().filter(|s| now.duration_since(s.at) <= Duration::from_secs(60)).count() as u64;

        serde_json::json!({
            "uptime_secs": self.started.elapsed().as_secs(),
            "requests_per_minute": requests_per_minute,
            "queries_total": queries,
            "cache_hits": hits,
            "cache_misses": misses,
            "cache_hit_rate": 0.0,
            "upstream": {
                "requests": upstream_requests,
                "successes": upstream_successes,
                "failures": upstream_failures,
                "availability_pct": availability,
                "latency_ms": upstream_stats
            },
            "response_time_ms": response_stats,
            "cache_evictions": self.cache_evictions.load(Ordering::Relaxed),
            "dns_errors": self.errors.load(Ordering::Relaxed),
            "query_types": self.query_types.lock().expect("metrics query type lock poisoned").clone(),
            "resolution_outcomes": self.outcomes.lock().expect("metrics outcome lock poisoned").clone()
        })
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
    serde_json::json!({
        "avg_ms": round(avg),
        "p50_ms": round(percentile(&sorted, 0.50)),
        "p95_ms": round(percentile(&sorted, 0.95)),
        "p99_ms": round(percentile(&sorted, 0.99))
    })
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[index]
}

fn round(value: f64) -> f64 { (value * 100.0).round() / 100.0 }

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
            samples: Mutex::new(VecDeque::with_capacity(MAX_SAMPLES)),
        }
    }
}
