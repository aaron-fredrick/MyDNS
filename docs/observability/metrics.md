# Metrics

## Responsibility

Metrics owns numerical measurements and their lifecycle: metric definitions, bounded labels, aggregation, recording, in-process metric state, and metric exposure/export. Metrics may correlate with request or trace context for diagnostics, but trace IDs and other unbounded identifiers must not become metric labels.

Metrics do not own log formatting/output, trace hierarchy/storage, health endpoint semantics, or alert notification delivery. Those components may consume metric signals.

## Current implementation

The existing backend `Metrics` type already provides:

- uptime
- requests/minute
- total DNS queries
- blocked queries
- upstream requests/successes/failures
- upstream availability percentage
- upstream latency avg/p50/p95/p99
- response latency avg/p50/p95/p99
- cache evictions
- DNS errors
- query type counts
- resolution outcome counts
- bounded 24-hour one-minute latency history

This is useful for the dashboard but should be formalized into canonical operational metrics.

## Canonical metric groups

### DNS traffic

| Metric | Type | Labels |
|---|---|---|
| `mydns_dns_queries_total` | counter | transport, record_type |
| `mydns_dns_responses_total` | counter | transport, response_code |
| `mydns_dns_blocked_total` | counter | reason |
| `mydns_dns_request_duration_seconds` | histogram | transport |
| `mydns_dns_active_requests` | gauge | transport |

### DNS resolution

| Metric | Type | Labels |
|---|---|---|
| `mydns_dns_local_lookup_total` | counter | outcome |
| `mydns_dns_cache_hits_total` | counter | cache |
| `mydns_dns_cache_misses_total` | counter | cache |
| `mydns_dns_cache_entries` | gauge | — |
| `mydns_dns_cache_evictions_total` | counter | reason |
| `mydns_dns_record_index_entries` | gauge | — |
| `mydns_dns_zone_count` | gauge | — |
| `mydns_dns_blocklist_entries` | gauge | — |

### Upstream DNS

| Metric | Type | Labels |
|---|---|---|
| `mydns_dns_upstream_requests_total` | counter | upstream |
| `mydns_dns_upstream_responses_total` | counter | upstream, response_code |
| `mydns_dns_upstream_failures_total` | counter | upstream, failure_reason |
| `mydns_dns_upstream_timeouts_total` | counter | upstream |
| `mydns_dns_upstream_retries_total` | counter | upstream |
| `mydns_dns_upstream_duration_seconds` | histogram | upstream |

### HTTP/API

| Metric | Type | Labels |
|---|---|---|
| `mydns_http_requests_total` | counter | method, route, status_class |
| `mydns_http_request_duration_seconds` | histogram | method, route |
| `mydns_http_request_size_bytes` | histogram | method, route |
| `mydns_http_response_size_bytes` | histogram | route, status_class |
| `mydns_http_active_requests` | gauge | — |
| `mydns_http_websocket_connections` | gauge | — |

### Authentication/security

| Metric | Type | Labels |
|---|---|---|
| `mydns_auth_attempts_total` | counter | outcome |
| `mydns_auth_rate_limited_total` | counter | — |
| `mydns_auth_token_validation_failures_total` | counter | reason |

Do not label authentication metrics with usernames or client IPs.

### Database

| Metric | Type | Labels |
|---|---|---|
| `mydns_db_operations_total` | counter | operation, outcome |
| `mydns_db_operation_duration_seconds` | histogram | operation |
| `mydns_db_errors_total` | counter | operation, error_class |
| `mydns_db_transactions_total` | counter | outcome |
| `mydns_db_busy_total` | counter | operation |

Operations must be bounded semantic names.

### Runtime/resources

| Metric | Type | Notes |
|---|---|---|
| `mydns_process_cpu_seconds_total` | counter | process CPU time where available |
| `mydns_process_cpu_utilization_ratio` | gauge | sampled utilization |
| `mydns_process_memory_bytes` | gauge | resident/process memory |
| `mydns_process_open_handles` | gauge | platform-dependent |
| `mydns_storage_free_bytes` | gauge | filesystem containing MyDNS data |
| `mydns_log_directory_bytes` | gauge | total log directory size |
| `mydns_log_file_bytes` | gauge | current log file |
| `mydns_sqlite_database_bytes` | gauge | main DB file |
| `mydns_sqlite_wal_bytes` | gauge | WAL file |
| `mydns_cache_memory_bytes` | gauge | estimated cache memory |
| `mydns_index_memory_bytes` | gauge | estimated index memory |

## Dashboard compatibility

The existing `/api/v1/stats` endpoint should remain a dashboard-facing aggregate endpoint.

It should not become the canonical external metrics protocol.

A future dedicated metrics endpoint/exporter can expose the canonical metric model.

## Histograms

