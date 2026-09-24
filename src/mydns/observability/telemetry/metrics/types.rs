use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MergeableHistogram {
    pub count: u64,
    pub sum: f64,
    // Buckets: e.g. <=1ms, <=5ms, <=10ms, <=50ms, <=100ms, <=500ms, <=1000ms, >1000ms
    pub buckets: [u64; 8],
}

impl MergeableHistogram {
    pub fn record(&mut self, value_ms: f64) {
        if !value_ms.is_finite() || value_ms < 0.0 {
            return;
        }
        self.count += 1;
        self.sum += value_ms;
        
        if value_ms <= 1.0 { self.buckets[0] += 1; }
        else if value_ms <= 5.0 { self.buckets[1] += 1; }
        else if value_ms <= 10.0 { self.buckets[2] += 1; }
        else if value_ms <= 50.0 { self.buckets[3] += 1; }
        else if value_ms <= 100.0 { self.buckets[4] += 1; }
        else if value_ms <= 500.0 { self.buckets[5] += 1; }
        else if value_ms <= 1000.0 { self.buckets[6] += 1; }
        else { self.buckets[7] += 1; }
    }

    pub fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.sum += other.sum;
        for i in 0..8 {
            self.buckets[i] += other.buckets[i];
        }
    }
}

/// A finalized 24-hour operational period snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalPeriodSnapshot {
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
    
    // Traffic
    pub queries: u64,
    pub responses: u64,
    pub blocked: u64,
    pub record_type_counts: HashMap<String, u64>,
    pub transport_counts: HashMap<String, u64>,
    
    // Outcomes
    pub response_code_counts: HashMap<String, u64>,
    pub resolution_outcome_counts: HashMap<String, u64>,
    pub resolution_path_counts: HashMap<String, u64>,
    
    // Cache
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    
    // Upstream
    pub upstream_requests: u64,
    pub upstream_successes: u64,
    pub upstream_failures: u64,
    pub upstream_timeouts: u64,
    pub upstream_retries: u64,
    
    // Performance
    pub response_latency: MergeableHistogram,
    pub upstream_latency: MergeableHistogram,
}

/// A finalized historical bucket (1-min, 1-hour, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryBucket {
    pub timestamp: DateTime<Utc>,
    pub resolution_seconds: u32,
    
    pub request_count: u64,
    pub error_count: u64,
    
    pub response_latency: MergeableHistogram,
    
    pub upstream_requests: u64,
    pub upstream_failures: u64,
    pub upstream_latency: MergeableHistogram,
}
