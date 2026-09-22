# Implementation Plan

## Phase 1 — establish observability contracts

1. Keep `src/mydns/observability` as the backend observability boundary.
2. Add a health module with explicit liveness/readiness state.
3. Add a background resource sampler.
4. Define canonical metric names and bounded labels.
5. Define structured logging fields and redaction rules.
6. Define tracing span names and hierarchy.

## Phase 2 — HTTP instrumentation

Add a Tower/Axum middleware layer for:

- request count
- route template
- method
- status class
- duration
- active requests

Add trace spans at the HTTP boundary.

Instrument authentication failures/rate limiting separately.

Add WebSocket connection gauges and lifecycle events.

## Phase 3 — DNS instrumentation

Retain the existing `MetricsHandler` as the high-level DNS measurement boundary.

Add tracing spans around:

- request parsing
- zone lookup
- blocklist
- cache
- local record index
- upstream resolution
- response construction

Add missing counters for:

- cache hits/misses
- upstream timeout/retry
- local lookup outcomes
- active requests

Do not put expensive logging in the DNS hot path.

## Phase 4 — database instrumentation

Instrument semantic DB operations.

Capture:

- operation
- duration
- success/failure
- error class
- busy/locked condition
- migration state

Avoid raw SQL in normal telemetry.

## Phase 5 — resource telemetry

Implement one sampler for:

- CPU
- resident memory
- filesystem free space
- log directory
- active log file
- SQLite database
- WAL

Publish values into the shared observability state.

## Phase 6 — health endpoints

Implement:

- `/health/live`
- `/health/ready`

Make readiness dependency-aware and liveness shallow.

Ensure graceful shutdown immediately changes readiness to not-ready.

## Phase 7 — alert integration

Expose stable metrics suitable for an external alert engine.

Start with:

- liveness/readiness
- DNS SERVFAIL
- DNS latency
- upstream failures/timeouts
- HTTP 5xx
- API latency
- DB errors/busy
- disk
- logs
- SQLite/WAL
- memory

## Phase 8 — external export

Only after the internal model is stable, add external exporters.

Preferred architecture:

```text
internal MyDNS observability model
        |
        +--> dashboard API
        +--> structured logs
        +--> metrics exporter
        +--> tracing exporter
```

Do not make dashboard functionality depend on external observability infrastructure.

## Existing code that should be evolved

### `src/main.rs`

Currently owns logging initialization and Samply setup. Keep initialization here, but move policy/configuration into the observability subsystem.

### `src/mydns/observability/`

Currently contains `metrics.rs` and `types.rs`. Expand this as the canonical observability boundary rather than adding metrics independently to DNS/API/DB modules.

### `src/mydns/state/`

Continue sharing the observability state through `AppState`.

### `src/mydns/dns/metrics_handler.rs`

Keep this as a DNS request-level metrics decorator. Add tracing spans and delegate detailed instrumentation to the relevant resolution stages.

### `src/mydns/web/routes.rs`

Add health routes and the HTTP telemetry middleware here or at the server/router boundary.

### `src/mydns/api/v1/stats.rs`

Keep this as the dashboard aggregate API. Do not overload it with exporter-specific formats.

### `src/mydns/db/`

Add DB operation instrumentation around semantic operations.

## Testing requirements

Add unit/component/integration coverage for:

- metric counter correctness
- histogram recording
- cardinality constraints
- log redaction
- trace field presence
- health status mapping
- readiness dependency failures
- graceful shutdown readiness
- CPU/memory sampling
- log-size calculation
- SQLite/WAL size calculation
- disk threshold transitions
- alert condition calculations

Add end-to-end checks for:

- `/health/live`
- `/health/ready`
- API telemetry
- DNS telemetry
- DB failure -> readiness behavior
- upstream failure -> degraded behavior
- critical disk/resource simulation where feasible

## Definition of done

Observability is complete when an operator can answer, from the system without attaching a debugger:

1. Is MyDNS alive?
2. Is it ready to serve?
3. How much DNS traffic is it handling?
4. What are DNS response/error rates?
5. Where is DNS latency coming from?
6. Is cache performance healthy?
7. Are upstream resolvers healthy?
8. Are API requests healthy?
9. Are authentication failures abnormal?
10. Is SQLite healthy?
11. How much CPU and memory is MyDNS using?
12. How much disk is available?
13. How large are the logs?
14. How large are SQLite/WAL files?
15. What exact request/span/event explains a detected anomaly?
16. What alert should fire when a defined operating boundary is crossed?
