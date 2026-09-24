use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::TimeZone;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use chrono_tz::Tz;

use super::types::{BoundedCounts, HistoryBucket, MergeableHistogram, OperationalPeriodSnapshot};

const BUCKET_SECONDS: i64 = 60;
const RECENT_HISTORY_SECONDS: i64 = 60 * 60;

struct OperationalState {
    period_start: DateTime<Utc>,
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
    blocked_reason_counts: BoundedCounts,
    record_type_counts: BoundedCounts,
    transport_counts: BoundedCounts,
    response_code_counts: BoundedCounts,
    resolution_outcome_counts: BoundedCounts,
    resolution_path_counts: BoundedCounts,
    response_latency: MergeableHistogram,
    upstream_latency: MergeableHistogram,
}

impl OperationalState {
    fn new(period_start: DateTime<Utc>) -> Self {
        Self {
            period_start,
            queries: 0,
            responses: 0,
            blocked: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            upstream_requests: 0,
            upstream_successes: 0,
            upstream_failures: 0,
            upstream_timeouts: 0,
            upstream_retries: 0,
            blocked_reason_counts: BoundedCounts::default(),
            record_type_counts: BoundedCounts::default(),
            transport_counts: BoundedCounts::default(),
            response_code_counts: BoundedCounts::default(),
            resolution_outcome_counts: BoundedCounts::default(),
            resolution_path_counts: BoundedCounts::default(),
            response_latency: MergeableHistogram::response(),
            upstream_latency: MergeableHistogram::upstream(),
        }
    }

    fn snapshot_and_reset(&mut self, end: DateTime<Utc>, timezone: Tz) -> OperationalPeriodSnapshot {
        let snapshot = OperationalPeriodSnapshot {
            timezone: timezone.to_string(),
            start_utc: self.period_start,
            end_utc: end,
            queries: self.queries,
            responses: self.responses,
            blocked: self.blocked,
            blocked_reason_counts: std::mem::take(&mut self.blocked_reason_counts),
            record_type_counts: std::mem::take(&mut self.record_type_counts),
            transport_counts: std::mem::take(&mut self.transport_counts),
            response_code_counts: std::mem::take(&mut self.response_code_counts),
            resolution_outcome_counts: std::mem::take(&mut self.resolution_outcome_counts),
            resolution_path_counts: std::mem::take(&mut self.resolution_path_counts),
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            cache_evictions: self.cache_evictions,
            upstream_requests: self.upstream_requests,
            upstream_successes: self.upstream_successes,
            upstream_failures: self.upstream_failures,
            upstream_timeouts: self.upstream_timeouts,
            upstream_retries: self.upstream_retries,
            response_latency: std::mem::take(&mut self.response_latency),
            upstream_latency: std::mem::take(&mut self.upstream_latency),
        };

        self.period_start = end;
        self.queries = 0;
        self.responses = 0;
        self.blocked = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.cache_evictions = 0;
        self.upstream_requests = 0;
        self.upstream_successes = 0;
        self.upstream_failures = 0;
        self.upstream_timeouts = 0;
        self.upstream_retries = 0;
        self
            .response_latency = MergeableHistogram::response();
        self.upstream_latency = MergeableHistogram::upstream();

        snapshot
    }
}

struct HistoryState {
    current: HistoryBucket,
    recent: VecDeque<HistoryBucket>,
    pending: VecDeque<HistoryBucket>,
}

pub struct MetricsAggregator {
    timezone: Tz,
    state: Mutex<AggregatorState>,
}

struct AggregatorState {
    operational: OperationalState,
    pending_periods: VecDeque<OperationalPeriodSnapshot>,
    history: HistoryState,
}

impl MetricsAggregator {
    pub fn new(timezone: Tz) -> Arc<Self> {
        let now = Utc::now();
        Self::new_at(timezone, now)
    }

    fn new_at(timezone: Tz, now: DateTime<Utc>) -> Arc<Self> {
        Arc::new(Self {
            timezone,
            state: Mutex::new(AggregatorState {
                operational: OperationalState::new(local_midnight_utc(now, timezone)),
                pending_periods: VecDeque::new(),
                history: HistoryState {
                    current: HistoryBucket::new(minute_start(now)),
                    recent: VecDeque::new(),
                    pending: VecDeque::new(),
                },
            }),
        })
    }

    pub fn record_query(&self, record_type: &str, transport: &str) {
        self.record_query_at(Utc::now(), record_type, transport);
    }

    pub fn record_response(&self, response_code: &str, latency_ms: f64) {
        self.record_response_at(Utc::now(), response_code, latency_ms);
    }

    pub fn record_blocked(&self, reason: &str) {
        self.record_blocked_at(Utc::now(), reason);
    }

    pub fn record_resolution(&self, outcome: &str, path: &str) {
        self.record_resolution_at(Utc::now(), outcome, path);
    }

