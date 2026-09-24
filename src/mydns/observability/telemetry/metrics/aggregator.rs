use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::types::{HistoryBucket, MergeableHistogram, OperationalPeriodSnapshot};

const BUCKET_SECONDS: i64 = 60;
const HISTORY: Duration = Duration::from_secs(60 * 60);

pub struct MetricsAggregator {
    // Current period state
    period_start: Mutex<DateTime<Utc>>,
    
    // Traffic
    queries: AtomicU64,
    responses: AtomicU64,
    blocked: AtomicU64,
    record_type_counts: Mutex<HashMap<String, u64>>,
    transport_counts: Mutex<HashMap<String, u64>>,
    
    // Outcomes
    response_code_counts: Mutex<HashMap<String, u64>>,
    resolution_outcome_counts: Mutex<HashMap<String, u64>>,
    resolution_path_counts: Mutex<HashMap<String, u64>>,
    
    // Cache
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    cache_evictions: AtomicU64,
    
    // Upstream
    upstream_requests: AtomicU64,
    upstream_successes: AtomicU64,
    upstream_failures: AtomicU64,
    upstream_timeouts: AtomicU64,
    upstream_retries: AtomicU64,
    
    // Performance
    response_latency: Mutex<MergeableHistogram>,
    upstream_latency: Mutex<MergeableHistogram>,

    // Bucket history
    history: Mutex<HistoryState>,
}

#[derive(Default, Clone)]
struct CounterSnapshot {
    queries: u64,
    responses: u64,
    blocked: u64,
    cache_hits: u64,
    cache_misses: u64,
    cache_evictions: u64,
    upstream_requests: u64,
    upstream_successes: u64,
    upstream_failures: u64,
    upstream_timeouts: u64,
    upstream_retries: u64,
}

struct HistoryState {
    buckets: VecDeque<HistoryBucket>,
    unpersisted_buckets: Vec<HistoryBucket>,
    current_bucket_start: DateTime<Utc>,
    current_response_latency: MergeableHistogram,
    current_upstream_latency: MergeableHistogram,
    last_counters: CounterSnapshot,
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self {
            period_start: Mutex::new(Utc::now()),
            queries: AtomicU64::new(0),
            responses: AtomicU64::new(0),
            blocked: AtomicU64::new(0),
            record_type_counts: Mutex::new(HashMap::new()),
            transport_counts: Mutex::new(HashMap::new()),
            response_code_counts: Mutex::new(HashMap::new()),
            resolution_outcome_counts: Mutex::new(HashMap::new()),
            resolution_path_counts: Mutex::new(HashMap::new()),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_evictions: AtomicU64::new(0),
            upstream_requests: AtomicU64::new(0),
            upstream_successes: AtomicU64::new(0),
            upstream_failures: AtomicU64::new(0),
            upstream_timeouts: AtomicU64::new(0),
            upstream_retries: AtomicU64::new(0),
            response_latency: Mutex::new(MergeableHistogram::default()),
            upstream_latency: Mutex::new(MergeableHistogram::default()),
            history: Mutex::new(HistoryState {
                buckets: VecDeque::new(),
                unpersisted_buckets: Vec::new(),
                current_bucket_start: minute_start(Utc::now()),
                current_response_latency: MergeableHistogram::default(),
                current_upstream_latency: MergeableHistogram::default(),
                last_counters: CounterSnapshot::default(),
            }),
        }
    }
}

