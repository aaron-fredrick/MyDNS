use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use chrono_tz::Tz;
use chrono::TimeZone;

use super::types::{BoundedCounts, HistoryBucket, MergeableHistogram, OperationalPeriodSnapshot};

const BUCKET_SECONDS: i64 = 60;
const RECENT_HISTORY_SECONDS: i64 = 60 * 60;

pub struct MetricsAggregator {
    timezone: Tz,
    period_start: Mutex<DateTime<Utc>>,
    pending_periods: Mutex<VecDeque<OperationalPeriodSnapshot>>,
    queries: AtomicU64, responses: AtomicU64, blocked: AtomicU64,
    cache_hits: AtomicU64, cache_misses: AtomicU64, cache_evictions: AtomicU64,
    upstream_requests: AtomicU64, upstream_successes: AtomicU64, upstream_failures: AtomicU64,
    upstream_timeouts: AtomicU64, upstream_retries: AtomicU64,
    record_type_counts: Mutex<BoundedCounts>, transport_counts: Mutex<BoundedCounts>,
    response_code_counts: Mutex<BoundedCounts>, resolution_outcome_counts: Mutex<BoundedCounts>,
    resolution_path_counts: Mutex<BoundedCounts>,
    response_latency: Mutex<MergeableHistogram>, upstream_latency: Mutex<MergeableHistogram>,
    history: Mutex<HistoryState>,
}

struct HistoryState { current: HistoryBucket, recent: VecDeque<HistoryBucket>, pending: VecDeque<HistoryBucket> }

