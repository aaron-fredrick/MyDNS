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
- timestamped files under `logs/`
- a broadcast channel used by the application state for log streaming
- Samply as an additional profiling layer

These are runtime composition concerns: the telemetry bootstrap should assemble the logging layers with other observability layers. The logging component owns the file/stdout output and non-blocking writer lifecycle; Samply remains an optional profiling consumer.

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

Current per-startup timestamped files are useful for the dashboard and local troubleshooting.

Production planning should add:

- maximum file size
- retention count/age
- total log-directory size limit
- startup cleanup
- disk-pressure warning
- optional compression after rotation

The logger must fail safely if disk writes fail. DNS service availability must not depend on successful log persistence.

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