Latency should use histograms for operational aggregation. The current in-memory percentile samples can remain as dashboard history, but should not be treated as a replacement for histogram telemetry.

Recommended DNS/API latency buckets should be tuned around sub-millisecond to multi-second behavior rather than generic web defaults.

## Retention

- counters/gauges: external backend controls retention
- dashboard in-memory history: maximum 24 hours as currently implemented
- local logs: bounded by rotation/retention policy
- resource samples: short in-memory history plus optional external export

## Metric correctness

Do not derive availability as 100% when there have been no upstream requests and then use that value directly for an alert. A no-traffic state is distinct from an observed healthy upstream.


## Operational period and historical retention

The metrics lifecycle is split into two related but distinct forms of data:

1. **24-hour operational period aggregate** — answers what happened during the current completed operational period.
2. **Historical performance time series** — answers how system performance changed over time.

The 24-hour operational period is not a lifetime counter. Counters, categorical breakdowns, and period aggregates accumulate in memory during the current period. At the 24-hour boundary, the period is finalized as a snapshot and persisted to SQLite. The in-memory operational counters and breakdowns are then reset for the next period.

The period snapshot may contain:

- total DNS queries
- blocked queries
- upstream requests, successes, and failures
- DNS errors
- cache evictions
- query-type breakdowns
- resolution-outcome breakdowns
- aggregate response latency
- aggregate upstream latency
- other finalized statistics belonging to that operational period

The period snapshot is independent of the historical time-series retention policy. It is a finalized 24-hour operational summary, not another copy of the time-series buckets.

### Historical performance resolution

Historical performance data is retained at progressively coarser resolutions as it ages:

| Data age | Resolution | Purpose |
|---|---:|---|
| Current 0–24 hours | 1 minute | Detailed operational analysis |
| 24–48 hours | 1 hour | Short-term historical comparison |
| ~2–7 days | 3–6 hours | Weekly trends |
| ~1 week–1 month | 12 hours | Monthly trends |
| ~1 month–1 year | 1 day | Long-term/yearly trends |
| Beyond 1 year | TBD | Retention/archive policy |

The exact weekly roll-up resolution (3 or 6 hours) remains configurable and can be finalized during implementation. Storage tiers should primarily be age-based; calendar week/month/year presentation belongs to the API/dashboard layer rather than defining the physical retention boundaries.

The roll-up lifecycle is:

```
1-minute buckets
      ↓
1-hour buckets
      ↓
3/6-hour buckets
      ↓
12-hour buckets
      ↓
1-day buckets
```

Once a coarser aggregate has been safely persisted, older finer-grained data can be removed according to the retention policy.

### In-memory versus SQLite ownership

SQLite is the authoritative store for retained historical metrics. RAM is an operational cache and hot-path aggregation layer.

```
RAM
├── current 24-hour operational tally
└── recent ~1 hour of 1-minute performance buckets

SQLite
├── finalized 24-hour operational snapshots
├── 1-minute history for the current 24 hours
├── 1-hour history for the previous 24–48 hours
├── 3/6-hour history for the weekly range
├── 12-hour history for the monthly range
└── 1-day history for the yearly range
```

The DNS/request hot path should update only in-memory counters and time buckets. A background persistence task writes finalized minute buckets to SQLite. Background roll-up work produces the coarser historical tiers.

Keeping approximately one hour of minute buckets in RAM is a performance optimization; it does not define the historical retention period. SQLite remains authoritative for historical queries and recovery of retained history.

### Historical bucket contents

Historical buckets should store aggregated observations rather than retaining individual latency samples for the entire retention period. A minute bucket should conceptually contain values such as:

- timestamp
- request count
- error count
- response-latency aggregate
- upstream request count
- upstream failure count
- upstream-latency aggregate
- other bounded performance aggregates needed by the dashboard

Percentiles such as p50, p95, and p99 must use a mergeable distribution representation (for example, an appropriate histogram/sketch) when they need to survive roll-up. Percentiles must not be averaged across buckets, because an average of percentiles is not generally a valid percentile of the combined population.

### Derived metrics

Metrics such as requests per minute, error rate, upstream availability, cache hit rate, and throughput should be derived from their underlying counters or aggregated buckets where practical. They should not be maintained as independent sources of truth when the underlying measurements already exist.

This preserves the distinction between:

- **event counters** — queries, blocks, upstream requests, failures, evictions, errors
- **categorical breakdowns** — query types and resolution outcomes
- **performance distributions** — response and upstream latency
- **current-state gauges** — cache size, record count, blocklist size, and similar state
- **calculated metrics** — rates, percentages, throughput, and other derived values

This retention model extends the existing dashboard history described above; it does not remove the existing metric groups, dashboard endpoint, histogram guidance, or correctness requirements.
