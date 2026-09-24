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


### Timezone and period boundaries

Metrics use the global MyDNS application timezone for calendar-aware period semantics. The timezone is configured once at the application level and is not independently selected by the metrics subsystem.

Use UTC internally for metric timestamps, bucket ordering, elapsed-time calculations, sliding windows, retention age, and cross-event correlation. Use the configured timezone when a metric operation depends on a calendar boundary.

In particular:

- **Sliding windows** such as "last 24 hours" are elapsed-time windows and are timezone-independent.
- **Calendar-aware periods** such as a local operational day, calendar week, month, or year use the configured timezone.
- **24-hour operational snapshots** use the configured timezone when their boundary is defined as a local operational-day boundary.
- Historical storage remains timestamped in UTC even when the dashboard presents data in local time.
- The host operating system's timezone must not implicitly change metric behavior.

The configured timezone should be a canonical application setting, such as system.timezone, rather than a metrics-specific setting.


## Measurement model and metric lifecycle

The canonical metric model is based on four stages:

```
Observation
    ↓
In-memory accumulation
    ↓
Time-period aggregation
    ↓
Derived presentation
```

An individual DNS, HTTP, database, or resource event produces one or more metric observations. The hot path updates bounded in-memory state. Finalized buckets and operational periods are persisted asynchronously. The API/dashboard derives rates, percentages, throughput, and percentile values from the persisted or in-memory aggregates.

The same underlying observation may therefore contribute to both the current operational period and the historical time series.

### Metric data classes

Every metric should be classified as one of these data classes:

| Class | Meaning | Examples | Aggregation rule |
|---|---|---|---|
| Event counter | Number of occurrences | queries, blocked requests, errors, upstream requests, cache hits, evictions | sum |
| Categorical count | Number of occurrences by bounded category | record type, response code, resolution outcome, resolution path | sum per category |
| Distribution | Distribution of measured values | response latency, upstream latency, HTTP latency, DB latency | merge distribution |
| Gauge | Current state/value | cache entries, records, zones, blocklist entries, memory, CPU utilization | current value; optionally sample historically |
| Derived metric | Calculated from primitive measurements | RPS/RPM, error rate, hit rate, availability, percentages | calculate on read |

Derived metrics are not independent counters. Their underlying primitive measurements are the source of truth.

### DNS observations

For a completed DNS request, the metrics pipeline should capture the bounded facts needed to answer:

- how many requests occurred?
- what record types were requested?
- what transport handled them?
- what response code/outcome was produced?
- was the request blocked?
- which resolution path handled it?
- did the request hit or miss cache?
- did it require upstream resolution?
- how long did the DNS operation take?
- if upstream was used, how long did upstream resolution take?
- did the upstream operation succeed, fail, timeout, or retry?

The DNS metric pipeline should therefore measure at least:

```
DNS traffic
├── request count
├── response count
├── blocked count
├── record-type counts
└── transport counts

DNS outcome
├── response-code counts
├── resolution-outcome counts
└── resolution-path counts

Cache
├── hits
├── misses
└── evictions

Upstream
├── requests
├── successes
├── failures
├── timeouts
└── retries

Performance
├── total DNS response latency distribution
└── upstream latency distribution
```

The existing canonical metric names remain the external vocabulary. The additional resolution-path and bounded categorical measurements are required so that the operational aggregate and historical buckets can explain where DNS traffic was handled, not only whether it succeeded.

### 1-minute performance bucket

Each finalized one-minute bucket is a compact summary of the observations belonging to that minute.

It should contain, as applicable:

```
timestamp
request_count
response_count
blocked_count

response_code_counts
record_type_counts
resolution_outcome_counts
resolution_path_counts

cache_hits
cache_misses
cache_evictions

upstream_requests
upstream_successes
upstream_failures
upstream_timeouts
upstream_retries

response_latency_distribution
upstream_latency_distribution
```

All categorical sets must remain bounded and normalized. Unknown or unexpected values should fall into bounded `OTHER`/equivalent categories rather than creating unbounded metric cardinality.

The bucket should store mergeable distribution state for latency rather than a long-lived vector of individual samples. From that state the API can calculate:

- request rate / requests per minute
- error rate
- blocked rate
- cache hit rate
- upstream failure rate
- upstream availability
- response latency average and quantiles
- upstream latency average and quantiles

The primitive counts and distributions remain authoritative; displayed rates and percentages are derived.

### 24-hour operational aggregate

The current operational period should accumulate the same primitive classes needed for a complete period summary:

```
Traffic
├── queries / responses
├── blocked
├── record-type counts
└── transport counts

Outcomes
├── response-code counts
├── resolution-outcome counts
└── resolution-path counts

Cache
├── hits
├── misses
└── evictions

Upstream
├── requests
├── successes
├── failures
├── timeouts
└── retries

Performance
├── response latency distribution
└── upstream latency distribution
```

At period finalization, the aggregate is persisted with its start/end timestamps. It must be sufficient to calculate the period's traffic volume, error/blocked rates, cache hit rate, upstream availability/failure rate, and response/upstream latency statistics without retaining individual request observations.

If the operational period is calendar-based, its boundary uses the configured application timezone. If it is defined as a fixed elapsed 24-hour window, its boundary is timezone-independent.

### Roll-up requirements

Historical roll-ups must operate on mergeable primitives.

Counters and categorical counts are summed:

```
combined_count = bucket_a.count + bucket_b.count
```

For averages, retain enough information to calculate a weighted result, such as count plus sum:

```
combined_average =
    (sum_a + sum_b) / (count_a + count_b)
```

For p50/p95/p99 and other quantiles, merge the underlying distribution representation. Never calculate a higher-level percentile by averaging lower-level percentile values.

This makes the following pipeline valid:

```
1-minute distribution
        ↓
merge
1-hour distribution
        ↓
merge
3/6-hour distribution
        ↓
merge
12-hour distribution
        ↓
merge
1-day distribution
```

### HTTP, database, and resource measurements

The same aggregation model applies outside DNS.

HTTP should measure request count, status class, bounded route/method dimensions, request/response sizes, request latency, active requests, and WebSocket connection state.

Database instrumentation should measure bounded operation counts, outcomes, errors, busy events, and operation latency.

Resource telemetry should measure current process/system state such as CPU, memory, open handles, storage, SQLite/WAL size, cache memory, and index memory. Resource gauges may optionally be sampled into historical buckets where dashboard trend analysis requires it.

These measurements should use the same observation → accumulation → bucket → roll-up lifecycle rather than introducing a separate metrics architecture for each subsystem.

### Measurement versus calculation

The implementation should prefer storing facts that can be combined correctly:

```
MEASURE
├── counts
├── bounded categorical counts
├── sums/counts for averages
├── mergeable latency distributions
└── current gauges

CALCULATE
├── rates
├── percentages
├── availability
├── cache hit rate
├── throughput
└── percentile presentation values
```

This keeps the data model stable as dashboard calculations evolve and prevents multiple competing sources of truth.
