# Load, Stress, and Soak Tests

Performance testing is broader than DNS packets. Future load tooling should cover:

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

Each run should record build/version, workload, concurrency, duration, throughput, latency percentiles, error rate, and resource observations.
