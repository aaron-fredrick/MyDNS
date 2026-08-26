# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS from a feature-complete development server to a defensible production release by addressing DNS correctness, API/security hardening, persistence, reliability, test coverage, CI/CD, packaging, deployment, and operational documentation.

## Current baseline

- `cargo fmt --check` passes after the latest local formatting verification.
- `cargo check` passes.
- `cargo clippy -- -D warnings` passes.
- Unit tests: 18 passed.
- HTTP/API integration tests: 7 passed.
- DNS wire integration tests: 4 passed, covering UDP positive/NODATA/NXDOMAIN, TCP positive answers, CNAME-only responses, multi-hop CNAME chains, and CNAME loops.
- Persistent cache lifecycle integration tests: 5 tests exist; 4 pass on the previous local run, while the concurrent-upsert test exposed a test timing flaw caused by 1-second TTLs under SQLite contention. The test has been corrected to use 300+ second TTLs so it verifies deduplication rather than expiration timing.
- `cargo audit` remains outstanding.
- `production-readiness` is the active implementation branch.

## Progress since the original audit

### Completed

- [x] DNS UDP/TCP binding honors configured `bind_host`.
- [x] DNS resolution distinguishes positive, NODATA, NXDOMAIN, and SERVFAIL outcomes.
- [x] Upstream resolver failures are not collapsed into NXDOMAIN.
- [x] Authoritative CNAME chasing is bounded and loop-safe.
- [x] Wire-level DNS coverage exists for UDP/TCP, NODATA, NXDOMAIN, CNAME chains, CNAME-only responses, and CNAME loops.
- [x] Persistent cache insertion deduplicates records using a database uniqueness constraint.
- [x] Persistent cache lookups enforce TTL expiration.
- [x] CNAME-dependent persistent cache invalidation is implemented.
- [x] API integration coverage verifies cache deduplication and dependent invalidation.
- [x] Persistent cache lifecycle test target added in `tests/cache_persistence.rs`, covering restart persistence, negative-cache persistence, expiration/pruning, explicit clear, and concurrent upserts.

## Remaining priority order

### P0 — Correctness and security blockers

- [ ] Add validation for DNS names, record types, record values, TTL bounds, and MX priority.
- [ ] Define and enforce zone/ownership rules for API-managed records.
- [ ] Complete Unix privilege dropping with deliberate UID/GID/groups handling and fail-closed behavior.
- [ ] Make DNS/HTTP shutdown propagation symmetric and handle OS termination signals cleanly.
- [ ] Decide and implement HTTPS deployment: built-in TLS or documented TLS-terminating reverse proxy.
- [ ] Add login/API request-size limits and login abuse/rate limiting.

### P1 — DNS correctness

- [ ] Add authoritative wire-level coverage for A, AAAA, CNAME, MX, NS, TXT, and PTR.
- [ ] Verify upstream timeout, unreachable-server, malformed-response, and SERVFAIL behavior.
- [ ] Verify query-name case normalization, trailing-dot normalization, and record-type separation.
- [ ] Verify response flags, authority behavior, and TTL propagation.

### P1 — Cache and persistence

- [x] Persistent-cache schema/uniqueness contract.
- [x] Duplicate-write prevention.
- [x] SQLite TTL filtering.
- [x] CNAME-dependent invalidation.
- [x] Positive-cache restart persistence test.
- [x] Negative-cache restart persistence test.
- [x] Expired-cache visibility/pruning test.
- [x] Explicit persistent cache clear test.
- [x] Concurrent identical cache-upsert test design, corrected to avoid false failures from 1-second TTL expiration.
- [ ] Verify the corrected lifecycle suite locally and integrate it into the normal CI matrix.
- [ ] Make all integration-test temporary database cleanup failure-safe.

### P1 — Web/API hardening

- [ ] Review admin bootstrap, password handling, JWT secret lifecycle, token lifetime, and rotation/recovery.
- [ ] Add authorization tests for every protected route, including records, settings, cache, and WebSocket.
- [ ] Standardize HTTP status codes/error payloads without leaking internals.
- [ ] Add security headers.
- [ ] Add audit logging for authentication and destructive/admin operations without credentials/tokens.
- [ ] Add bounded request/body and concurrency/resource controls.

### P1 — Dependency and supply-chain security

- [ ] Run `cargo audit` and review every advisory against the actual dependency graph.
- [ ] Review the current Dependabot findings and upgrade/justify each one.
- [ ] Keep direct dependency requirements intentional.
- [ ] Pin/verify GitHub Actions versions.
- [ ] Decide whether SBOM generation is required for releases.

### P1 — Test-suite expansion

- [ ] Add API validation tests for malformed names, unsupported types, invalid values, TTLs, and priorities.
- [ ] Add complete CRUD update coverage, including rename/type/value/TTL changes and invalidation.
- [ ] Add protected-route authorization coverage for records/settings/cache/WebSocket.
- [ ] Add configuration parsing tests, including missing credentials and malformed values.
- [ ] Add bind-address tests for localhost and explicit interface/all-interface configuration.
- [ ] Add startup/shutdown and signal-handling tests.
- [ ] Add DNS tests for all supported record types over UDP and TCP.
- [ ] Add WebSocket authentication and disconnect/lag handling tests.
- [ ] Adopt a deterministic temporary-directory strategy so test artifacts never pollute the repository root.

