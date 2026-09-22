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

### Traces -> Metrics / Logs

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

Internal modules may emit signals, but only the observability subsystem owns:

- canonical metric names
- common log fields
- sampling policy
- resource sampling
- health aggregation
- alert definitions
- external observability export

Feature modules should not implement their own competing observability systems.