impl MetricsAggregator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_query(&self, record_type: &str, transport: &str) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        increment(&self.record_type_counts, record_type);
        increment(&self.transport_counts, transport);
    }

    pub fn record_response(&self, response_code: &str, latency_ms: f64) {
        self.responses.fetch_add(1, Ordering::Relaxed);
        increment(&self.response_code_counts, response_code);
        self.response_latency.lock().unwrap().record(latency_ms);
        
        let mut hist = self.history.lock().unwrap();
        hist.current_response_latency.record(latency_ms);
    }

    pub fn record_blocked(&self, reason: &str) {
        self.blocked.fetch_add(1, Ordering::Relaxed);
        // outcome counts handled via resolution outcome
    }

    pub fn record_resolution(&self, outcome: &str, path: &str) {
        increment(&self.resolution_outcome_counts, outcome);
        increment(&self.resolution_path_counts, path);
    }

    pub fn record_cache(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_cache_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_upstream(&self, latency_ms: f64, success: bool, timeout: bool, retry: bool) {
        self.upstream_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.upstream_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.upstream_failures.fetch_add(1, Ordering::Relaxed);
        }
        if timeout {
            self.upstream_timeouts.fetch_add(1, Ordering::Relaxed);
        }
        if retry {
            self.upstream_retries.fetch_add(1, Ordering::Relaxed);
        }
        
        self.upstream_latency.lock().unwrap().record(latency_ms);
        let mut hist = self.history.lock().unwrap();
        hist.current_upstream_latency.record(latency_ms);
    }

    fn current_counters(&self) -> CounterSnapshot {
        CounterSnapshot {
            queries: self.queries.load(Ordering::Relaxed),
            responses: self.responses.load(Ordering::Relaxed),
            blocked: self.blocked.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            cache_evictions: self.cache_evictions.load(Ordering::Relaxed),
            upstream_requests: self.upstream_requests.load(Ordering::Relaxed),
            upstream_successes: self.upstream_successes.load(Ordering::Relaxed),
            upstream_failures: self.upstream_failures.load(Ordering::Relaxed),
            upstream_timeouts: self.upstream_timeouts.load(Ordering::Relaxed),
            upstream_retries: self.upstream_retries.load(Ordering::Relaxed),
        }
    }

    pub fn try_finalize_period(&self, timezone: &str) -> Option<OperationalPeriodSnapshot> {
        let now = Utc::now();
        let mut period_start = self.period_start.lock().unwrap();
        
        let tz: chrono_tz::Tz = timezone.parse().unwrap_or(chrono_tz::UTC);
        let start_local = period_start.with_timezone(&tz);
        let now_local = now.with_timezone(&tz);
        
        if start_local.date_naive() != now_local.date_naive() || (now - *period_start) >= chrono::Duration::hours(24) {
            let snapshot = OperationalPeriodSnapshot {
                start_utc: *period_start,
                end_utc: now,
                queries: self.queries.swap(0, Ordering::Relaxed),
                responses: self.responses.swap(0, Ordering::Relaxed),
                blocked: self.blocked.swap(0, Ordering::Relaxed),
                record_type_counts: take_map(&self.record_type_counts),
                transport_counts: take_map(&self.transport_counts),
                response_code_counts: take_map(&self.response_code_counts),
                resolution_outcome_counts: take_map(&self.resolution_outcome_counts),
                resolution_path_counts: take_map(&self.resolution_path_counts),
                cache_hits: self.cache_hits.swap(0, Ordering::Relaxed),
                cache_misses: self.cache_misses.swap(0, Ordering::Relaxed),
                cache_evictions: self.cache_evictions.swap(0, Ordering::Relaxed),
                upstream_requests: self.upstream_requests.swap(0, Ordering::Relaxed),
                upstream_successes: self.upstream_successes.swap(0, Ordering::Relaxed),
                upstream_failures: self.upstream_failures.swap(0, Ordering::Relaxed),
                upstream_timeouts: self.upstream_timeouts.swap(0, Ordering::Relaxed),
                upstream_retries: self.upstream_retries.swap(0, Ordering::Relaxed),
                response_latency: take_histogram(&self.response_latency),
                upstream_latency: take_histogram(&self.upstream_latency),
            };
            
            // Sync history counters offset
            let mut hist = self.history.lock().unwrap();
            hist.last_counters = CounterSnapshot::default();
            
            *period_start = now;
            Some(snapshot)
        } else {
            None
        }
    }

    pub fn take_finalized_buckets(&self) -> Vec<HistoryBucket> {
        let now = Utc::now();
        let current_bucket_start = minute_start(now);
        let current_counters = self.current_counters();
        
        let mut hist = self.history.lock().unwrap();
        
        if current_bucket_start > hist.current_bucket_start {
            let request_count = current_counters.queries.saturating_sub(hist.last_counters.queries);
            let error_count = current_counters.upstream_failures.saturating_sub(hist.last_counters.upstream_failures); // Simplified for history
            let upstream_requests = current_counters.upstream_requests.saturating_sub(hist.last_counters.upstream_requests);
            let upstream_failures = current_counters.upstream_failures.saturating_sub(hist.last_counters.upstream_failures);
            
            let bucket = HistoryBucket {
                timestamp: hist.current_bucket_start,
                resolution_seconds: BUCKET_SECONDS as u32,
                request_count,
                error_count,
                response_latency: std::mem::take(&mut hist.current_response_latency),
                upstream_requests,
                upstream_failures,
                upstream_latency: std::mem::take(&mut hist.current_upstream_latency),
            };
            
            hist.buckets.push_back(bucket.clone());
            hist.unpersisted_buckets.push(bucket);
            
            let cutoff = now.timestamp() - HISTORY.as_secs() as i64;
            while hist.buckets.front().map(|b| b.timestamp.timestamp()) < Some(cutoff) {
                hist.buckets.pop_front();
            }
            
            hist.current_bucket_start = current_bucket_start;
            hist.last_counters = current_counters;
        }
        
        std::mem::take(&mut hist.unpersisted_buckets)
    }
    
    pub fn get_in_memory_history(&self) -> Vec<HistoryBucket> {
        let hist = self.history.lock().unwrap();
        hist.buckets.iter().cloned().collect()
    }
}

fn minute_start(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = timestamp.timestamp();
    let start = seconds - seconds.rem_euclid(BUCKET_SECONDS);
    DateTime::<Utc>::from_timestamp(start, 0).expect("valid minute timestamp")
}

fn increment(map: &Mutex<HashMap<String, u64>>, key: &str) {
    let mut map = map.lock().expect("poisoned");
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn take_map(map: &Mutex<HashMap<String, u64>>) -> HashMap<String, u64> {
    std::mem::take(&mut *map.lock().unwrap())
}

fn take_histogram(hist: &Mutex<MergeableHistogram>) -> MergeableHistogram {
    std::mem::take(&mut *hist.lock().unwrap())
}
