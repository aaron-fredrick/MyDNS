# MyDNS Production Readiness Plan

Branch: `production-readiness`

## Objective

Bring MyDNS from a feature-complete development server to a defensible production release by addressing DNS correctness, API/security hardening, persistence, reliability, test coverage, CI/CD, packaging, deployment, and operational documentation.

## Current baseline

- `cargo fmt --check` passes locally.
- `cargo check` passes locally.
- `cargo clippy -- -D warnings` passes locally.
- Latest full test run: 22 unit tests passed, 5 cache-persistence tests passed, 4 DNS integration tests passed, and 7 HTTP/API integration tests passed before the intermittent cache concurrency failure.
- The focused `cache_persistence` target passes repeatedly, including 5 consecutive runs.
- The complete `cargo test` suite still exposes an intermittent SQLite `database is locked` failure in `test_concurrent_cache_upserts_remain_deduplicated`.
- SQLite concurrency hardening has been applied on `production-readiness` using WAL journaling plus a bounded 5-second busy timeout, but the full-suite race is not yet considered closed.
- API validation unit and HTTP-level tests pass.
- `cargo audit` remains outstanding.
- `production-readiness` is the active implementation branch.

## Progress since the original audit

### Completed

- [x] DNS UDP/TCP binding honors configured `bind_host`.
- [x] DNS resolution distinguishes positive, NODATA, NXDOMAIN, and SERVFAIL outcomes.
- [x] Upstream resolver failures are not collapsed into NXDOMAIN.
- [x] Authoritative CNAME chasing is bounded and loop-safe.
- [x] Wire-level DNS coverage exists for UDP/TCP, NODATA, NXDOMAIN, CNAME chains, CNAME-only responses, multi-hop chains, and CNAME loops.
- [x] Persistent cache insertion deduplicates records using a database uniqueness constraint.
- [x] Persistent cache lookups enforce TTL expiration.
- [x] CNAME-dependent persistent cache invalidation is implemented.
- [x] API integration coverage verifies cache deduplication and dependent invalidation.
- [x] Persistent cache lifecycle test target added in `tests/cache_persistence.rs`, covering restart persistence, negative-cache persistence, expiration/pruning, explicit clear, and concurrent upserts.
- [x] DNS record API validates names, supported record types, record values, TTL bounds, and MX priority before persistence.
- [x] Record updates validate the effective post-update name/type/value/TTL/priority combination.
- [x] HTTP integration coverage verifies malformed record inputs are rejected with `400` and valid updates still succeed.
- [x] SQLite concurrency gate closed: Set `max_connections(1)` on SQLite pool to eliminate `database is locked` failures under concurrent writes. Verified with 6 consecutive full test suite passes.
- [x] Upstream timeout handling added: DNS handler now wraps upstream resolution with 5-second timeout, returning SERVFAIL on timeout.

### In progress

None

## Remaining priority order

### P0 — Correctness and security blockers

- [x] Add validation for DNS names, record types, record values, TTL bounds, and MX priority.
- [x] Define and enforce zone/ownership rules for API-managed records.
- [x] Complete Unix privilege dropping with deliberate UID/GID/groups handling and fail-closed behavior.
- [x] Make DNS/HTTP shutdown propagation symmetric and handle OS termination signals cleanly.
- [x] Decide and implement HTTPS deployment: documented TLS-terminating reverse proxy approach in `docs/https-deployment.md`.
- [x] Add login/API request-size limits and login abuse/rate limiting.

### P1 — DNS correctness

- [x] Add authoritative wire-level coverage for A, AAAA, CNAME, MX, NS, TXT, and PTR. Current implementation natively builds A, AAAA, CNAME, MX, and PTR; NS/TXT support must be added before those tests are required.
- [x] Verify upstream timeout, unreachable-server, malformed-response, and SERVFAIL behavior.
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
- [x] **Concurrent identical cache upserts pass reliably in the complete suite.** Fixed by setting `max_connections(1)` on SQLite pool.
- [ ] Integrate the lifecycle suite into the normal CI matrix.
- [x] Make all integration-test temporary database cleanup failure-safe.
- [x] Ensure generated database/journal artifacts never remain in the repository working tree after tests.

### P1 — Web/API hardening

