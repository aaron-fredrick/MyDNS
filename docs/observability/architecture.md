# Observability Architecture

## Signal model

MyDNS uses one shared operational model with several consumers:

```text
                    MyDNS runtime
                         |
                  Instrumentation
                         |
          +--------------+--------------+
          |              |              |
       Metrics          Logs          Traces
          |              |              |
          +--------------+--------------+
                         |
                Observability API
                         |
             +-----------+-----------+
             |                       |
          Dashboard              Alert rules
             |                       |
             +-----------+-----------+
                         |
                       Alerts

Health is evaluated from direct service checks plus selected
telemetry/resource conditions.

Resource samplers feed metrics/logs and may contribute to health.
```

## Responsibility boundaries

The observability components are intentionally separate responsibilities. They may share infrastructure and correlation context, but one component must not implement another component's lifecycle or storage concerns.

| Component | Owns | May integrate with |
|---|---|---|
| **Metrics** | Numerical measurements, aggregation, bounded labels, and metric exposure/export | request/trace context where useful for correlation, without using trace IDs as metric labels |
| **Logging** | Log events, levels, filtering, formatting, stdout/file output, retention, and non-blocking log-writer lifecycle | tracing context, request context, and operational state |
| **Tracing** | Instrumentation model, spans, parent/child relationships, trace context, span attributes, sampling, and future trace storage/export | logging context and request lifecycle |
| **Telemetry pipeline** | Composition and initialization of the observability components and their shared subscriber infrastructure | all telemetry components |
| **Profiling** | Optional runtime performance profiling integration such as Samply and performance-profile lifecycle | tracing/subscriber infrastructure where the profiler consumes it |

`tracing-subscriber` is shared Rust infrastructure for consuming `tracing` events and spans. Its use by logging, tracing, or profiling does not transfer ownership of those responsibilities to another component.

For example, a request may have one trace context while producing a trace span, structured log events, and metric observations. Correlation is intentional; the resulting signals remain independently owned.

## Telemetry composition

The runtime topology is:

```text
                         MyDNS application
                                |
                       instrumentation/events
                                |
                    +-----------v-----------+
                    |  Telemetry composition|
                    |       / bootstrap      |
                    +-----------+-----------+
                                |
              +-----------------+-----------------+
              |                 |                 |
              v                 v                 v
           Metrics           Logging           Tracing
              |                 |                 |
          metric API       stdout/file       TraceStore
          /export             output          /export

                         Optional profiling
                              (Samply)
```

The composition layer installs the single global subscriber/registry and combines the layers supplied by the individual observability components. Logging owns log-output concerns; tracing owns span and trace concerns; profiling owns the optional Samply integration. They are not separate competing global subscriber systems.

## Cross-layer relationships


### Metrics -> Health

Examples:

- sustained DNS failure rate can make readiness degraded
- database connection failure can make readiness fail
- excessive resource pressure can make readiness degraded
- exporter/telemetry failure must not automatically make the DNS server unhealthy

### Metrics -> Alerts

Examples:

- high DNS SERVFAIL ratio
- upstream availability below threshold
- HTTP 5xx rate
- API latency above threshold
- SQLite failures
- disk/log storage pressure
- memory pressure

### Logs -> Alerts

Logs are useful for event-based alerts such as:

- database migration failure
- listener bind failure
- repeated startup/shutdown failure
- authentication abuse
- unrecoverable internal errors

Logs should not be the primary source for high-frequency numerical alerts when a metric can represent the same condition.

### Profiling -> Performance analysis\n\nProfiling is consumed as a diagnostic performance-analysis capability. It can be used alongside metrics and traces to explain CPU hotspots, but it is not a primary alerting signal and does not own request or trace semantics.\n\n### Traces -> Metrics / Logs

A trace/span should carry enough context to correlate a diagnostic event with a request. Trace identifiers should be included in structured logs where available.

Metrics must not contain trace IDs as labels because that creates extreme cardinality.

### Health -> Alerts

Health endpoints provide simple operational probes. External monitoring can alert on repeated readiness failure or liveness failure.

## Request correlation

For HTTP:

```text
HTTP request
  -> request_id / trace_id
  -> route span
  -> authentication span/event
  -> database spans
  -> response
```

For DNS:

```text
DNS packet
  -> DNS request span
  -> cache/index lookup
  -> local authoritative path OR upstream path
  -> response construction
  -> DNS response
```

For upstream:

```text
DNS request span
   -> upstream span
      -> selected upstream
      -> timeout/retry
      -> response
```

## Boundary rule

Internal modules may emit signals, but only the appropriate observability component owns its canonical semantics:

- **Metrics** owns canonical metric names, labels, aggregation, and metric exposure/export.
- **Logging** owns log fields used for log records, levels, formatting, filtering, destinations, retention, redaction policy, and log-writer lifecycle.
- **Tracing** owns span names, trace fields/context, hierarchy, sampling, trace retention/storage, and trace export.
- **Telemetry composition** owns assembly of those components into the runtime pipeline.
- **Resource telemetry** owns resource sampling and resource measurements, while health and alerts consume the resulting signals.
- **Health** owns health aggregation and endpoint semantics.
- **Alerts** owns alert definitions and evaluation policy.

Feature modules should emit observations through these canonical boundaries rather than implementing competing telemetry systems.
