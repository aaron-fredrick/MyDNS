# Logging

## Responsibility

Logging owns the lifecycle of log records from emission to configured destinations. It owns log levels, filtering, formatting, stdout/file output, retention/rotation policy, redaction, live log presentation, and the lifetime of non-blocking log writers.

Logging may consume tracing context when a log event occurs inside a span. That context is correlation data; it does not make tracing responsible for log output or make logging responsible for trace storage.

## Current implementation

MyDNS currently initializes `tracing-subscriber` with:

- environment filtering
- a non-blocking file writer
- stdout output
- ANSI disabled in files
- ANSI enabled on stdout
- structured JSONL files under `logs/`, partitioned by the configured local calendar day
- a broadcast channel used by the application state for live log streaming

These are runtime composition concerns. The telemetry bootstrap assembles the logging layers with the other telemetry layers. The logging component owns the file/stdout output and non-blocking writer lifecycle.

## Log levels

### ERROR

Use when an operation failed and requires investigation or has affected service behavior.

Examples:

- database initialization failure
- HTTP/DNS listener failure
- migration failure
- unrecoverable upstream subsystem failure
- internal invariant failure

### WARN

Use for recoverable degradation or unusual conditions.

Examples:

- upstream timeout
- retry
- readiness degradation
- high resource pressure
- rejected request due to rate limiting
- log/storage nearing configured limits

### INFO

Use for important lifecycle and configuration events.

Examples:

- service start
- listener started
- configuration loaded
- database initialized
- zones/index loaded
- graceful shutdown
- configuration change

### DEBUG

Use for diagnostic execution detail that is not appropriate for normal production volume.

Examples:

- cache miss reason
- resolver branch selection
- detailed API/database operation context

### TRACE

Use for highly detailed development diagnostics. It should not be the normal production level.

## Structured fields

Log fields describe log records. Where a log event is emitted inside a tracing span, tracing context such as `trace_id` and `request_id` may be included for correlation. Canonical tracing span names and trace context remain owned by the tracing component.

Common fields:

- timestamp
- level
- target
- message
- service = `mydns`
- component
- operation
- outcome
- duration_ms
- trace_id when available
- request_id when available

DNS-specific:

- transport
- record_type
- response_code
- resolution_path
- upstream identifier

HTTP-specific:

- method
- normalized route
- status
- duration_ms

Database-specific:

- operation
- duration_ms
- outcome
- error_class

## Security/redaction

Never log:

- passwords
- admin password
- JWT secret
- JWT bearer token
- Authorization header
- session cookies
- raw credential material

Avoid logging complete DNS payloads by default.

Client IPs and queried domains should be considered operationally sensitive and only logged when required by the configured diagnostic level/policy.

## File logging

Structured JSON Lines (JSONL) files are the canonical durable log store for MyDNS. SQLite should not be used as a second durable log store; keeping both would duplicate the same log data and introduce another write path and storage lifecycle for logs.

Logs should be organized by the configured application-local calendar day:

```text
logs/
├── mydns-2026-09-22.jsonl
├── mydns-2026-09-23.jsonl
└── mydns-2026-09-24.jsonl
```

The date used in the filename is determined by converting the event timestamp to the configured `system.timezone`. The event timestamp stored in each JSONL record remains UTC. This keeps storage, ordering, correlation, retention, and cross-system processing deterministic while making daily files align with the operator's configured calendar.

Each line must represent one complete structured log event. JSONL is the persistence format; human-readable formatting belongs to presentation layers such as the dashboard or command-line tooling.

Production planning should add:

- maximum file size as an additional safety bound within a local-day file
- retention count/age
- total log-directory size limit
- startup cleanup
- disk-pressure warning
- optional compression after a local-day file is closed and rotated

The logger must fail safely if disk writes fail. DNS service availability must not depend on successful log persistence. Log persistence must remain non-blocking with respect to DNS/API request processing.

## Log parser and historical access

The log parser is the read path for persisted logs. It reads JSONL files and provides structured records to the backend Logs API; the dashboard should not read or parse files directly.

Conceptually:

```text
JSONL log files
      ↓
log parser
      ↓
Logs API
      ↓
dashboard
```

The parser should support, as the API requires:

- local or UTC time-range filtering
- log-level filtering
- target/component filtering
- event filtering
- structured-field filtering
- text search over approved fields
- pagination
- reading the relevant daily file(s) for a requested range

The parser must compare actual UTC timestamps in records when applying precise time ranges. File names are an index to the relevant local calendar day, not the authoritative event timestamp.

The parser is read-only with respect to log persistence. It must not rewrite, normalize, or migrate log files as part of ordinary dashboard queries.

## Live log stream

The existing broadcast channel is appropriate for a bounded live dashboard stream.

It must:

- drop or summarize events when a client is too slow
- never block DNS/API work
- expose structured fields where practical
- not become the authoritative log store

The dashboard should consume a sanitized presentation representation rather than arbitrary internal events.

## Logging versus tracing

Logging records events. Tracing records execution structure. A single request may therefore produce both a trace and log events carrying the same correlation context. Neither signal replaces the other, and the logging subsystem must not become the trace store.

## Logging versus metrics

Do not log every DNS request at INFO.

Use metrics for volume and rates. Use traces for execution paths. Use logs for exceptional or state-transition events.


## Timezone handling

Log timestamps should remain machine-correlatable and deterministic. The logging system should use UTC as the canonical stored timestamp.

Where a human-facing log view or dashboard presentation benefits from local time, the configured application timezone may be used for presentation. Logging must not independently infer its timezone from the Windows/Linux host configuration.

The global application timezone is therefore a presentation/calendar setting, not a replacement for UTC event timestamps.


## Log persistence and delivery model

Logging has one durable storage destination and one live delivery destination:

```text
application events
       ↓
tracing / logging pipeline
       ├── JSONL files ──→ durable historical logs
       │                       ↓
       │                  log parser
       │                       ↓
       │                   Logs API
       │                       ↓
       │                   dashboard
       │
       └── WebSocket ─────→ live dashboard stream
```

The WebSocket stream is a live subscription and is not a persistence mechanism. A disconnected or slow dashboard client must not prevent file persistence or application work.

The JSONL files are therefore the canonical source for historical logs. The parser provides the query abstraction over those files, while the WebSocket provides low-latency delivery of newly emitted events.

## Log record format

The persisted format should be JSONL, with one JSON object per line. A representative record is:

```json
{"timestamp":"2026-09-24T18:31:42.123Z","level":"WARN","target":"mydns::dns::upstream","event":"upstream_timeout","upstream":"1.1.1.1","duration_ms":2000}
```

The schema should remain structured and extensible. Fields that are not applicable to an event should be omitted rather than represented by arbitrary placeholder values.

The canonical timestamp is UTC. The configured `system.timezone` determines calendar-day file partitioning and may be used for human-facing display, but it does not replace UTC timestamps in persisted records.

## Storage and backpressure

The logging path must be isolated from request-critical work:

- DNS and HTTP request handling must not wait for the dashboard WebSocket.
- Slow WebSocket clients may lose live events according to the bounded-stream policy; historical events remain available from JSONL files.
- File writing should use the existing non-blocking logging writer.
- File-write failures should be surfaced through the logging/diagnostic path without making DNS availability dependent on the logger.
- Log rotation and cleanup should execute outside the DNS request hot path.

This keeps logging operationally useful without making it a dependency of DNS service availability.
