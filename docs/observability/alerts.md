# Alerts

## Principle

Alerts convert observable conditions into actionable notifications.

They should be based primarily on metrics and health checks, with logs used for discrete events that do not have an appropriate metric.

Avoid alerting on every error log.

## Severity model

### Critical

The service cannot perform a required production role or is at immediate risk of failing.

Examples:

- liveness probe failing
- readiness continuously failing for a production instance
- filesystem critically full
- SQLite database unavailable for authoritative management operations

### Warning

The service is degraded or approaching a resource/latency boundary.

Examples:

- sustained upstream failure
- elevated DNS SERVFAIL
- elevated HTTP 5xx
- high API latency
- high memory
- log storage approaching limit
- SQLite WAL growing unusually large

### Informational

Operational events worth recording but not paging.

Examples:

- configuration reload
- service restart
- graceful shutdown
- upstream recovered

## Initial alert catalogue

| Alert | Signal | Initial condition | Severity |
|---|---|---|---|
| MyDNSDown | liveness | probe fails repeatedly | critical |
| MyDNSNotReady | readiness | readiness fails for sustained interval | critical |
| DNSHighServfail | DNS response counters | SERVFAIL ratio sustained above threshold | warning |
| DNSHighLatency | DNS latency histogram | p95/p99 sustained above threshold | warning |
| UpstreamUnavailable | upstream metrics | failure ratio sustained above threshold | warning |
| UpstreamTimeouts | upstream timeout counter | sustained timeout rate | warning |
| APIHigh5xx | HTTP metrics | 5xx ratio sustained above threshold | warning |
| APIHighLatency | HTTP latency | p95 sustained above threshold | warning |
| DatabaseErrors | DB error counter | sustained DB error rate | critical/warning by operation |
| DatabaseBusy | SQLite busy metric | sustained busy/locked events | warning |
| DiskSpaceLow | filesystem metric | free space below configured percentage/bytes | warning |
| DiskSpaceCritical | filesystem metric | critically low free space | critical |
| LogStorageHigh | log directory metric | configured log storage threshold exceeded | warning |
| SQLiteStorageHigh | DB file metric | configured DB storage threshold exceeded | warning |
| MemoryHigh | process memory | sustained high memory | warning |
| MemoryCritical | process memory | sustained critical memory pressure | critical |
| AuthRateLimitSpike | auth metric | abnormal sustained rejected-login rate | warning |

Exact thresholds must be configuration, not hard-coded assumptions. Thresholds should be validated during load testing.

## Alert design

Use:

- sustained windows instead of single samples
- hysteresis for recovery
- deduplication
- grouping by service/instance
- explicit recovery notifications where supported
- maintenance/silence support outside the Rust process

## Alert dependencies

Do not page on downstream symptoms when a root-cause alert already exists.

For example:

```text
filesystem critically full
      |
      +--> logging errors
      +--> SQLite write failures
      +--> readiness failure
```

The filesystem alert should be treated as the primary infrastructure signal.

## Alert transport

MyDNS should emit alert-ready signals, not hard-code a single notification provider into core DNS code.

Possible external consumers:

- Prometheus/Alertmanager
- Grafana alerting
- hosted monitoring services
- webhook-based operational systems

A future in-process alert manager may be added only if local/offline alerting is a product requirement.

## Recovery

Every alert should define:

- firing condition
- recovery condition
- evaluation window
- severity
- operational owner
- dashboard link/context where available
