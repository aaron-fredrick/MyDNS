# Load, Stress, and Soak Tests

This area is for performance validation across the full MyDNS surface, not only DNS packet throughput.

## Planned workload sectors

1. **DNS throughput** — sustained queries/sec, burst traffic, concurrency, UDP/TCP behavior where supported.
2. **DNS latency** — p50/p95/p99/p99.9 latency for cache hits, local records, blocked queries, and upstream misses.
3. **HTTP API load** — read-heavy, write-heavy, authenticated, and mixed endpoint workloads.
4. **Mixed workload** — DNS traffic running concurrently with API reads/mutations and WebSocket clients.
5. **Cache pressure** — hit rate, eviction behavior, memory growth, and persistent-cache interaction.
6. **Blocklist scale** — large rule sets, lookup latency, hot updates, and exact/subdomain matching cost.
7. **Database pressure** — concurrent reads/writes, migration/startup behavior, and SQLite contention.
8. **Upstream dependency** — latency, failures, timeouts, retry behavior, and degraded upstream conditions.
9. **Resource saturation** — CPU, memory, file descriptors/sockets, connection counts, and disk I/O.
10. **Soak/stability** — multi-hour sustained traffic, leak detection, counter stability, and recovery after transient faults.
11. **Capacity/regression** — repeatable baselines per release and comparison against previous builds.

## Rule

Load tests must record workload, concurrency, duration, environment, build/version, and observed p50/p95/p99 latency plus throughput and error rate. They are not substitutes for correctness tests.
