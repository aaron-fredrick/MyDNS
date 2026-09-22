# Health

## Purpose

Health is the operational contract used by process supervisors, load balancers, container orchestration, and external monitors.

Health is distinct from telemetry:

- telemetry describes what MyDNS is doing
- health states whether MyDNS can perform its required service role

## Endpoints

Implement:

- `GET /health/live`
- `GET /health/ready`

These should be outside `/api/v1` and should not require dashboard authentication.

## Liveness

Liveness answers:

> Is the MyDNS process running and capable of executing its event loop?

Liveness should be intentionally shallow.

Recommended result:

- **200 OK** — process is alive
- **503 Service Unavailable** — process is not able to service the probe

Do not make liveness depend on SQLite, upstream DNS, or disk health. A temporary dependency failure should not cause a process restart loop.

## Readiness

Readiness answers:

> Should this instance receive normal DNS/HTTP workload?

Readiness checks:

1. process is running
2. shutdown has not started
3. DNS subsystem is bound/started
4. HTTP subsystem is bound/started
5. SQLite is initialized and usable
6. required in-memory indexes are loaded
7. resolver subsystem is initialized

Upstream DNS availability is **not** a basic readiness requirement in forwarding mode. MyDNS can be alive and locally authoritative even when an upstream is unavailable.

Resource pressure can degrade readiness when continuing to serve would be unsafe.

## Status model

Use a small internal state model:

```text
UNKNOWN
  |
STARTING
  |
READY
  |
DEGRADED
  |
DRAINING
  |
STOPPED
```

For HTTP health responses, keep the externally visible mapping simple:

| Internal state | Liveness | Readiness |
|---|---:|---:|
| STARTING | 200 | 503 |
| READY | 200 | 200 |
| DEGRADED | 200 | 200 or 503 depending on dependency |
| DRAINING | 200 | 503 |
| STOPPED | unreachable | unreachable/503 |

The readiness response should include a machine-readable status and individual check results.

## Dependency check states

Each readiness check should return:

- `healthy`
- `degraded`
- `unhealthy`
- `not_applicable`

The aggregator maps these to overall readiness.

Examples:

- SQLite query succeeds -> healthy
- SQLite temporarily busy -> degraded/unhealthy depending on persistence role
- upstream unavailable -> degraded, not automatically unhealthy
- DNS listener failed -> unhealthy
- shutdown requested -> unhealthy for readiness

## HTTP status mapping

Use:

- **200** when the service is ready
- **503** when it is not ready

Do not return 500 for an expected readiness failure.

## Health response

A readiness response should resemble:

```json
{
  "status": "ready",
  "checks": {
    "process": "healthy",
    "dns": "healthy",
    "http": "healthy",
    "database": "healthy",
    "indexes": "healthy",
    "upstream": "degraded",
    "resources": "healthy"
  }
}
```

The exact schema is implementation-defined but must be stable.

## Dashboard usage

The dashboard can consume the same health model for:

- service status badge
- dependency status
- degraded-state explanation
- startup/shutdown state

It should not infer health from a random combination of dashboard metrics.

## Alert relationship

Health endpoints are ideal for external probe alerts.

Telemetry-derived alerts provide richer conditions such as:

- rising latency
- high error rate
- storage pressure
- upstream failures

Health and telemetry therefore complement one another.
