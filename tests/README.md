# Test Suite Layout

Organize tests by purpose rather than by production module.

```text
 tests/
 ├── unit/
 │   └── frontend/       # frontend unit tests; Rust unit tests stay beside src
 ├── integration/        # cross-module and subsystem contract tests
 ├── e2e/                # full running-system workflows
 ├── smoke/              # fast post-build/install/deployment health checks
 ├── load/               # performance, stress, concurrency, capacity, and soak tests
 │   └── scenarios/      # editable JSON workload definitions
 ├── fixtures/           # reusable deterministic test inputs
 └── support/            # shared Rust harnesses, clients, lifecycle helpers
```

## Integration-test organization

The existing Rust integration tests have been moved under `tests/integration/` and are registered explicitly in `Cargo.toml`. This keeps the source tree organized while preserving normal `cargo test` and `cargo llvm-cov --all-targets` discovery.

Shared HTTP/DNS/database fixtures now live in `tests/support/mod.rs`. The small `tests/integration/common.rs` adapter imports those helpers so the existing `mod common;` declarations remain simple and Cargo does not treat the helper as a separate test target.

## Frontend unit tests

Frontend tests live under `tests/unit/frontend/`. They use Node's built-in test runner so the test suite does not require a second frontend test framework just to exercise pure TypeScript utilities and API-client behavior. Coverage is collected separately and uploaded to Codecov with the `frontend` flag.

## Load testing

Python load generators live under `tests/load/` and use only the standard library. Scenarios support a controlled:

```text
warmup → sustain → burst → recovery
```

pattern with an explicit RPS safety ceiling. This allows repeatable tests that hold a manageable request rate, abruptly increase load, and then verify recovery. DNS scenarios accept custom query lists so local records, cacheable public names, NXDOMAINs, and blocklist candidates can be mixed deliberately.

Load tests are manual/opt-in performance tests, not normal CI correctness tests. They must target an explicitly selected test instance.

## Test layers

- **Unit:** deterministic validation, parsing, normalization, data structures, error behavior, and frontend client/utilities.
- **Integration:** DNS, Local DNS, blocklist, cache, SQLite persistence, API, auth, validation, upstream, and observability contracts.
- **E2E:** real MyDNS process + DNS/API clients + frontend workflows + lifecycle operations.
- **Smoke:** minimal release/deployment health checks.
- **Load:** DNS/API throughput, burst handling, latency percentiles, cache/blocklist scale, upstream degradation, resource saturation, and soak/recovery.


## Current coverage suites

The integration layer now includes records_db.rs, covering DNS record CRUD, dev-record cleanup, admin seeding, zone lifecycle/apex records, config-zone normalization, settings persistence, cache identity, and recursive CNAME-dependent cache invalidation.

The blocklist integration suite covers enabled-domain projection, update/delete behavior, invalid sources, duplicate canonical domains, and DNS enforcement.

Black-box helpers are also available:

- tests/e2e/api_e2e.py — authenticated records, zones, blocklist, settings, stats, and history workflow against a running instance.
- tests/smoke/api-smoke.ps1 — fast HTTP readiness/auth/records/history smoke check.

Run the Rust correctness and coverage suite with cargo test --all-targets --all-features and cargo llvm-cov --all-targets --all-features --lcov --output-path lcov.info.

Run the frontend unit suite with the existing c8 command documented in .github/workflows/README.md.

Run the API E2E workflow against an isolated test instance with: python tests/e2e/api_e2e.py --base-url http://127.0.0.1:8080 --username admin --password "$MYDNS_ADMIN_PASSWORD"

The E2E and smoke layers intentionally remain outside ordinary unit-test coverage: their purpose is black-box contract and deployment verification rather than line coverage.
## CI test order

The required CI progression is:

1. Unit
2. Component
3. Contract (frontend/backend compatibility)
4. Integration
5. Smoke
6. E2E (policy-dependent)

A failure at a stage prevents dependent stages from running. Contract testing is required before integration and smoke for pull requests targeting any branch, and for dev/main policies.

Load and fuzz testing are separate manual-only workflows for now and are not policy gates.