    pub fn record_cache(&self, hit: bool) {
        self.record_cache_at(Utc::now(), hit);
    }

    pub fn record_cache_eviction(&self) {
        self.record_cache_eviction_at(Utc::now());
    }

    pub fn record_upstream(&self, latency_ms: f64, success: bool, timeout: bool, retry: bool) {
        self.record_upstream_at(Utc::now(), latency_ms, success, timeout, retry);
    }

    pub fn record_query_at(
        &self,
        now: DateTime<Utc>,
        record_type: &str,
        transport: &str,
    ) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        let record_type = normalize_record_type(record_type);
        let transport = normalize_transport(transport);
        state.operational.queries += 1;
        state.operational.record_type_counts.record(&record_type);
        state.operational.transport_counts.record(&transport);
        state.history.current.request_count += 1;
        state.history.current.record_type_counts.record(&record_type);
        state.history.current.transport_counts.record(&transport);
    }

    pub fn record_response_at(
        &self,
        now: DateTime<Utc>,
        response_code: &str,
        latency_ms: f64,
    ) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        let response_code = normalize_response_code(response_code);
        state.operational.responses += 1;
        state.operational.response_code_counts.record(&response_code);
        state.operational.response_latency.record(latency_ms);
        state.history.current.response_count += 1;
        state.history.current.response_code_counts.record(&response_code);
        state.history.current.response_latency.record(latency_ms);
    }

    pub fn record_blocked_at(&self, now: DateTime<Utc>, reason: &str) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        let reason = normalize_reason(reason);
        state.operational.blocked += 1;
        state.operational.blocked_reason_counts.record(&reason);
        state.history.current.blocked_count += 1;
        state.history.current.blocked_reason_counts.record(&reason);
    }

    pub fn record_resolution_at(&self, now: DateTime<Utc>, outcome: &str, path: &str) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        let outcome = normalize_dimension(outcome);
        let path = normalize_dimension(path);
        state.operational.resolution_outcome_counts.record(&outcome);
        state.operational.resolution_path_counts.record(&path);
        state.history.current.resolution_outcome_counts.record(&outcome);
        state.history.current.resolution_path_counts.record(&path);
    }

    pub fn record_cache_at(&self, now: DateTime<Utc>, hit: bool) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        if hit {
            state.operational.cache_hits += 1;
            state.history.current.cache_hits += 1;
        } else {
            state.operational.cache_misses += 1;
            state.history.current.cache_misses += 1;
        }
    }

    pub fn record_cache_eviction_at(&self, now: DateTime<Utc>) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);
        state.operational.cache_evictions += 1;
        state.history.current.cache_evictions += 1;
    }

    pub fn record_upstream_at(
        &self,
        now: DateTime<Utc>,
        latency_ms: f64,
        success: bool,
        timeout: bool,
        retry: bool,
    ) {
        let mut state = self.state.lock().unwrap();
        advance(&mut state, now, self.timezone);

        state.operational.upstream_requests += 1;
        if success {
            state.operational.upstream_successes += 1;
        } else {
            state.operational.upstream_failures += 1;
        }
        if timeout {
            state.operational.upstream_timeouts += 1;
        }
        if retry {
            state.operational.upstream_retries += 1;
        }
        state.operational.upstream_latency.record(latency_ms);

        state.history.current.upstream_requests += 1;
        if success {
            state.history.current.upstream_successes += 1;
        } else {
            state.history.current.upstream_failures += 1;
        }
        if timeout {
            state.history.current.upstream_timeouts += 1;
        }
        if retry {
            state.history.current.upstream_retries += 1;
        }
        state.history.current.upstream_latency.record(latency_ms);
    }

    pub fn finalize_due_periods(&self, now: DateTime<Utc>) {
        let mut state = self.state.lock().unwrap();
        advance_periods(&mut state, now, self.timezone);
    }

    pub fn pending_periods(&self) -> Vec<OperationalPeriodSnapshot> {
        self.state
            .lock()
            .unwrap()
            .pending_periods
            .iter()
            .cloned()
            .collect()
    }

    pub fn acknowledge_periods(&self, periods: &[OperationalPeriodSnapshot]) {
        let keys: Vec<_> = periods.iter().map(|p| (p.start_utc, p.end_utc)).collect();
        self.state
            .lock()
            .unwrap()
            .pending_periods
            .retain(|p| !keys.contains(&(p.start_utc, p.end_utc)));
    }

    pub fn finalize_completed_buckets(&self, now: DateTime<Utc>) {
        let mut state = self.state.lock().unwrap();
        advance_history(&mut state, now);
    }

    pub fn pending_buckets(&self) -> Vec<HistoryBucket> {
        self.state
            .lock()
            .unwrap()
            .history
            .pending
            .iter()
            .cloned()
            .collect()
    }

    pub fn acknowledge_buckets(&self, buckets: &[HistoryBucket]) {
        let keys: Vec<_> = buckets
            .iter()
            .map(|b| (b.timestamp, b.resolution_seconds))
            .collect();
        self.state
            .lock()
            .unwrap()
            .history
            .pending
            .retain(|b| !keys.contains(&(b.timestamp, b.resolution_seconds)));
    }

    pub fn get_in_memory_history(&self) -> Vec<HistoryBucket> {
        self.state
            .lock()
            .unwrap()
            .history
            .recent
            .iter()
            .cloned()
            .collect()
    }
}

