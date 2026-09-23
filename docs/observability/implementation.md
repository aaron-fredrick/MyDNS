# Implementation Plan

## Status overview

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1 — Telemetry foundation | ✅ **Done** | Logging, tracing vocabulary, telemetry composition, and optional profiling integration; see §Phase 1 below |
| Phase 2 — HTTP instrumentation | 🔲 Not started | |
| Phase 3 — DNS instrumentation | 🔲 Not started | |
| Phase 4 — Database instrumentation | 🔲 Not started | |
| Phase 5 — Resource telemetry | 🔲 Not started | |
| Phase 6 — Health endpoints | 🔲 Not started | |
| Phase 7 — Alert integration | 🔲 Not started | |
| Phase 8 — External export | 🔲 Not started | |

> **Scope boundary.** Phase 1 establishes the telemetry composition foundation and tracing vocabulary — the
> subscriber composition, logging output configuration, log-writer lifetime management, and canonical
> span/field vocabulary. **Application instrumentation** (adding
> `#[tracing::instrument]` to DNS/HTTP/DB handlers, recording span attributes,
> using `span_names` and `fields` constants in production code) has **not**
> been started. That work begins in Phase 2 (HTTP) and Phase 3 (DNS).

---

## Phase 1 — Telemetry foundation ✅

### What was implemented

The observability telemetry boundary now contains the real subscriber/composition foundation rather than keeping subscriber policy in `main.rs`. Tracing vocabulary is defined separately from logging output concerns.

#### Current files

| File | Purpose |
|------|---------|
| `telemetry/pipeline.rs` | Single subscriber/composition entry point; assembles logging, tracing, and optional profiling |
| `telemetry/logging/config.rs` | Logging configuration: log directory, filename, and filter fallback |
| `telemetry/logging/layer.rs` | Non-blocking log writer infrastructure |
| `telemetry/logging/guard.rs` | Lifetime guard for the non-blocking log writer |
| `telemetry/logging/mod.rs` | Logging ownership boundary and public logging types |
| `telemetry/tracing/span_names.rs` | Canonical span-name vocabulary |
| `telemetry/tracing/fields.rs` | Canonical structured field-name vocabulary |
| `telemetry/profiling/mod.rs` | Optional Samply profiling-layer integration |
| `telemetry/mod.rs` | Declares the telemetry subsystems |

#### Modified files

| File | Change |
|------|--------|
| `src/main.rs` | Constructs `LoggingConfig` explicitly and initializes the top-level telemetry pipeline; retains the returned logging guard for process lifetime |

### Design decisions

1. **Single telemetry bootstrap entry point.** Subscriber composition is owned by the observability boundary. Logging owns log-output layers and writer lifetime; tracing owns span vocabulary and instrumentation; profiling owns the optional Samply integration. The telemetry bootstrap composes those components, and `main.rs` no longer needs to know about subscriber crate internals.

2. **The non-blocking file writer has logging ownership.** Its `WorkerGuard` must stay alive for the process lifetime. This is a logging lifecycle concern, not a trace lifecycle concern. `LoggingGuard` now owns that lifetime directly.

3. **No new crate dependencies.** All required crates (`tracing-subscriber`,
   `tracing-appender`, `tracing-samply`) are already in `Cargo.toml`.

4. **Samply remains optional.** `SamplyLayer::new()` failing is non-fatal and
   is the expected normal case — Samply is only present when the operator
   explicitly attaches the `samply record` profiler. The absent case is
   **silent**: no stderr output is emitted. Only the rarer attached case
   prints a brief diagnostic to stderr (using `eprintln!`, because the
   tracing subscriber is not yet installed at that point).

5. **Text format preserved.** File output: no ANSI. Stdout: ANSI enabled. No
   JSON format has been added. JSON is a potential future task.

6. **Span/field constants defined.** `span_names` and `fields` modules define
   the complete documented span/field vocabulary. Application code has not been
   changed to use them yet — that is Phase 2+.

### Limitations / review points for Phase 2