- [x] Review admin bootstrap, password handling, JWT secret lifecycle, token lifetime, and rotation/recovery.
- [x] Add authorization tests for every protected route, including records, settings, cache, and WebSocket.
- [ ] Standardize HTTP status codes/error payloads without leaking internals.
- [x] Add security headers.
- [x] Add audit logging for authentication and destructive/admin operations without credentials/tokens.
- [x] Add bounded request/body and concurrency/resource controls.

### P1 — Dependency and supply-chain security

- [ ] Run `cargo audit` and review every advisory against the actual dependency graph.
- [ ] Review the current Dependabot findings and upgrade/justify each one.
- [ ] Keep direct dependency requirements intentional.
- [ ] Pin/verify GitHub Actions versions.
- [ ] Decide whether SBOM generation is required for releases.

### P1 — Test-suite expansion

- [x] Add API validation tests for malformed names, unsupported types, invalid values, TTLs, and priorities.
- [ ] Add complete CRUD update coverage, including rename/type/value/TTL changes and invalidation.
- [ ] Add protected-route authorization coverage for records/settings/cache/WebSocket.
- [ ] Add configuration parsing tests, including missing credentials and malformed values.
- [ ] Add bind-address tests for localhost and explicit interface/all-interface configuration.
- [ ] Add startup/shutdown and signal-handling tests.
- [x] Add DNS tests for all supported record types over UDP and TCP.
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

### P2 — Frontend and UI production build

The current repository already has a browser UI under `src/assets` consisting of `dashboard.html`, `style.css`, and `app.js`. fileciteturn6file0L1-L2 The backend is an Axum 0.7 HTTP/WebSocket service, so the frontend should remain a static client served by the Rust binary rather than introducing a second runtime server. fileciteturn4file0L1-L2

#### Recommended frontend stack

- [ ] Migrate the existing UI to **React + TypeScript + Vite**.
- [ ] Use Vite only as the development/build tool; production output must be static assets served by MyDNS/Axum.
- [ ] Keep application state/API integration lightweight: React state/hooks for local UI state, native `fetch` for mutations, and a small API/query abstraction for server state. Avoid adding a large state-management framework unless real complexity requires it.
- [ ] Use native WebSocket handling for live DNS/log/status streams, with explicit reconnect/backoff and connection-state UI.
- [ ] Use a small icon library such as Lucide rather than shipping a large UI component framework.
- [ ] Keep the visual system in project-owned CSS/design tokens so the My Systems/MyDNS brand is not locked to a third-party component theme.
- [ ] Use semantic HTML, keyboard-accessible controls, visible focus states, reduced-motion support, and WCAG-conscious contrast.
- [ ] Add responsive layouts for desktop, tablet, and narrow/mobile management views.
- [ ] Build production assets with hashed filenames and a deterministic Node/npm or pnpm build step in CI.
- [ ] Decide and document the Rust-to-frontend asset handoff: preferred approach is a dedicated frontend source directory (for example `web/`) compiled to a static `dist/` directory consumed/served by Axum; do not make the Rust backend depend on Node at runtime.
- [ ] Add frontend lint/type-check/build checks to CI.
- [ ] Add browser-level smoke tests for authentication, navigation, CRUD record flows, protected routes, and WebSocket reconnect behavior.
- [ ] Ensure production frontend assets are reproducible and no development source maps/debug endpoints are exposed unintentionally.

#### Version-controlled UI design artifact

Figma MCP exhaustion must not block UI design or implementation. The repository will maintain a version-controlled visual specification/prototype that can later be transferred into Figma without redesigning the product.

- [ ] Create `docs/ui/README.md` as the authoritative UI/design specification.
- [ ] Create `docs/ui/design-system.css` containing reusable design tokens and component styling used by the prototype.
- [ ] Create `docs/ui/prototype.html` as a high-fidelity clickable dashboard prototype.
- [ ] Include prototype states for login, dashboard, DNS records, cache, live logs, settings, search/filtering, record-type badges, status indicators, modals, toasts, empty/loading/error states, destructive confirmations, and WebSocket disconnected/reconnecting states.
- [ ] Base terminology, navigation, data fields, and workflows on the actual MyDNS backend/API rather than inventing Figma-only functionality.
- [ ] Use the My Systems brand profile as the visual source of truth for brand treatment while keeping implementation tokens in-repository.
- [ ] Treat the prototype and design-system files as the visual specification for the eventual `src/assets`/frontend implementation.
- [ ] When Figma MCP availability returns, optionally reproduce the approved repository design in Figma; Figma is a secondary design surface, not a blocker or the sole source of truth.

