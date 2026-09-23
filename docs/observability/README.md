# MyDNS Observability

## Purpose

This directory defines the production observability architecture for MyDNS. It is the design contract for implementation, operational behavior, dashboard consumption, and alerting.

The architecture is intentionally layered:

```text
Observability
├── Telemetry
│   ├── Metrics
│   ├── Logging
│   └── Tracing
├── Health
│   ├── Liveness
│   └── Readiness
├── Alerts
│   └── Alert rules derived from telemetry and health
└── Resources
    ├── CPU
    ├── Memory
    ├── Disk / log storage
    └── SQLite storage
```

Telemetry is the primary signal-producing layer. Profiling is a separate diagnostic capability for sampled runtime performance analysis. Resources are an operational observability category, not a telemetry signal type. They are included as a dedicated operational category because MyDNS needs to expose host/process/storage pressure and because those signals feed health and alerts.

## Current project baseline

The current refactor branch already contains a backend-owned `Metrics` collector and a metrics API. It records DNS totals, blocked queries, upstream success/failure, latency samples, cache evictions, query types, outcomes, and 24-hour minute-bucket history.

The current application also has:

- Rust `tracing` + `tracing-subscriber`
- file and stdout logging through `tracing-appender`
- Samply profiling integration
- Axum HTTP API and static frontend serving
- SQLite through SQLx, WAL mode, one connection
- DNS handling through Hickory
- shared `AppState` carrying metrics, DB, cache, indexes, upstream resolver, and cancellation
- `/api/v1/stats` and `/api/v1/stats/history`

The current implementation is therefore a foundation, not the final observability system.

## Design principles

1. **Instrumentation is separate from telemetry storage/export.**
2. **Metrics are low-cardinality and machine-oriented.**
3. **Logs are event-oriented and diagnostic.**
4. **Traces explain a request's execution path.**
5. **Health answers whether the service is alive and able to serve.**
6. **Alerts are derived from operational signals; they are not a fourth telemetry signal.**
7. **Resource telemetry is sampled periodically rather than measured on every request.**
8. **DNS hot paths must not perform expensive telemetry work.**
9. **No secrets, passwords, JWTs, authorization headers, or full DNS payloads are emitted by default.**
10. **The management dashboard consumes stable observability APIs rather than coupling directly to internal Rust structures.**
11. **Metrics, logging, and tracing have separate responsibilities even when they share infrastructure and correlation context.**
12. **The telemetry pipeline composes observability components; it does not absorb their domain responsibilities.**
13. **The same underlying execution context may be correlated across metrics, logs, traces, health, dashboard views, and alert rules without making those signals interchangeable.**
14. **Observability must remain bounded in memory, CPU, disk usage, and label cardinality.**

## Documents

- [Architecture](architecture.md) — signal flow and boundaries.
- [Telemetry](telemetry.md) — instrumentation strategy and signal ownership.
- [Metrics](metrics.md) — metric catalogue, names, labels, aggregation, and retention.
- [Logging](logging.md) — structured logs, levels, fields, destinations, and redaction.
- [Tracing](tracing.md) — spans, propagation, sampling, and DNS/API/DB traces.\n- [Profiling](profiling.md) — runtime performance profiling and Samply integration.
- [Health](health.md) — liveness/readiness semantics and status mapping.
- [Alerts](alerts.md) — alert conditions, severity, deduplication, and notification mapping.
- [Resources](resources.md) — CPU, memory, disk, log-file, and SQLite storage telemetry.
- [Implementation](implementation.md) — implementation order and migration plan.

## Scope

This plan covers:

- DNS request/response processing
- cache and in-memory indexes
- upstream DNS resolution
- HTTP/API requests
- authentication and authorization failures
- SQLite operations and database health
- frontend/static asset requests
- WebSocket connections
- process/runtime state
- CPU and memory
- log storage
- SQLite database/WAL storage
- liveness/readiness
- operational alerts

It does not prescribe a specific hosted observability vendor. Export targets should remain replaceable.