### P1 — CI quality gates

- [ ] Run `cargo fmt --check` on every push/PR.
- [ ] Run `cargo check` on every push/PR.
- [ ] Run `cargo clippy -- -D warnings` on every push/PR.
- [ ] Run the complete test suite on Linux and Windows.
- [ ] Add release-profile build/test verification.
- [ ] Add CodeQL coverage to the active development/PR path.
- [ ] Add dependency auditing to CI.
- [ ] Configure branch protection so required checks gate merges to `main`.

### P2 — Release engineering

- [ ] Define supported release targets.
- [ ] Build versioned release archives containing binaries, configuration examples, and required documentation.
- [ ] Generate SHA-256 checksums.
- [ ] Publish GitHub Releases from version tags.
- [ ] Verify release binaries on clean machines/containers.
- [ ] Add release notes/changelog generation.
- [ ] Decide on SBOM/signing/provenance.
- [ ] Decide Linux tarball vs `.deb`/`.rpm` packaging.
- [ ] Decide Windows ZIP vs installer/service packaging.

### P2 — Containerisation

- [ ] Add a minimal multi-stage production Dockerfile.
- [ ] Run as non-root where the DNS binding strategy permits it.
- [ ] Document any required DNS capability/host-network configuration; never require unrestricted `--privileged`.
- [ ] Keep SQLite/configuration persistent outside the image.
- [ ] Add a container healthcheck.
- [ ] Add a Compose example where useful.
- [ ] Scan the built image for known vulnerabilities.

### P2 — Native service deployment

- [ ] Provide a Linux systemd unit with least privilege, restart policy, limits, writable paths, and dependency ordering.
- [ ] Provide a Windows service/install procedure if Windows remains supported.
- [ ] Document DNS UDP/TCP and management HTTP(S) ports.
- [ ] Document firewall rules, filesystem permissions, database backup/restore, and log rotation.
- [ ] Define health/readiness checks and operational failure behavior.

### P2 — Documentation

- [ ] Rewrite README Quick Start for `config.ini` and remove stale `.env.example` instructions.
- [ ] Document supported DNS record types and resolver behavior.
- [ ] Document production topology, HTTPS, ports, privileges, database/log locations, and backups.
- [ ] Add native and container deployment guides.
- [ ] Add a complete configuration reference.
- [ ] Add release/upgrade and rollback guidance.
- [ ] Add changelog/release notes.
- [ ] Document the security model and threat assumptions.

## Acceptance criteria

A production release is complete only when:

1. Format, check, clippy, unit tests, integration tests, and release-profile verification pass with zero warnings/errors.
2. DNS correctly distinguishes positive answers, NODATA, NXDOMAIN, and SERVFAIL.
3. All documented record types behave correctly over both UDP and TCP.
4. Authoritative CNAME chains are correct, bounded, and loop-safe.
5. Cache persistence, expiration, deduplication, invalidation, restart behavior, clear behavior, and concurrency are verified.
6. Management surfaces are authenticated/authorized as intended, with abuse/resource limits and no credential/token leakage.
7. Default exposure is localhost-only and configured bind addresses are honored.
8. Production dashboard traffic is protected by HTTPS.
9. Unix privilege handling fails closed and does not leave the service unnecessarily privileged.
10. CI gates pull requests and releases; dependency/security checks are automated.
11. Release artifacts are versioned, checksummed, tested, and published for documented targets.
12. At least one documented native deployment and one documented container deployment are reproducible.
13. README and deployment/configuration documentation match the implementation.
14. A clean test run leaves no generated database/log/runtime artifacts in the working tree.
15. A final clean-tree audit is performed before tagging the production release.

## Immediate next steps

1. Pull the latest `production-readiness` branch.
2. Run `cargo fmt --check`.
3. Run `cargo check`.
4. Run `cargo clippy -- -D warnings`.
5. Run `cargo test --test cache_persistence -- --nocapture` and confirm all 5 lifecycle tests pass with the corrected TTLs.
6. Run `cargo test` and confirm the complete suite is green.
7. Remove any generated `test_*.db` / `test_dns_*.db` artifacts left in the working tree; do not commit them.
8. If clean, commit/push only the intended changes.
9. Then start the next implementation tranche: **P0 DNS/API input validation and zone/ownership enforcement**.

## Execution order

1. **P0 correctness/security:** validation, ownership, privilege handling, shutdown, HTTPS, abuse controls.
2. **P1 DNS/cache:** complete DNS record-type behavior and upstream failure coverage; finish cache CI integration.
3. **P1 API/security:** authorization, error handling, headers, audit logging, resource limits.
4. **P1 CI/security:** audit dependencies, harden CI, enable branch protection.
5. **P2 deployment:** native service, container, HTTPS topology, backups.
6. **P2 release:** artifacts, checksums, supported targets, clean-machine verification, release notes.
7. **Final gate:** complete security review, clean-tree audit, reproducible deployment verification, then tag the production-ready commit.
