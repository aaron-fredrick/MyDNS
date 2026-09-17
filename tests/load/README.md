# Load, Stress, and Soak Tests

Load tests are intentionally separate from correctness tests. They should exercise a running MyDNS instance and record throughput, latency, failures, and recovery behavior.

## Controlled phase model

The Python generators support a phase sequence such as:

```text
warmup → sustain → burst → recovery
```

This makes it possible to hold a manageable baseline rate, suddenly increase request pressure, then verify that the resolver recovers after the burst.

Each phase has:

- `name`
- `rps`
- `duration_seconds`

Every scenario also has an explicit `max_rps` safety ceiling. Increase that ceiling deliberately when moving to a higher-capacity test environment.

## DNS load generator

`dns_load.py` uses only the Python standard library and sends raw DNS/UDP queries. No external DNS resolver package is required.

Example:

```bash
python tests/load/dns_load.py --scenario tests/load/scenarios/dns-burst.json
```

The example scenario uses a deliberately modest baseline and a short burst. Edit the query list, phase rates, durations, worker count, timeout, and safety ceiling for a specific test environment.

The generator accepts DNS responses with `NOERROR` and `NXDOMAIN` as valid protocol outcomes. Transport failures and other DNS response codes are reported as load-test failures.

## API load generator

`api_load.py` applies the same phase model to HTTP GET endpoints:

```bash
python tests/load/api_load.py --scenario tests/load/scenarios/api-burst.json
```

For authenticated endpoints, pass a bearer token with `--token` rather than embedding credentials in a scenario file.

## Planned workload families

1. DNS throughput and burst handling
2. DNS p50/p95/p99/p99.9 latency
3. HTTP API read/write/authenticated load
4. Mixed DNS + API + WebSocket concurrency
5. cache hit/miss and eviction pressure
6. blocklist lookup and large-rule-set scale
7. SQLite read/write contention
8. upstream latency, timeout, retry, and failure behavior
9. CPU, memory, sockets/file descriptors, and disk I/O saturation
10. multi-hour soak and recovery behavior
11. repeatable capacity/regression baselines between releases

For every recorded run, capture the MyDNS build/version, workload scenario, concurrency, duration, achieved throughput, latency percentiles, error rate, and resource observations. Load tests must target an explicitly selected test instance; do not point burst scenarios at production or an uncontrolled public resolver.
