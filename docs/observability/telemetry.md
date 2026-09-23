# Telemetry Plan

## Definition

Telemetry is the operational information produced by MyDNS instrumentation.

The three primary signals are:

- **Metrics** — numerical measurements and counters.
- **Logs** — structured events.
- **Traces** — timed execution paths represented as spans.

Instrumentation is the code-level mechanism that creates these signals.

## Responsibility boundaries

Telemetry is the umbrella for the signals, not a single implementation that owns all of their storage or output.

- **Metrics** owns numerical measurements, aggregation, bounded labels, and metric exposure/export.
- **Logging** owns structured log events, levels, filtering, formatting, destinations, retention, and the lifecycle of log writers.
- **Tracing** owns spans, trace context, parent/child relationships, span attributes, sampling, and trace storage/export.
- **Telemetry composition** owns initialization and integration of these components into the runtime observability pipeline.

These components may use shared Rust infrastructure such as `tracing` and `tracing-subscriber`. Shared infrastructure is an integration mechanism, not a transfer of responsibility. A log event can inherit tracing context, and metrics can describe the same request window, without making logging responsible for traces or tracing responsible for log persistence.

The single global subscriber/registry is composed once by the telemetry bootstrap layer. Individual components contribute their own layers or consumers; they do not install competing global subscriber systems.

## Instrumentation rules

Use Rust `tracing` for request/span instrumentation and structured events. The `tracing` API provides the common instrumentation mechanism; logging and tracing remain separate semantic responsibilities.

Prefer:

```rust
#[tracing::instrument(skip(state), fields(...))]
async fn operation(...) { ... }
```

over manually timing and logging the same operation in multiple places.

Metrics should be recorded at stable semantic boundaries.

Tracing should capture execution structure.

Logging should capture state transitions, failures, security events, and operational events.

## Instrumentation domains

### DNS

Instrument:

- request received
- query parse/validation
- query type/class
- zone ownership lookup
- blocklist decision
- cache lookup
- local record lookup
- upstream selection
- upstream request
- upstream retry/timeout
- response construction
- response code
- request completion

Primary metrics:

- total queries
- queries by record type
- response code counts
- latency
- blocked queries
- cache hits/misses
- upstream requests/success/failure
- upstream latency
- timeout/retry counts

### HTTP/API

Instrument:

- method
- normalized route template, not raw URL
- response status
- request duration
- request/response size where useful
- authentication outcome
- rate-limit outcome
- handler errors

Do not use query strings, authorization headers, JWTs, cookies, or arbitrary request bodies as metric labels.

### Database

Instrument:

- connection/open
- migration execution
- query/operation category
- transaction begin/commit/rollback
- query duration
- SQLite busy/locked conditions
- errors
- pool wait if it becomes relevant after pool scaling

Prefer semantic operation names such as `records.list`, `records.create`, `zones.load`, `settings.get` rather than raw SQL text.

### Frontend/static web requests

The Rust web server should measure:

- static asset request count
- route/status
- response latency
- 4xx/5xx responses
- WebSocket connects/disconnects

Browser-side frontend telemetry can be added later if needed. It should remain separate from backend service telemetry.

### Runtime/resources

Sample:

- process CPU usage
- process memory usage
- resident set size where available
- open file descriptors/handles where available
- log directory size
- current log file size
- SQLite database size
- SQLite WAL size
- free disk space
- cache/index sizes

Resource sampling should run on a low-frequency background task, not on request paths.

## Cardinality policy

Safe metric labels:

- protocol: `udp`, `tcp`, `http`
- DNS record type: bounded enum
- DNS response code: bounded enum
- HTTP method: bounded enum
- HTTP route: normalized route template
- HTTP status class: bounded enum
- upstream target: configured endpoint identifier, not arbitrary user data
- outcome: bounded enum

Avoid labels containing:

- full domain names
- client IP addresses
- user IDs
- record IDs
- database primary keys
- JWTs
- trace IDs
- arbitrary error strings

Full DNS names belong in controlled debug logs/traces only when operationally necessary and subject to privacy policy.

## Export strategy

The internal signal API should remain independent from exporters.

Recommended conceptual pipeline:

```text
MyDNS instrumentation
        |
        +--> in-process metrics
        +--> tracing spans/events
        +--> structured logs
        |
        +--> local observability API
        |
        +--> optional external exporters
```

OpenTelemetry may be introduced for external traces/metrics where it provides value, but MyDNS should not make its core DNS logic depend on a specific backend.

## Performance requirements

Telemetry must be:

- asynchronous where possible
- bounded
- non-blocking on DNS hot paths
- cheap when disabled
- safe during shutdown
- resilient to exporter failure

Telemetry failure must never take down DNS service.