- `span_names` and `fields` constants are defined but **not yet referenced**
  by any application code. Adoption happens in Phase 2 (HTTP) and Phase 3
  (DNS).
- No JSON/structured log format is implemented. The existing text format is
  sufficient for local observability but should be reconsidered before external
  export (Phase 8).
- `EnvFilter` is a single top-level filter. Per-crate / per-module filtering
  (e.g. reducing noise from hickory internals at `info` while enabling
  `debug` for `mydns`) is straightforward to add in `TracingConfig` before
  Phase 2 lands.
- There is currently no mechanism to change the filter level at runtime. A
  reload handle (`EnvFilter::with_reloader`) could be added to `TracingGuard`
  if runtime log-level adjustment becomes a requirement.
- The live log broadcast channel (`log_tx`) still routes raw strings through
  `AppState`. A future task could integrate a custom logging layer that writes
  structured events directly to the channel. This remains a logging concern even
  though the events originate from the Rust `tracing` API.
- `pipeline::init()` is not unit-tested in isolation because it sets the
  process-global tracing subscriber, which cannot be re-set. The structural
  properties of `TracingConfig`, `span_names`, and `fields` are tested. The
  subscriber composition is validated by the full integration test suite which
  exercises the process boundary.

---

## Component ownership during implementation

Implementation work must preserve the same boundaries as the architecture:

- Logging changes belong to the logging component: output destinations, formatting, filtering, retention, redaction, live-log delivery, and writer lifetime.
- Tracing changes belong to the tracing component: span names, instrumentation, context, hierarchy, sampling, and future trace storage/export.
- Metrics changes belong to the metrics component: measurements, aggregation, labels, and metric exposure.
- Subscriber/global-pipeline assembly belongs to the telemetry composition layer.

A feature module such as DNS, HTTP, or DB may emit metrics, logs, and spans as part of one operation, but it must not create parallel output pipelines or take ownership of another signal's storage or lifecycle.

---

## Phase 2 — HTTP instrumentation

Add a Tower/Axum middleware layer for:

- request count
- route template
- method
- status class
- duration
- active requests

Add trace spans at the HTTP boundary using `span_names::HTTP_REQUEST` and
the field constants from `fields`.

Instrument authentication failures/rate limiting separately.

Add WebSocket connection gauges and lifecycle events.

---

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

---

## Phase 4 — Database instrumentation

Instrument semantic DB operations.

Capture:

- operation
- duration
- success/failure
- error class
- busy/locked condition
- migration state

Avoid raw SQL in normal telemetry.

---

## Phase 5 — Resource telemetry

Implement one sampler for:

- CPU
- resident memory
- filesystem free space
- log directory
- active log file
- SQLite database
- WAL

Publish values into the shared observability state.

---

## Phase 6 — Health endpoints

Implement:

- `/health/live`
- `/health/ready`

Make readiness dependency-aware and liveness shallow.

Ensure graceful shutdown immediately changes readiness to not-ready.

---

## Phase 7 — Alert integration

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

---

## Phase 8 — External export

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

---

## Existing code that should be evolved

### `src/main.rs`

Currently calls the tracing-module initialization entry point. The architectural target is for `main.rs` to call the top-level observability/telemetry bootstrap instead. No subscriber, logging, tracing, or profiling policy should be added here.

### `src/mydns/observability/`

Owns the canonical observability boundary. The implementation is being evolved so telemetry composition and individual signal responsibilities remain separate even when they share subscriber infrastructure.

### `src/mydns/state/`

Continue sharing the observability state through `AppState`.

### `src/mydns/dns/metrics_handler.rs`

Keep this as a DNS request-level metrics decorator. Add tracing spans (Phase 3)
and delegate detailed instrumentation to the relevant resolution stages.

### `src/mydns/web/routes.rs`

Add health routes and the HTTP telemetry middleware here or at the
server/router boundary (Phase 2, Phase 6).

### `src/mydns/api/v1/stats.rs`

Keep this as the dashboard aggregate API. Do not overload it with
exporter-specific formats.

### `src/mydns/db/`

Add DB operation instrumentation around semantic operations (Phase 4).

---

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

---

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

