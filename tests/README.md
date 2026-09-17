# Test Suite Layout

This directory is organized by test purpose rather than by implementation module.

```text
 tests/
 ├── unit/          # Small isolated black-box test plans; most Rust unit tests remain beside src code
 ├── integration/   # Cross-module Rust tests and subsystem contracts
 ├── e2e/           # Full running-system workflows, including API + frontend flows
 ├── smoke/         # Fast release/deployment health checks
 ├── load/          # Performance, stress, soak, concurrency, and capacity tests
 ├── fixtures/      # Reusable DNS, API, config, and dataset fixtures
 ├── support/       # Test harnesses, process helpers, clients, and assertions
 └── common/        # Existing shared Rust integration-test helpers; migrate here only with test-module changes
```

## Scope

- **Unit:** deterministic, isolated behavior. Prefer `#[cfg(test)]` modules beside Rust production code for true unit tests.
- **Integration:** DNS resolution, blocklist, cache, database, API, auth, upstream, and subsystem contracts.
- **E2E:** a real MyDNS process plus HTTP/DNS clients and, where applicable, the built React frontend.
- **Smoke:** minimal checks suitable immediately after install, upgrade, startup, or deployment.
- **Load:** DNS query throughput, mixed DNS workloads, HTTP API load, concurrent mutations/reads, cache behavior, blocklist scale, upstream latency, sustained soak, and resource saturation.

The category directories are intentionally documentation-only for now. Test implementations can be added incrementally without changing the product architecture.

## Naming and execution

Rust integration-test entry points currently live at the top level of `tests/` because Cargo discovers those files automatically. Do not move existing `.rs` entry points into subdirectories until the corresponding Cargo test-target/module structure is updated. The folders below provide the target structure for future cleanup.
