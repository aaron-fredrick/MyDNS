# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS from a feature-complete development server to a defensible production release by addressing DNS correctness, API/security hardening, persistence, reliability, test coverage, CI/CD, packaging, deployment, and operational documentation.

## Current baseline

- `cargo fmt --check` passes on the latest verified local baseline.
- `cargo check` passes on the latest verified local baseline.
- `cargo clippy -- -D warnings` passes on the latest verified local baseline.
- Unit tests pass: 18 passed.
- Existing API integration tests pass: 7 passed.
- DNS wire integration tests pass: 4 passed, covering UDP positive/NODATA/NXDOMAIN behavior, TCP positive-answer behavior, CNAME-only responses, multi-hop CNAME chains, and CNAME loops.
- Manual DNS smoke testing has covered A, AAAA, MX, NS, TXT, CNAME, PTR, NXDOMAIN, and cache-hit behavior.
- Dependencies were refreshed and `Cargo.lock` updated; `cargo audit` is still required.
- `production-readiness` is the active implementation branch.

## Progress since the original audit

### Completed

- [x] DNS UDP/TCP binding now honors configured `bind_host`.
- [x] DNS resolution now has an explicit internal outcome model for positive answers, NODATA, NXDOMAIN, and SERVFAIL.
- [x] Upstream resolver failures no longer automatically become NXDOMAIN; resolver errors are classified as NXDOMAIN, NODATA, or SERVFAIL.
- [x] NXDOMAIN remains the only negative result persisted through the existing negative-cache path.
- [x] Added real DNS wire-level integration coverage for UDP positive/NODATA/NXDOMAIN behavior and TCP positive-answer behavior.
- [x] Verified the DNS wire-level integration tests locally after correcting the fixture naming mismatch.
- [x] Implemented bounded authoritative CNAME chasing for locally managed records.
- [x] Added authoritative CNAME loop detection with `SERVFAIL` on cycles/recursion-limit exhaustion.
- [x] Added wire-level regression coverage for CNAME-only responses, multi-hop CNAME chains, and CNAME loops.
- [x] Persistent cache insertion now deduplicates rows using a database uniqueness constraint.
- [x] Cache invalidation now discovers and removes dependent CNAME aliases when a target record changes.
- [x] Added integration coverage for persistent-cache deduplication and dependent-cache invalidation.

### Still open from the audit

1. Cache restart/persistence and TTL lifecycle behavior still needs explicit integration coverage.
2. Unix privilege dropping needs a deliberate UID/GID/groups strategy and fail-closed behavior.
3. DNS/HTTP shutdown propagation is asymmetric.
4. Management API login/request abuse and resource limits need explicit controls.
5. HTTPS deployment needs to be implemented/documented and tested.
6. CI does not yet fully cover the active production-readiness branch.
7. Release workflow needs a complete verification/distribution contract.
8. README and deployment documentation need to match the current `config.ini`-based system.
9. Generated integration-test SQLite files must be cleaned up automatically and ignored appropriately.

## Decisions

### Configuration and exposure

- `config.ini` is the canonical runtime configuration source for release builds.
- `.env` is debug/development-only and must never implicitly supply release credentials.
- DNS and HTTP default to localhost; configured bind addresses must be honored exactly.
- An explicit `0.0.0.0` bind is an intentional request to expose the service on all interfaces.
- Missing admin username/password is a fatal startup configuration error.
- Production management traffic must use HTTPS directly or through a documented TLS-terminating reverse proxy.

### DNS behavior

- Distinguish `NOERROR/NODATA`, `NXDOMAIN`, and `SERVFAIL` internally and on the wire.
- Locally authoritative records must take precedence over upstream results.
- CNAME chaining must be bounded, loop-safe, and consistent between authoritative records and cached records.
- Supported record types and validation rules must be explicit.
- Upstream failures must never be cached as NXDOMAIN.

## Work plan

### P0 — Correctness and security blockers

- [x] Fix DNS UDP/TCP binding to honor `config.ini` `bind_host`.
- [x] Implement a typed DNS resolution outcome so `NOERROR/NODATA`, `NXDOMAIN`, and `SERVFAIL` cannot collapse into the same empty vector.
- [x] Fix upstream error handling so timeouts/unavailable resolvers produce `SERVFAIL`, not NXDOMAIN.
- [x] Implement consistent authoritative CNAME chasing, including bounded chains and loop detection.
- [ ] Add validation for DNS names, record types, record values, TTL bounds, and MX priority.
- [ ] Define and enforce zone/ownership rules for records managed through the API.
- [ ] Complete Unix privilege dropping with a deliberate UID/GID/groups strategy; fail closed if the requested least-privilege transition cannot be completed.
- [ ] Make shutdown propagation symmetric between DNS and HTTP and handle OS termination signals cleanly.
- [ ] Decide and implement HTTPS deployment: built-in TLS or a supported reverse-proxy topology.
- [ ] Add login/API request-size limits and login abuse/rate limiting appropriate for an admin service.