impl MetricsAggregator {
    pub fn new(timezone: Tz) -> Arc<Self> {
        let now = Utc::now();
        Arc::new(Self {
            timezone,
            period_start: Mutex::new(local_midnight_utc(now, timezone)),
            pending_periods: Mutex::new(VecDeque::new()),
            queries: AtomicU64::new(0), responses: AtomicU64::new(0), blocked: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0), cache_misses: AtomicU64::new(0), cache_evictions: AtomicU64::new(0),
            upstream_requests: AtomicU64::new(0), upstream_successes: AtomicU64::new(0),
            upstream_failures: AtomicU64::new(0), upstream_timeouts: AtomicU64::new(0), upstream_retries: AtomicU64::new(0),
            record_type_counts: Mutex::new(BoundedCounts::default()), transport_counts: Mutex::new(BoundedCounts::default()),
            response_code_counts: Mutex::new(BoundedCounts::default()), resolution_outcome_counts: Mutex::new(BoundedCounts::default()),
            resolution_path_counts: Mutex::new(BoundedCounts::default()),
            response_latency: Mutex::new(MergeableHistogram::response()), upstream_latency: Mutex::new(MergeableHistogram::upstream()),
            history: Mutex::new(HistoryState {
                current: HistoryBucket::new(minute_start(now)), recent: VecDeque::new(), pending: VecDeque::new(),
            }),
        })
    }

    pub fn record_query(&self, record_type: &str, transport: &str) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        self.record_type_counts.lock().unwrap().record(record_type);
        self.transport_counts.lock().unwrap().record(transport);
        let mut h = self.history.lock().unwrap();
        h.current.request_count += 1;
        h.current.record_type_counts.record(record_type);
        h.current.transport_counts.record(transport);
    }

    pub fn record_response(&self, response_code: &str, latency_ms: f64) {
        self.responses.fetch_add(1, Ordering::Relaxed);
        self.response_code_counts.lock().unwrap().record(response_code);
        self.response_latency.lock().unwrap().record(latency_ms);
        let mut h = self.history.lock().unwrap();
        h.current.response_count += 1;
        h.current.response_code_counts.record(response_code);
        h.current.response_latency.record(latency_ms);
    }

    pub fn record_blocked(&self, _reason: &str) {
        self.blocked.fetch_add(1, Ordering::Relaxed);
        self.history.lock().unwrap().current.blocked_count += 1;
    }

    pub fn record_resolution(&self, outcome: &str, path: &str) {
        self.resolution_outcome_counts.lock().unwrap().record(outcome);
        self.resolution_path_counts.lock().unwrap().record(path);
        let mut h = self.history.lock().unwrap();
        h.current.resolution_outcome_counts.record(outcome);
        h.current.resolution_path_counts.record(path);
    }

    pub fn record_cache(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            self.history.lock().unwrap().current.cache_hits += 1;
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            self.history.lock().unwrap().current.cache_misses += 1;
        }
    }

    pub fn record_cache_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
        self.history.lock().unwrap().current.cache_evictions += 1;
    }

    pub fn record_upstream(&self, latency_ms: f64, success: bool, timeout: bool, retry: bool) {
        self.upstream_requests.fetch_add(1, Ordering::Relaxed);
        if success { self.upstream_successes.fetch_add(1, Ordering::Relaxed); }
        else { self.upstream_failures.fetch_add(1, Ordering::Relaxed); }
        if timeout { self.upstream_timeouts.fetch_add(1, Ordering::Relaxed); }
        if retry { self.upstream_retries.fetch_add(1, Ordering::Relaxed); }
        self.upstream_latency.lock().unwrap().record(latency_ms);

        let mut h = self.history.lock().unwrap();
        h.current.upstream_requests += 1;
        if success { h.current.upstream_successes += 1; } else { h.current.upstream_failures += 1; }
        if timeout { h.current.upstream_timeouts += 1; }
        if retry { h.current.upstream_retries += 1; }
        h.current.upstream_latency.record(latency_ms);
    }

    pub fn finalize_due_periods(&self, now: DateTime<Utc>) {
        loop {
            let start = *self.period_start.lock().unwrap();
            let end = next_local_midnight(start, self.timezone);
            if now < end { break; }

            let snapshot = OperationalPeriodSnapshot {
                start_utc: start, end_utc: end,
                queries: self.queries.swap(0, Ordering::Relaxed), responses: self.responses.swap(0, Ordering::Relaxed),
                blocked: self.blocked.swap(0, Ordering::Relaxed),
                record_type_counts: std::mem::take(&mut *self.record_type_counts.lock().unwrap()),
                transport_counts: std::mem::take(&mut *self.transport_counts.lock().unwrap()),
                response_code_counts: std::mem::take(&mut *self.response_code_counts.lock().unwrap()),
                resolution_outcome_counts: std::mem::take(&mut *self.resolution_outcome_counts.lock().unwrap()),
                resolution_path_counts: std::mem::take(&mut *self.resolution_path_counts.lock().unwrap()),
                cache_hits: self.cache_hits.swap(0, Ordering::Relaxed), cache_misses: self.cache_misses.swap(0, Ordering::Relaxed),
                cache_evictions: self.cache_evictions.swap(0, Ordering::Relaxed),
                upstream_requests: self.upstream_requests.swap(0, Ordering::Relaxed),
                upstream_successes: self.upstream_successes.swap(0, Ordering::Relaxed),
                upstream_failures: self.upstream_failures.swap(0, Ordering::Relaxed),
                upstream_timeouts: self.upstream_timeouts.swap(0, Ordering::Relaxed),
                upstream_retries: self.upstream_retries.swap(0, Ordering::Relaxed),
                response_latency: std::mem::replace(&mut *self.response_latency.lock().unwrap(), MergeableHistogram::response()),
                upstream_latency: std::mem::replace(&mut *self.upstream_latency.lock().unwrap(), MergeableHistogram::upstream()),
            };
            self.pending_periods.lock().unwrap().push_back(snapshot);
            *self.period_start.lock().unwrap() = end;
        }
    }

    pub fn pending_periods(&self) -> Vec<OperationalPeriodSnapshot> {
        self.pending_periods.lock().unwrap().iter().cloned().collect()
    }

    pub fn acknowledge_periods(&self, periods: &[OperationalPeriodSnapshot]) {
        let keys: Vec<_> = periods.iter().map(|p| (p.start_utc, p.end_utc)).collect();
        self.pending_periods.lock().unwrap().retain(|p| !keys.contains(&(p.start_utc, p.end_utc)));
    }

    pub fn finalize_completed_buckets(&self, now: DateTime<Utc>) {
        let target = minute_start(now);
        let mut h = self.history.lock().unwrap();
        while h.current.timestamp < target {
            let next = h.current.timestamp + ChronoDuration::seconds(BUCKET_SECONDS);
            let completed = std::mem::replace(&mut h.current, HistoryBucket::new(next));
            h.pending.push_back(completed.clone());
            h.recent.push_back(completed);
        }
        let cutoff = target - ChronoDuration::seconds(RECENT_HISTORY_SECONDS);
        while h.recent.front().is_some_and(|b| b.timestamp < cutoff) { h.recent.pop_front(); }
    }

    pub fn pending_buckets(&self) -> Vec<HistoryBucket> {
        self.history.lock().unwrap().pending.iter().cloned().collect()
    }

    pub fn acknowledge_buckets(&self, buckets: &[HistoryBucket]) {
        let keys: Vec<_> = buckets.iter().map(|b| (b.timestamp, b.resolution_seconds)).collect();
        self.history.lock().unwrap().pending.retain(|b| !keys.contains(&(b.timestamp, b.resolution_seconds)));
    }

    pub fn get_in_memory_history(&self) -> Vec<HistoryBucket> {
        self.history.lock().unwrap().recent.iter().cloned().collect()
    }
}

fn minute_start(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = timestamp.timestamp();
    DateTime::<Utc>::from_timestamp(seconds - seconds.rem_euclid(BUCKET_SECONDS), 0).expect("valid minute timestamp")
}

fn local_midnight_utc(now: DateTime<Utc>, timezone: Tz) -> DateTime<Utc> {
    let local = now.with_timezone(&timezone);
    let midnight = local.date_naive().and_hms_opt(0, 0, 0).expect("valid local midnight");
    timezone.from_local_datetime(&midnight).single().expect("valid local midnight").with_timezone(&Utc)
}

fn next_local_midnight(start_utc: DateTime<Utc>, timezone: Tz) -> DateTime<Utc> {
    let next_date: NaiveDate = start_utc.with_timezone(&timezone).date_naive().succ_opt().expect("valid next date");
    let midnight = next_date.and_hms_opt(0, 0, 0).expect("valid local midnight");
    timezone.from_local_datetime(&midnight).single().expect("valid local midnight").with_timezone(&Utc)
}