#### Frontend implementation sequence

1. [ ] Audit the existing `src/assets` HTML/CSS/JS against the actual API and current production-readiness requirements.
2. [ ] Freeze the information architecture and core workflows in `docs/ui/prototype.html`.
3. [ ] Establish My Systems/MyDNS design tokens, typography, spacing, surfaces, controls, status semantics, and responsive rules in the repository design system.
4. [ ] Scaffold the React/TypeScript/Vite frontend without changing Rust/backend behavior.
5. [ ] Implement authentication and application shell/navigation.
6. [ ] Implement dashboard and live service/status views.
7. [ ] Implement DNS record CRUD with validation/error handling matching the backend API.
8. [ ] Implement cache and logs views, including live WebSocket behavior and reconnect states.
9. [ ] Implement settings and destructive/admin confirmation flows.
10. [ ] Add loading, empty, error, unauthorized, session-expired, and disconnected states across all views.
11. [ ] Run accessibility, responsive, browser smoke, lint, type-check, and production-build verification.
12. [ ] Replace the legacy `src/assets` implementation only after the new build is functionally and visually verified.

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
16. The production frontend is a reproducible static build served by MyDNS, with browser smoke coverage and no required Node runtime in production.
17. The approved UI design is represented in version-controlled repository documentation/prototype and matches the implemented frontend workflows.

## Immediate next steps

1. Pull the latest `production-readiness` branch.
2. Remove any leftover `test_*.db`, `test_dns_*.db`, and `*.db-journal` artifacts; never commit them.
3. Inspect the SQLite connection/pool initialization and persistent-cache upsert path.
4. Reproduce the lock failure with the **full** `cargo test` suite, not only the focused cache target.
5. Fix the underlying SQLite contention/transaction/pool lifecycle issue. Do not weaken the concurrency assertion merely to make the test pass.
6. Run `cargo fmt --check`.
7. Run `cargo check`.
8. Run `cargo clippy -- -D warnings`.
9. Run `cargo test --test cache_persistence -- --nocapture` repeatedly.
10. Run the complete `cargo test` suite repeatedly until the concurrency test is stable.
11. Verify the working tree is clean of generated runtime artifacts.
12. Commit and push the verified SQLite fix.
13. Only after the cache gate is genuinely stable, proceed to the next P0 item: **zone/ownership enforcement**.
14. Run `cargo audit` as the next security gate and record advisory dispositions in this document.
15. In parallel with backend hardening, begin the repository UI design artifact under `docs/ui/`; UI work must not be blocked by Figma MCP quota.
16. After the visual prototype is approved, scaffold the React/TypeScript/Vite frontend and plan the CI/build handoff into Axum's static asset serving.

## Handoff state

The current blocking issue for the next engineer/agent is **SQLite concurrent-write reliability**. The isolated persistence test can pass repeatedly, but the complete suite has demonstrated an intermittent `database is locked` failure. Treat the cache concurrency gate as open until the full suite is consistently green.

Recommended investigation order:

1. SQLite connection initialization and PRAGMA settings.
2. SQLx pool size, acquisition/release behavior, and test pool lifecycle.
3. Cache upsert transaction scope and duration.
4. SQLite busy timeout/WAL interaction and whether retries are required for transient contention.
5. Parallel integration-test database isolation and cleanup.
6. Repeat the complete suite enough times to establish stability.

## Execution order

1. **P0 correctness/security:** validation, ownership, privilege handling, shutdown, HTTPS, abuse controls.
2. **P1 DNS/cache:** complete DNS record-type behavior and upstream failure coverage; finish cache CI integration.
3. **P1 API/security:** authorization, error handling, headers, audit logging, resource limits.
4. **P1 CI/security:** audit dependencies, harden CI, enable branch protection.
5. **P2 UI foundation:** repository design system/prototype, then React/TypeScript/Vite frontend migration and browser smoke coverage.
6. **P2 deployment:** native service, container, HTTPS topology, backups.
7. **P2 release:** artifacts, checksums, supported targets, clean-machine verification, release notes.
8. **Final gate:** complete security review, clean-tree audit, reproducible deployment verification, frontend production-build verification, then tag the production-ready commit.