### P1 — DNS feature correctness

- [ ] Add authoritative handling/tests for A, AAAA, CNAME, MX, NS, TXT, and PTR.
- [x] Test CNAME-only responses and multi-hop CNAME chains.
- [x] Test CNAME loops and recursion limits.
- [x] Verify the new DNS outcome model over the real wire for positive answers, NODATA, and NXDOMAIN (UDP), plus positive answers over TCP.
- [ ] Verify upstream timeout, unreachable-server, malformed-response, and SERVFAIL behavior.
- [x] Verify UDP and TCP DNS listener behavior independently at the socket level and with wire-level integration tests.
- [ ] Verify query-name case normalization, trailing-dot normalization, and record-type separation.
- [ ] Verify response flags/authority behavior and TTL propagation.

### P1 — Cache and persistence

- [x] Define the persistent cache schema contract, including uniqueness/deduplication.
- [x] Prevent duplicate cache rows for the same owner/type/value where inappropriate.
- [x] Enforce TTL expiration consistently in memory and SQLite for normal cache access.
- [x] Invalidate all affected aliases when a CNAME target or dependent record changes.
- [ ] Add positive-cache restart tests.
- [ ] Add negative-cache restart tests.
- [x] Add cache mutation/invalidation coverage for record create/update/delete paths.
- [ ] Add explicit cache clear coverage.
- [ ] Add concurrent cache access tests and establish expected contention behavior.

### P1 — Web/API hardening

- [ ] Review admin bootstrap, password handling, JWT secret lifecycle, token lifetime, and rotation/recovery behavior.
- [ ] Add authorization tests for every protected route, including records, settings, cache, and WebSocket.
- [ ] Standardize HTTP status codes and error payloads; never leak internal errors in production responses.
- [ ] Add security headers appropriate for the dashboard deployment.
- [ ] Add audit logging for authentication and destructive/admin operations without logging passwords or JWTs.
- [ ] Add bounded request/body handling and concurrency/resource controls.

### P1 — Dependency and supply-chain security

- [ ] Run `cargo audit` and review every advisory against the actual dependency graph.
- [ ] Review Dependabot findings and close/upgrade/justify each finding.
- [ ] Keep direct dependency requirements intentional rather than relying on broad accidental transitive upgrades.
- [ ] Add dependency/license policy documentation if required for distribution.
- [ ] Consider SBOM generation for releases.
- [ ] Pin/verify GitHub Actions versions according to the project's supply-chain policy.

### P1 — Test suite expansion

- [ ] Add API validation tests for malformed names, unsupported types, invalid values, TTLs, and priorities.
- [ ] Add complete CRUD update coverage, including rename/type/value/TTL changes and cache invalidation.
- [ ] Add protected-route authorization tests for records/settings/cache/WebSocket.
- [ ] Add configuration parsing tests, including missing credentials and malformed values.
- [ ] Add bind-address tests for localhost and explicit interface/all-interface configuration.
- [ ] Add startup/shutdown and signal handling tests.
- [ ] Add DNS integration tests for every supported record type.
- [ ] Add NODATA/NXDOMAIN/SERVFAIL regression tests for the new outcome model.
- [ ] Add cache restart/persistence tests.
- [ ] Add WebSocket authentication and disconnect/lag handling tests.
- [ ] Make integration tests clean up all temporary SQLite files automatically, including failure paths.
- [ ] Add a deterministic test-data/temp-directory strategy so test artifacts never pollute the repository root.

### P1 — CI quality gates

- [ ] Run `cargo fmt --check` on every push/PR.
- [ ] Run `cargo check` on every push/PR.
- [ ] Run `cargo clippy -- -D warnings` on every push/PR.
- [ ] Run the complete test suite on Linux and Windows.
- [ ] Add a release-profile build/test job.
- [ ] Add CodeQL coverage to the active development/PR path.
- [ ] Add dependency auditing to CI.
- [ ] Configure branch protection so required CI checks gate merges to `main`.

