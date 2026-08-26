# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS from a feature-complete development server to a defensible production release by addressing DNS correctness, API/security hardening, persistence, reliability, test coverage, CI/CD, packaging, deployment, and operational documentation.

## Current baseline

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo clippy -- -D warnings` passes.
- Unit/integration tests currently pass; the existing plan records 18 tests with 0 failures.
- Release tests pass, including release CORS restriction coverage.
- Manual DNS smoke testing has covered A, AAAA, MX, NS, TXT, CNAME, PTR, NXDOMAIN, and cache-hit behavior.
- Dependency versions were recently refreshed and `Cargo.lock` was updated; the dependency tree still needs an explicit vulnerability/advisory audit before this is considered complete.
- `production-readiness` is the active implementation branch.

## Important code-audit findings

The current codebase is **not yet production-ready**, even though the basic checks pass. The following are concrete gaps found by reviewing the current branch:

1. **Configured DNS bind address is not actually used.** `AppConfig` exposes `bind_host`, but `src/dns/server.rs` binds UDP and TCP to `0.0.0.0` unconditionally. This contradicts the localhost-safe default and is a P0 correctness/security issue.
2. **Authoritative/manual CNAME resolution is incomplete.** Persistent-cache lookup chases CNAMEs, but the normal database-record path only looks up the requested record type. A locally configured CNAME therefore does not consistently resolve the target for A/AAAA/etc. queries.
3. **NXDOMAIN and NODATA are still conflated at the DNS protocol boundary.** The cache has explicit positive/negative states, but the request handler ultimately represents both missing records and empty upstream lookups as `Vec<Record>` and emits NXDOMAIN for an empty result. NODATA must produce `NOERROR` with an empty answer, while NXDOMAIN means the name does not exist.
4. **Upstream failure semantics are underspecified.** Upstream resolver errors currently collapse into `None`, after which the handler creates an NXDOMAIN cache entry. Timeouts/unreachable upstreams should not be advertised to clients as NXDOMAIN; SERVFAIL behavior needs explicit handling.
5. **Cache invalidation does not cover dependent CNAME aliases.** Name-wide invalidation is good for mutations to the owner, but changing a CNAME target can leave cached answers for aliases pointing at the old target.
6. **Persistent cache insertion can accumulate duplicate rows.** There is an index but no uniqueness/deduplication constraint and `insertCache` always inserts. The schema and insertion strategy need review.
7. **Privilege dropping is incomplete on Unix.** The current implementation changes UID to `nobody` but does not establish a complete least-privilege identity/group setup, and failure to find `nobody` merely logs a warning and continues privileged.
8. **Shutdown fate-sharing is asymmetric.** DNS cancellation propagates to HTTP, but HTTP termination does not cancel the DNS server. The comment in `main.rs` claims either server can trigger shutdown, which is not currently true.
9. **The management API is authenticated but not meaningfully rate-limited.** Login abuse/brute-force protection and request-size/resource limits need an explicit production decision.
10. **The dashboard is HTTP-only.** JWT credentials/tokens must not be sent over an untrusted network in cleartext. Production deployment therefore needs HTTPS termination (reverse proxy) or built-in TLS, with the chosen deployment model documented and tested.
11. **Current CI does not exercise the production-readiness branch.** `test.yml` and `codeql.yml` trigger on `main` and pull requests targeting `main`, so pushes to `production-readiness` do not receive the same checks.
12. **Release workflow builds binaries but does not run the full verification gate or publish a complete distribution contract.** It needs release tests, checksums, reproducible/versioned archives, and documented artifacts/targets.
13. **README is stale.** It still instructs users to copy `.env.example` and configure `.env`, while release configuration has moved to `config.ini`; it also understates the supported DNS records and deployment model.
14. **Generated test databases are not covered by the ignore rules.** The existing integration tests create random `test_*.db` files, and those can appear as untracked working-tree artifacts after tests.

These findings should be resolved before packaging/deployment work is treated as a production release.

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
- Supported record types and their validation rules must be explicit rather than inferred from whatever `hickory` happens to parse.

### Web/API

- Protected APIs and WebSocket endpoints require authentication.
- CORS is an additional browser policy, not an authorization mechanism.
- Login and API endpoints need bounded request sizes and basic abuse/rate-limit protection before Internet-facing deployment.
- Internal errors must be logged server-side without exposing implementation details to clients.

## Work plan

### P0 — Correctness and security blockers

- [ ] Fix DNS UDP/TCP binding to honor `config.ini` `bind_host`.
- [ ] Implement a typed DNS resolution outcome so `NOERROR/NODATA`, `NXDOMAIN`, and `SERVFAIL` cannot collapse into the same empty vector.
- [ ] Fix upstream error handling so timeouts/unavailable resolvers produce `SERVFAIL`, not NXDOMAIN.
- [ ] Implement consistent authoritative CNAME chasing, including bounded chains and loop detection.
- [ ] Add validation for DNS names, record types, record values, TTL bounds, and MX priority.
- [ ] Define and enforce zone/ownership rules for records managed through the API.
- [ ] Complete Unix privilege dropping with a deliberate UID/GID/groups strategy; fail closed if the requested least-privilege transition cannot be completed.
- [ ] Make shutdown propagation symmetric between DNS and HTTP and handle OS termination signals cleanly.
- [ ] Decide and implement HTTPS deployment: built-in TLS or a supported reverse-proxy topology.
- [ ] Add login/API request-size limits and login abuse/rate limiting appropriate for an admin service.

### P1 — Web/API hardening

- [ ] Review admin bootstrap, password handling, JWT secret lifecycle, token lifetime, and secret rotation/recovery behavior.
- [ ] Add authorization tests for every protected route, including records, settings, cache, and WebSocket.
- [ ] Standardize HTTP status codes and error payloads; never leak internal errors in production responses.
- [ ] Add security headers appropriate for the dashboard deployment.
- [ ] Add audit logging for authentication and destructive/admin operations without logging passwords or JWTs.
- [ ] Add bounded request/body handling and concurrency/resource controls.

### P1 — DNS feature correctness

- [ ] Add authoritative handling/tests for A, AAAA, CNAME, MX, NS, TXT, and PTR.
- [ ] Test CNAME-only responses and multi-hop CNAME chains.
- [ ] Test CNAME loops and recursion limits.
- [ ] Verify NXDOMAIN versus NODATA for every supported query type.
- [ ] Verify PTR normalization and reverse-name behavior.
- [ ] Verify upstream timeout, unreachable-server, malformed-response, and SERVFAIL behavior.
- [ ] Verify UDP and TCP DNS behavior independently.
- [ ] Verify query-name case normalization, trailing-dot normalization, and record-type separation.
- [ ] Verify response flags/authority behavior and TTL propagation against expected DNS semantics.

### P1 — Cache and persistence

- [ ] Define the persistent cache schema contract, including uniqueness/deduplication.
- [ ] Prevent duplicate cache rows for the same owner/type/value where inappropriate.
- [ ] Enforce TTL expiration consistently in memory and SQLite.
- [ ] Invalidate all affected aliases when a CNAME target or dependent record changes.
- [ ] Add positive-cache restart tests.
- [ ] Add negative-cache restart tests.
- [ ] Add cache mutation/invalidation tests for create, update, delete, and clear operations.
- [ ] Add concurrent cache access tests and establish expected contention behavior.

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
- [ ] Add NODATA/NXDOMAIN/SERVFAIL regression tests.
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
- [ ] Upload test/build artifacts only when useful for diagnosis.
- [ ] Configure branch protection so required CI checks gate merges to `main`.

### P2 — Release engineering and distributions

- [ ] Define the supported release targets and test each target rather than assuming every cross target works.
- [ ] Build versioned release archives containing the binary plus required example/config/documentation files.
- [ ] Generate SHA-256 checksums for every release artifact.
- [ ] Publish GitHub Releases from version tags with a clear artifact naming convention.
- [ ] Verify release binaries on clean machines/containers.
- [ ] Add release notes/changelog generation.
- [ ] Add optional SBOM/signing/provenance once the basic release pipeline is stable.
- [ ] Decide whether Linux distributions need `.deb`, `.rpm`, or tarball-only support for the first production release.
- [ ] Decide whether Windows distribution needs ZIP-only or an installer/service registration package.

### P2 — Containerisation

- [ ] Add a minimal production Dockerfile using a multi-stage build.
- [ ] Run the container as a non-root user where the chosen DNS binding strategy permits it.
- [ ] If port 53 requires host privileges/capabilities, document the exact container capability/network mode rather than using unrestricted `--privileged`.
- [ ] Keep the SQLite database and configuration outside the image via volumes/bind mounts.
- [ ] Add a container healthcheck suitable for the selected deployment topology.
- [ ] Add a `docker-compose.yml`/Compose example for local deployment where useful.
- [ ] Scan the built image for known vulnerabilities.

### P2 — Service deployment

- [ ] Provide a Linux systemd unit with least-privilege settings, restart policy, limits, writable paths, and dependency ordering.
- [ ] Provide a Windows service/install procedure if Windows is a supported production target.
- [ ] Document DNS port 53 UDP/TCP and management HTTP(S) port requirements.
- [ ] Document firewall rules, filesystem permissions, database backup/restore, and log rotation.
- [ ] Define a backup strategy for SQLite and a recovery procedure.
- [ ] Define health/readiness checks and operational failure behavior.

### P2 — Documentation and release UX

- [ ] Rewrite README Quick Start for `config.ini`; remove stale `.env.example` instructions.
- [ ] Document supported DNS record types and actual resolver behavior.
- [ ] Document production topology, HTTPS requirement, ports, privileges, database/log locations, and backups.
- [ ] Add a deployment guide with native and containerized options.
- [ ] Add a configuration reference with every supported key and default.
- [ ] Add a release/upgrade guide covering database compatibility and rollback.
- [ ] Add a changelog/release notes file.
- [ ] Document security model and threat assumptions.

## Acceptance criteria

A production release is complete only when:

1. Format, check, clippy, unit tests, integration tests, and release-profile verification pass with zero warnings/errors.
2. DNS correctly distinguishes positive answers, NODATA, NXDOMAIN, and SERVFAIL.
3. All documented record types behave correctly over both UDP and TCP.
4. Authoritative CNAME chains are correct, bounded, and loop-safe.
5. Cache persistence, expiration, deduplication, and invalidation are verified across restarts and mutations.
6. All management surfaces are authenticated/authorized as intended, with abuse/resource limits and no credential/token leakage.
7. Default exposure is localhost-only and configured bind addresses are actually honored.
8. Production dashboard traffic is protected by HTTPS.
9. Unix privilege handling fails closed and does not leave the service unnecessarily privileged.
10. CI gates pull requests and releases; dependency/security checks are automated.
11. Release artifacts are versioned, checksummed, tested, and published with documented supported targets.
12. At least one documented native deployment and one documented container deployment are reproducible.
13. README and deployment/configuration documentation match the actual implementation.
14. A clean test run leaves no generated database/log/runtime artifacts in the working tree.
15. A final clean-tree audit is performed before tagging the production release.

## Execution order

1. **Fix P0 code correctness/security blockers first:** bind address, DNS outcome semantics, CNAME behavior, upstream failures, privilege handling, shutdown, and HTTPS deployment decision.
2. **Expand tests around those fixes** so the intended behavior is locked down before further packaging work.
3. **Finish API/cache hardening** and run the dependency/security audit.
4. **Make CI authoritative** for format/check/clippy/tests/release verification and ensure the active branch is covered.
5. **Bring documentation in sync** with the real configuration and feature set.
6. **Implement release engineering:** versioning, archives, checksums, GitHub Releases, supported target validation.
7. **Implement deployment options:** native service plus container/Compose, with least privilege and persistent storage.
8. **Run clean-machine smoke tests** for native and container distributions.
9. **Final gate:** full test suite, security/dependency review, clean-tree audit, release artifacts, release notes, then tag the production-ready commit.
