# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS to a production-ready baseline by addressing correctness, security, reliability, operational hardening, and regression coverage without weakening existing DNS behavior.

## Current baseline

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo clippy -- -D warnings` passes.
- Unit and integration tests pass: 18 total tests, 0 failures.
- Release tests pass, including release CORS restriction coverage.
- Manual DNS smoke testing has covered A, AAAA, MX, NS, TXT, CNAME, PTR, NXDOMAIN, and cache-hit behavior.
- Manual release CORS testing confirms the configured HTTP origin is accepted and an unrelated origin is rejected.
- `production-readiness` is the active implementation branch; work continues on this branch.

## Decisions

### Configuration

- `config.ini` is the canonical runtime configuration source for release builds.
- Do not use environment variables as the primary production configuration mechanism.
- `.env` may be used only for debug/development builds and must never provide release credentials implicitly.
- DNS and HTTP bind addresses default to localhost (`127.0.0.1`) unless explicitly overridden in `config.ini`.
- An explicit `0.0.0.0` bind is treated as an intentional request to expose the service on all interfaces.
- Missing admin username or password is a fatal startup configuration error. There is no insecure default admin credential.

### Web/CORS

- Debug builds may continue to use `CorsLayer::permissive()` for development convenience.
- Release builds use an explicit CORS allowlist; `permissive()` is not allowed.
- By default, release CORS origins are derived from the configured HTTP bind address plus the default dashboard hostname `mydns.local` where applicable.
- If HTTP binds to `0.0.0.0`, release CORS may include the machine's usable local interface IP origins plus `mydns.local`.
- If a domain list is configured, that list is authoritative for domain-based origins rather than automatically allowing arbitrary hostnames.
- CORS remains separate from authentication/authorization; protected APIs must still require valid credentials.

### Default dashboard hostname

- The default dashboard hostname is `mydns.local`.
- CORS acceptance of `mydns.local` does not imply that MyDNS must implement mDNS or automatically modify the OS hosts file.
- Actual name resolution for `mydns.local` is a separate concern and may be addressed later.

## Work plan

### P0 — Configuration, network exposure, and security

- [x] Bind DNS sockets before dropping Unix privileges.
- [x] Require authentication for the WebSocket dashboard endpoint.
- [x] Invalidate persistent cache entries when DNS records are created, updated, or deleted.
- [x] Make cache result states explicit so positive empty results and NXDOMAIN are not represented ambiguously by `Vec<Record>`.
- [x] Implement `config.ini` parsing as the canonical release configuration path.
- [x] Move DNS and HTTP bind configuration to `config.ini` with localhost-safe defaults.
- [x] Remove insecure default admin username/password behavior and fail fast when either credential is missing.
- [x] Restrict `.env` handling to debug/development builds only.
- [x] Implement release CORS allowlist generation from bind address and configured domains.
- [ ] Verify negative-cache persistence and invalidation across process restarts.
- [ ] Review all privileged operations and privilege-drop ordering for startup/shutdown races.

### P1 — Web/API hardening

- [ ] Review admin authentication/bootstrap behavior and secret handling after the configuration migration.
- [x] Replace release `CorsLayer::permissive()` with explicit origin/method/header policy.
- [ ] Validate DNS record names, types, values, TTLs, and zone boundaries consistently at the API/database boundary.
- [ ] Review error responses for information leakage and consistent HTTP status handling.
- [ ] Add authorization tests for every protected API surface.
- [x] Add regression tests for debug-permissive versus release-restricted CORS behavior.

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

### P2 — Dependency and operational security

- [ ] Audit GitHub/Dependabot dependency findings and map each advisory to the actual Cargo dependency tree.
- [ ] Upgrade vulnerable dependencies where practical without unnecessary breaking changes.
- [ ] Review logging for useful structured context without sensitive data leakage.
- [ ] Review configuration defaults and startup validation.
- [ ] Add graceful shutdown coverage.
- [ ] Verify filesystem/database permissions and failure modes.
- [ ] Document deployment requirements, ports, privileges, database location, and `config.ini` configuration.
- [ ] Add a repeatable release smoke-test procedure.

### P2 — Test and CI coverage

- [ ] Add integration coverage for every supported DNS record type.
- [ ] Add explicit cache-hit/miss and invalidation assertions.
- [ ] Add negative-cache regression coverage.
- [ ] Add authenticated/unauthenticated WebSocket tests.
- [ ] Add API validation and authorization tests.
- [ ] Add configuration parsing and fail-fast startup tests.
- [ ] Add bind-address/default-exposure tests.
- [x] Add release CORS origin-generation tests.
- [ ] Add CI workflow covering format, check, clippy, tests, and release build.
- [x] Run the complete release smoke test against `target/release/mydns.exe`.

## Acceptance criteria

A production-readiness pass is complete when:

1. Formatting, compilation, clippy, unit tests, and integration tests all pass with zero warnings/errors.
2. DNS behavior is correct for supported record types, including CNAME, NXDOMAIN/NODATA, PTR, and upstream failure cases.
3. Cache invalidation is correct after record mutations and across process restarts.
4. Protected web/API/WebSocket surfaces require appropriate authentication and authorization.
5. Release configuration comes from `config.ini`, missing admin credentials fail fast, and default network exposure is localhost-only.
6. Release CORS is explicit and does not use `CorsLayer::permissive()`.
7. Privilege handling does not require unnecessary runtime privileges.
8. A clean release build passes the scripted DNS smoke test.
9. No generated test databases or other runtime artifacts remain in the working tree after tests.

## Execution order

1. ~~Implement `config.ini` parsing and configuration precedence.~~ **Done.**
2. ~~Implement localhost-safe DNS/HTTP binding with explicit override support.~~ **Done.**
3. ~~Remove insecure admin credential defaults and add fail-fast validation.~~ **Done.**
4. ~~Implement debug/release CORS behavior and default/configured domain handling.~~ **Done.**
5. ~~Add configuration, binding, CORS, and authentication regression tests.~~ **CORS/auth coverage done; configuration/binding test expansion remains.**
6. **Next: complete the dependency vulnerability audit and targeted upgrades.**
7. Continue DNS/cache correctness, persistence, privilege, and operational hardening.
8. Verify CI and release smoke testing.
9. Run a final clean-tree audit and tag the production-ready commit.