### P2 — Release engineering and distributions

- [ ] Define supported release targets and test each target.
- [ ] Build versioned release archives containing the binary plus required example/config/documentation files.
- [ ] Generate SHA-256 checksums for every release artifact.
- [ ] Publish GitHub Releases from version tags with a clear artifact naming convention.
- [ ] Verify release binaries on clean machines/containers.
- [ ] Add release notes/changelog generation.
- [ ] Decide on SBOM/signing/provenance after the basic release pipeline is stable.
- [ ] Decide whether Linux needs `.deb`, `.rpm`, or tarball-only support for the first release.
- [ ] Decide whether Windows needs ZIP-only or an installer/service package.

### P2 — Containerisation

- [ ] Add a minimal production Dockerfile using a multi-stage build.
- [ ] Run the container as a non-root user where the DNS binding strategy permits it.
- [ ] If port 53 requires capabilities/host networking, document the exact requirement rather than using unrestricted `--privileged`.
- [ ] Keep SQLite/configuration outside the image via volumes/bind mounts.
- [ ] Add a suitable container healthcheck.
- [ ] Add a Compose example where useful.
- [ ] Scan the built image for known vulnerabilities.

### P2 — Service deployment

- [ ] Provide a Linux systemd unit with least-privilege settings, restart policy, limits, writable paths, and dependency ordering.
- [ ] Provide a Windows service/install procedure if Windows remains a supported production target.
- [ ] Document DNS port 53 UDP/TCP and management HTTP(S) ports.
- [ ] Document firewall rules, filesystem permissions, database backup/restore, and log rotation.
- [ ] Define a SQLite backup and recovery procedure.
- [ ] Define health/readiness checks and operational failure behavior.

### P2 — Documentation and release UX

- [ ] Rewrite README Quick Start for `config.ini`; remove stale `.env.example` instructions.
- [ ] Document supported DNS record types and actual resolver behavior.
- [ ] Document production topology, HTTPS requirement, ports, privileges, database/log locations, and backups.
- [ ] Add native and container deployment guides.
- [ ] Add a configuration reference with every supported key and default.
- [ ] Add a release/upgrade guide covering database compatibility and rollback.
- [ ] Add a changelog/release notes file.
- [ ] Document the security model and threat assumptions.

## Acceptance criteria

A production release is complete only when:

1. Format, check, clippy, unit tests, integration tests, and release-profile verification pass with zero warnings/errors.
2. DNS correctly distinguishes positive answers, NODATA, NXDOMAIN, and SERVFAIL.
3. All documented record types behave correctly over both UDP and TCP.
4. Authoritative CNAME chains are correct, bounded, and loop-safe.
5. Cache persistence, expiration, deduplication, and invalidation are verified across restarts and mutations.
6. Management surfaces are authenticated/authorized as intended, with abuse/resource limits and no credential/token leakage.
7. Default exposure is localhost-only and configured bind addresses are honored.
8. Production dashboard traffic is protected by HTTPS.
9. Unix privilege handling fails closed and does not leave the service unnecessarily privileged.
10. CI gates pull requests and releases; dependency/security checks are automated.
11. Release artifacts are versioned, checksummed, tested, and published with documented supported targets.
12. At least one documented native deployment and one documented container deployment are reproducible.
13. README and deployment/configuration documentation match the implementation.
14. A clean test run leaves no generated database/log/runtime artifacts in the working tree.
15. A final clean-tree audit is performed before tagging the production release.

## Execution order

1. **P0 correctness/security:** bind address, DNS outcome semantics, upstream failures, CNAME behavior, privilege handling, shutdown, HTTPS decision.
2. **Regression tests:** lock down NODATA/NXDOMAIN/SERVFAIL, CNAME, UDP/TCP, and bind behavior.
3. **API/cache hardening:** validation, authorization, cache invalidation/deduplication, restart/TTL persistence, resource limits, dependency audit.
4. **CI:** make format/check/clippy/tests/release verification authoritative and cover the active development branch.
5. **Documentation:** synchronize README/config/deployment/security documentation with implementation.
6. **Release engineering:** versioning, archives, checksums, supported targets, GitHub Releases.
7. **Deployment:** native service plus container/Compose, least privilege, persistent storage, HTTPS topology.
8. **Clean-machine validation:** verify native and container distributions from fresh environments.
9. **Final gate:** complete test/security review, clean-tree audit, release artifacts, release notes, then tag the production-ready commit.
