# Profiling

## Purpose

Profiling is a performance-analysis capability for MyDNS. It answers a different question from telemetry:

- **Metrics** — how much or how often?
- **Logs** — what happened?
- **Tracing** — what execution path did the operation take?
- **Profiling** — where is execution time being spent?

Profiling is therefore part of the broader observability architecture, but it is **not a fourth primary telemetry signal**.

## Current implementation

MyDNS currently integrates Samply through the tracing-samply crate.

The runtime profiling path is:

```text
MyDNS process
    |
observability pipeline
    |
optional profiling layer
    |
Samply profiler
    |
sampled execution profile
```

The profiling layer is optional:

- normal MyDNS startup does not require a profiler to be attached
- when no profiler is present, the profiling integration is skipped silently
- when Samply is attached, the telemetry pipeline installs the SamplyLayer
- profiling failures do not prevent MyDNS from starting

The integration is owned by:

```text
src/mydns/observability/profiling/
└── mod.rs
```

The telemetry pipeline composes this layer but does not own profiling semantics.

## What profiling provides

Sampling profiling periodically observes where the process is executing. Repeated samples can identify:

- CPU-heavy functions
- expensive DNS resolution paths
- database or serialization hotspots
- runtime scheduling overhead
- unexpected time spent in dependencies
- changes in execution cost between versions

This complements tracing. A trace can show that a DNS request spent time in cache lookup, local lookup, and upstream resolution; a profile can show which functions consumed the CPU during that execution.

## Profiling and tracing

Profiling and tracing should remain separate:

```text
                 MyDNS execution
                       |
             +---------+---------+
             |                   |
          tracing             profiling
             |                   |
      execution structure   sampled execution cost
             |                   |
        spans/events         Samply profile
```

Tracing should not be expanded into a profiler abstraction, and profiling should not become responsible for request/span semantics.

The profiling integration may share the Rust tracing-subscriber infrastructure because Samply is implemented as a subscriber layer. That is an implementation integration, not an ownership boundary.

## Production use

Profiling should normally be used as an explicit diagnostic capability rather than continuously treated as a required service signal.

Typical workflow:

1. Start MyDNS normally.
2. Attach Samply when investigating a performance issue.
3. Reproduce representative DNS/API workload.
4. Inspect the sampled profile for CPU hotspots.
5. Use tracing and metrics to identify the affected request class or operating path.
6. Fix the underlying code.
7. Re-profile to verify the change.

Profiling output should not be treated as an alerting source or as a replacement for service metrics.

## Performance and safety requirements

Profiling integration must:

- remain optional
- not prevent normal startup
- avoid introducing application-level profiling abstractions prematurely
- avoid putting profiling work directly into DNS request code
- remain bounded by the profiler's own sampling behavior
- remain independent of persistent trace storage

## Future scope

No additional profiling subsystem is planned at this stage.

Potential future work, only if needed:

- explicit profiling configuration
- documented profiling runbooks
- profile capture metadata
- automated performance-regression workflows
- correlation between profile captures and application versions

These are separate from the current telemetry instrumentation work and should only be added when there is a concrete operational or performance requirement.