fn advance(state: &mut AggregatorState, now: DateTime<Utc>, timezone: Tz) {
    advance_periods(state, now, timezone);
    advance_history(state, now);
}

fn advance_periods(state: &mut AggregatorState, now: DateTime<Utc>, timezone: Tz) {
    loop {
        let start = state.operational.period_start;
        let end = next_local_midnight(start, timezone);
        if now < end {
            break;
        }

        let snapshot = state.operational.snapshot_and_reset(end, timezone);
        state.pending_periods.push_back(snapshot);
    }
}

fn advance_history(state: &mut AggregatorState, now: DateTime<Utc>) {
    let target = minute_start(now);

    while state.history.current.timestamp < target {
        let next = state.history.current.timestamp + ChronoDuration::seconds(BUCKET_SECONDS);
        let completed = std::mem::replace(
            &mut state.history.current,
            HistoryBucket::new(next),
        );
        state.history.pending.push_back(completed.clone());
        state.history.recent.push_back(completed);
    }

    let cutoff = target - ChronoDuration::seconds(RECENT_HISTORY_SECONDS);
    while state
        .history
        .recent
        .front()
        .is_some_and(|bucket| bucket.timestamp < cutoff)
    {
        state.history.recent.pop_front();
    }
}

fn normalize_record_type(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn normalize_response_code(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn normalize_transport(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_reason(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_dimension(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn minute_start(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = timestamp.timestamp();
    DateTime::<Utc>::from_timestamp(seconds - seconds.rem_euclid(BUCKET_SECONDS), 0)
        .expect("valid minute timestamp")
}

fn local_midnight_utc(now: DateTime<Utc>, timezone: Tz) -> DateTime<Utc> {
    let local = now.with_timezone(&timezone);
    let midnight = local
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid local midnight");

    timezone
        .from_local_datetime(&midnight)
        .single()
        .expect("valid local midnight")
        .with_timezone(&Utc)
}

fn next_local_midnight(start_utc: DateTime<Utc>, timezone: Tz) -> DateTime<Utc> {
    let next_date: NaiveDate = start_utc
        .with_timezone(&timezone)
        .date_naive()
        .succ_opt()
        .expect("valid next date");
    let midnight = next_date
        .and_hms_opt(0, 0, 0)
        .expect("valid local midnight");

    timezone
        .from_local_datetime(&midnight)
        .single()
        .expect("valid local midnight")
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono_tz::Asia::Colombo;

    fn at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn records_stay_in_the_correct_minute_bucket() {
        let metrics = MetricsAggregator::new_at(Colombo, at("2026-09-24T00:00:00Z"));

        metrics.record_query_at(at("2026-09-24T00:00:59Z"), "a", "UDP");
        metrics.record_query_at(at("2026-09-24T00:01:01Z"), "aaaa", "UDP");

        let history = metrics.get_in_memory_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].request_count, 1);
        assert_eq!(history[1].request_count, 1);
        assert_eq!(history[0].record_type_counts.get("A"), 1);
        assert_eq!(history[1].record_type_counts.get("AAAA"), 1);
    }

    #[test]
    fn operational_period_uses_configured_local_midnight() {
        let metrics = MetricsAggregator::new_at(Colombo, at("2026-09-23T18:29:00Z"));
        metrics.record_query_at(at("2026-09-23T18:29:30Z"), "A", "udp");

        metrics.finalize_due_periods(at("2026-09-23T18:30:00Z"));

        let periods = metrics.pending_periods();
        assert_eq!(periods.len(), 1);
        assert_eq!(periods[0].queries, 1);
        assert_eq!(periods[0].start_utc, at("2026-09-22T18:30:00Z"));
        assert_eq!(periods[0].end_utc, at("2026-09-23T18:30:00Z"));
    }

    #[test]
    fn failed_persistence_keeps_pending_data_until_acknowledged() {
        let metrics = MetricsAggregator::new_at(chrono_tz::UTC, at("2026-09-24T00:00:00Z"));
        metrics.record_query_at(at("2026-09-24T00:00:10Z"), "A", "udp");
        metrics.finalize_completed_buckets(at("2026-09-24T00:01:00Z"));

        assert_eq!(metrics.pending_buckets().len(), 1);
        assert_eq!(metrics.pending_buckets()[0].request_count, 1);

        metrics.acknowledge_buckets(&metrics.pending_buckets());
        assert!(metrics.pending_buckets().is_empty());
    }
}
