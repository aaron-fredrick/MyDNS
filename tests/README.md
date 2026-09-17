# Test Suite Layout

Organize tests by purpose rather than by production module.

```text
 tests/
 ├── unit/          # isolated/unit-like black-box tests; true Rust unit tests stay beside src
 ├── integration/   # cross-module and subsystem contract tests
 ├── e2e/           # full running-system workflows
 ├── smoke/         # fast post-build/install/deployment health checks
 ├── load/          # performance, stress, concurrency, capacity, and soak tests
 ├── fixtures/      # reusable deterministic test inputs
 ├── support/       # harnesses, clients, process lifecycle, and assertions
 └── common/        # existing shared Rust integration-test helpers
```

The category directories are documentation-only placeholders for now. No test implementation is being moved or added by this cleanup.

### Cargo note

Existing Rust integration-test entry points remain at the top level of `tests/` because Cargo discovers those files automatically. Do not move the current `.rs` entry points into category directories until explicit Cargo test-target/module wiring is introduced.

### Coverage intent

- **Unit:** deterministic validation, parsing, normalization, data structures, and error behavior.
- **Integration:** DNS, local DNS, blocklist, cache, database, API, auth, and upstream contracts.
- **E2E:** real process + DNS/API clients + frontend workflows + lifecycle operations.
- **Smoke:** minimal release/deployment checks.
- **Load:** DNS, API, mixed workload, cache/blocklist scale, upstream degradation, resource saturation, and long-running soak.
