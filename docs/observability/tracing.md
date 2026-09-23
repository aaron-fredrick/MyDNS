# Tracing

## Goal

Tracing explains **how a request moved through MyDNS** and where time was spent.

## Responsibility

Tracing owns the execution model represented by spans and traces:

- instrumentation and meaningful operation boundaries
- canonical span names
- span hierarchy and parent/child relationships
- trace and span context
- bounded span attributes
- error/outcome semantics
- sampling policy
- future trace storage/query
- trace export, including a possible future OTLP/OpenTelemetry path

Logging and tracing are intentionally correlated but remain separate responsibilities. Logging owns log records and their destinations; tracing owns spans and trace storage. A log event may inherit trace context, and a trace may be correlated with logs, without either subsystem taking ownership of the other.

The telemetry bootstrap/composition layer owns installation of the single global subscriber/registry. `tracing-subscriber` is shared infrastructure and is not itself the definition of the tracing subsystem.

The project already uses `tracing` and `tracing-subscriber`. Samply is an optional profiling consumer attached by the telemetry composition layer. The next step is consistent span instrumentation rather than adding ad-hoc timers everywhere.

## Trace hierarchy

### Process

```text
mydns
├── dns.server
├── http.server
└── background
```

### DNS request

```text
dns.request
├── parse
├── zone_lookup
├── blocklist_lookup
├── cache_lookup
├── local_record_lookup
├── upstream.resolve
│   ├── upstream.request
│   └── retry
└── response
```

Only spans that represent meaningful work should be created. Do not create a span for every trivial helper.

### HTTP request

```text
http.request
├── authentication
├── handler
│   └── database operation
└── response
```

### Database

Use semantic child spans:

- `db.connect`
- `db.migrate`
- `db.records.list`
- `db.records.create`
- `db.records.update`
- `db.records.delete`
- `db.zones.load`
- `db.settings.get`

Do not record raw SQL text as a normal span attribute.

## Span attributes

Safe bounded attributes:

- component
- operation
- protocol
- DNS record type
- response code
- outcome
- upstream identifier
- HTTP method
- normalized route
- status code

Avoid:

- full Authorization values
- passwords
- JWTs
- raw SQL parameters
- unbounded domain/client identifiers as routine attributes

## Error semantics

A span should record an error when an operation fails.

The span should also carry a bounded outcome such as:

- success
- timeout
- refused
- not_found
- validation_error
- internal_error

Do not rely solely on log messages to understand failures.

## Sampling

Recommended initial policy:

- lifecycle/background spans: retain
- errors: retain
- slow requests: retain
- normal high-volume DNS requests: sampled
- normal API requests: sampled at a higher rate than DNS if volume permits

Sampling must not remove metrics.

## Trace/log correlation

When a tracing span exists, structured logs emitted inside that span should inherit the trace context.

This allows:

```text
metric anomaly
   -> timestamp/window
   -> trace
   -> structured log
   -> exact failing operation
```

## Future trace storage and visualisation

Trace storage is a **planned later phase**, not part of the current tracing implementation.

The goal is for MyDNS to remain self-contained while eventually allowing the dashboard to inspect a request as a visual span tree and correlate it with logs. Before the observability pipeline is configured for dashboard visualisation, introduce a dedicated trace storage/query layer.

The initial planned storage approach is **embedded SQLite**, using a trace-specific schema rather than treating the existing application database as a generic telemetry dump.

Conceptually:

```text
Rust tracing
    ↓
MyDNS trace model
    ↓
TraceStore
    ↓
SQLite
    ↓
MyDNS dashboard trace API
    ↓
visual trace tree
```

The trace store should preserve enough information to reconstruct parent/child relationships, including:

- trace ID
- span ID
- parent span ID
- span/operation name
- start time
- duration
- status
- bounded attributes

The storage design should include bounded retention, indexes for recent/slow/error traces, and configurable storage limits. Normal high-volume DNS traffic should be sampled; errors and slow operations should remain highly visible.

This storage layer should **not be implemented now**. It is a later step that should be designed and implemented alongside the observability pipeline work required for dashboard visualisation.

The storage abstraction should also keep the option open for a future OTLP/OpenTelemetry export path without coupling the dashboard to SQLite.

## Performance

Tracing must be low overhead:

- avoid formatting expensive fields unless emitted
- avoid recording entire DNS messages
- avoid high-cardinality attributes
- sample high-volume successful DNS requests
- keep error/slow-request visibility high
