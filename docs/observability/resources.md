# Resource Telemetry

## Purpose

MyDNS needs visibility into resources consumed by the process and the storage used by operational artifacts.

Resource telemetry owns the measurement and sampling of CPU, memory, filesystem, log-storage, and SQLite-storage conditions. It publishes those measurements through the metrics/telemetry boundary. Health and alerts consume the resulting resource signals; they do not own the sampling implementation.

This is telemetry and belongs under the telemetry layer. It has a dedicated document because resource pressure can affect health and alerting.

## CPU

Measure process CPU rather than only host-wide CPU.

Preferred signals:

- cumulative process CPU time
- sampled process CPU utilization ratio
- optional host CPU utilization if the deployment environment exposes it

The sampled ratio should be calculated over a time interval rather than inferred from a single instantaneous reading.

Platform implementation:

- Unix: process/resource APIs available through libc/nix.
- Windows: process accounting through Win32 APIs.

The existing project already has platform-specific dependencies that can support this without introducing a large system-information framework.

## Memory

Measure:

- resident set size/process working set
- optional virtual memory size
- optional estimated cache/index memory

The primary alert signal should be resident/process memory.

Memory should be sampled periodically, e.g. every 5–15 seconds.

## File/storage usage

Track the filesystem containing:

- current log directory
- SQLite database
- SQLite WAL

Required signals:

- total filesystem capacity
- free bytes
- used bytes
- log directory bytes
- current log file bytes
- SQLite main database bytes
- SQLite WAL bytes

The filesystem free-space metric is more important than merely measuring file size.

## Log storage

Because MyDNS creates timestamped startup log files, the observability subsystem must know:

- active log filename
- active log size
- total logs directory size
- number of log files
- oldest log timestamp

Recommended operational controls:

- max active file size
- max total directory size
- max age
- max file count

Cleanup should be deterministic and logged.

## SQLite storage

Track:

- main `.db` file size
- `-wal` file size
- `-shm` file size where applicable
- database free/usable space
- migration version
- last successful database operation

Because MyDNS uses WAL mode and a single SQLx connection, WAL growth is operationally relevant.

Do not run expensive SQLite maintenance on every sample.

## Resource sampling

Use one background resource sampler:

```text
resource sampler
   |
   +--> CPU
   +--> memory
   +--> filesystem
   +--> logs
   +--> SQLite files
   |
   +--> resource metrics
   +--> logging on resource state transitions
   +--> health state input
   +--> alert evaluation inputs
```

Recommended sample interval: 10 seconds initially.

The interval should be configurable later if needed.

The sampler should not write logs, decide readiness, or fire notifications directly. It produces resource observations; logging, health, and alerting consume those observations according to their own responsibilities.

## Resource health states

Use threshold bands:

```text
NORMAL
  |
WARNING
  |
CRITICAL
```

Use hysteresis so a resource oscillating around a threshold does not constantly flip health state.

Resource pressure should not cause liveness failure.

Readiness impact depends on the resource:

- critically low disk -> readiness may fail
- high memory -> degraded first, readiness failure only at critical configured pressure
- high CPU -> normally degraded/alert only, not immediate readiness failure

## Privacy

Resource telemetry contains no user-level DNS data and is safe for normal operational dashboards.
