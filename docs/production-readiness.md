# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS to a production-ready baseline by addressing correctness, security, reliability, operational hardening, and regression coverage without weakening existing DNS behavior.

## Current baseline

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo clippy -- -D warnings` passes.
- Unit and integration tests pass: 16 total tests, 0 failures.
- Manual DNS smoke testing has covered A, AAAA, MX, NS, TXT, CNAME, PTR, NXDOMAIN, and cache-hit behavior.
- CNAME behavior has been compared against Cloudflare DNS for both working and non-working examples.

## Work plan

### P0 — Security and correctness

- [x] Bind DNS sockets before dropping Unix privileges.
- [x] Require authentication for the WebSocket dashboard endpoint.
- [x] Invalidate persistent cache entries when DNS records are created, updated, or deleted.
- [ ] Make cache result states explicit so positive empty results and NXDOMAIN are not represented ambiguously by `Vec<Record>`.
- [ ] Verify negative-cache persistence and invalidation across process restarts.
- [ ] Review all privileged operations and privilege-drop ordering for startup/shutdown races.

### P1 — Web/API hardening

- [ ] Review admin authentication/bootstrap behavior and secret handling.
- [ ] Review HTTP bind address and CORS defaults for safe deployment.
- [ ] Validate DNS record names, types, values, TTLs, and zone boundaries consistently at the API/database boundary.
- [ ] Review error responses for information leakage and consistent HTTP status handling.
- [ ] Add authorization tests for every protected API surface.

### P1 — DNS correctness and resolver behavior

- [ ] Audit CNAME handling, including CNAME-only responses and chained resolution.
- [ ] Verify NXDOMAIN versus NODATA semantics for all supported record types.
- [ ] Verify PTR handling and reverse-name normalization.
- [ ] Verify upstream failures/timeouts and SERVFAIL behavior.
- [ ] Verify TCP DNS behavior alongside UDP.
- [ ] Add regression tests for cache key normalization and record-type separation.

### P2 — Cache and persistence

- [ ] Review persistent cache schema and deduplication behavior.
- [ ] Ensure cache expiration is enforced consistently in memory and SQLite.
- [ ] Ensure record mutations invalidate all affected cached names/types.
- [ ] Add restart-persistence tests for positive and negative responses.
- [ ] Confirm cache behavior under concurrent requests.

### P2 — Operational readiness

- [ ] Review logging for useful structured context without sensitive data leakage.
- [ ] Review configuration defaults and startup validation.
- [ ] Add graceful shutdown coverage.
- [ ] Verify filesystem/database permissions and failure modes.
- [ ] Document deployment requirements, ports, privileges, database location, and configuration.
- [ ] Add a repeatable release smoke-test procedure.

### P2 — Test and CI coverage

- [ ] Add integration coverage for every supported DNS record type.
- [ ] Add explicit cache-hit/miss and invalidation assertions.
- [ ] Add negative-cache regression coverage.
- [ ] Add authenticated/unauthenticated WebSocket tests.
- [ ] Add API validation and authorization tests.
- [ ] Add CI workflow covering format, check, clippy, tests, and release build.
- [ ] Run the complete release smoke test against `target/release/mydns.exe`.

## Acceptance criteria

A production-readiness pass is complete when:

1. Formatting, compilation, clippy, unit tests, and integration tests all pass with zero warnings/errors.
2. DNS behavior is correct for supported record types, including CNAME, NXDOMAIN/NODATA, PTR, and upstream failure cases.
3. Cache invalidation is correct after record mutations and across process restarts.
4. Protected web/API/WebSocket surfaces require appropriate authentication and authorization.
5. Default network exposure and configuration are safe for deployment.
6. Privilege handling does not require unnecessary runtime privileges.
7. A clean release build passes the scripted DNS smoke test.
8. No generated test databases or other runtime artifacts remain in the working tree after tests.

## Execution order

1. Finish the explicit cache-state model and negative-cache semantics.
2. Add regression tests for cache persistence/invalidation and DNS edge cases.
3. Harden web/API authentication, validation, and network exposure.
4. Complete privilege and operational hardening review.
5. Add/verify CI and release smoke testing.
6. Run a final clean-tree audit and tag the production-ready commit.
