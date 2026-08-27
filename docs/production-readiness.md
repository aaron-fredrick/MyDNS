# MyDNS Production Readiness Plan

Branch: `production-readiness`

## 1. Purpose

This is the implementation and release plan for **MyDNS v1**.

The current branch is **not V1-ready yet**. The core DNS/API implementation is in good shape, but the release still requires a final correctness, stress/reliability, observability, UI, security, packaging, and deployment pass.

This is deliberately a **finite V1 completion plan**, not an agile backlog. We will finish the items required to make the first release genuinely usable and supportable, release V1, and only then consider later improvements based on real deployment experience, reported issues, and feedback.

Do not expand this plan with speculative features. If something is not necessary for a credible V1, defer it.

---

## 2. Current branch state

The branch currently has working coverage for:

- DNS UDP and TCP serving.
- A/AAAA/CNAME/MX/NS/TXT/PTR handling currently covered by the test suite.
- Positive answers, NODATA, NXDOMAIN, and SERVFAIL behavior.
- Authoritative CNAME chaining and loop protection.
- Upstream timeout, NXDOMAIN, and SERVFAIL handling.
- Persistent positive and negative cache storage.
- Cache expiration, pruning, persistence, deduplication, and invalidation.
- Record validation and effective-state validation.
- Authentication and protected-route coverage currently present.
- Security headers, audit logging, request/resource controls, graceful shutdown, and Unix privilege work already present.
- React migration has **not yet been completed**; the current management UI remains the existing HTML/CSS/JavaScript application under `src/assets`.
- WebSocket log streaming exists, but its observability and lifecycle behavior still need production-level verification.

### Latest local verification

The latest local run is currently green for:

- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

The complete suite includes library, authentication, cache persistence, DNS integration, HTTP/API integration, upstream integration, and validation API tests.

`cargo audit` still reports `RUSTSEC-2023-0071` for `rsa 0.9.10`. The current Cargo feature/dependency inspection indicates that MyDNS selects `jsonwebtoken`'s AWS-LC-RS backend and does not expose `rsa` through the active dependency tree, but the lockfile still contains the package. This remains a release-security disposition item and must be explicitly closed before V1.

---

## 3. V1 completion work

The following is the remaining V1 scope. It is ordered by dependency and risk, not by sprint.

### 3.1 Stress and reliability testing — REQUIRED

The existing functional tests prove correctness for individual scenarios; they do **not** yet establish that the server remains stable under sustained/concurrent load.

Add a dedicated stress-test area and run it against the real server processes where practical.

Cover at minimum:

- High-rate concurrent DNS queries.
- Mixed query types and names.
- Repeated cache hits/misses under concurrency.
- Concurrent cache population and expiration.
- CNAME chains under concurrency.
- Upstream success, timeout, NXDOMAIN, and SERVFAIL under load.
- Concurrent HTTP record CRUD operations.
- Concurrent authenticated API requests.
- Repeated WebSocket connect/disconnect cycles.
- Log broadcast pressure and slow WebSocket consumers.
- Graceful shutdown while DNS/API/WebSocket traffic is active.
- Startup/shutdown/restart cycles.
- Long-running cache expiration/pruning behavior.
- Resource behavior: memory growth, task growth, file descriptors/sockets, and SQLite stability.

The stress tests must be deterministic enough to run in CI for a bounded smoke profile, with a heavier profile available for release verification.

Do not treat a single successful stress run as sufficient. Investigate failures and rerun after fixes.

---

### 3.2 Test/portfolio directory correction and repository hygiene — REQUIRED

Review the repository layout before V1 and correct the test/support directory structure so production code, integration tests, stress tests, UI artifacts, and portfolio/demo material are clearly separated.

The final repository must have an intentional layout with:

- Production Rust source under `src/`.
- Normal integration tests under `tests/`.
- Stress/load tests separated from ordinary deterministic integration tests where appropriate.
- UI specification/prototype material under `docs/ui/`.
- Portfolio/demo material in its explicitly intended location rather than being mixed into production/test paths.
- No generated databases, WAL/SHM/journal files, logs, build output, screenshots, temporary metadata, or debug dumps committed as application artifacts.

Correct any existing naming/location inconsistency found during this review and document the intended structure.

---

### 3.3 DNS observability and terminal logging — REQUIRED

The server needs useful operational visibility when run directly from a terminal. Existing tracing is present, but V1 should make DNS behavior understandable without attaching a debugger.

Implement structured, readable DNS request tracing covering:

- Client/source IP and source port.
- Requested FQDN.
- Query type.
- Transport/protocol where available (UDP/TCP).
- Normalized query name.
- Resolution path: memory cache, persistent cache, authoritative DB, special record, or upstream.
- Cache hit/miss.
- Upstream target when an upstream lookup occurs.
- CNAME hops where relevant.
- Final response classification: positive, NODATA, NXDOMAIN, or SERVFAIL.
- Response/answer count.
- Effective TTL where relevant.
- Resolution duration/latency.
- Errors and timeout causes.

Example operational flow should be easy to follow:

```text
DNS RX  client=192.168.1.20:53142  query=example.com. type=A
DNS    cache=MISS
DNS    source=UPSTREAM target=1.1.1.1:53
DNS    result=NOERROR answers=1 ttl=287 latency=18ms
DNS TX  client=192.168.1.20:53142  query=example.com. type=A
```

The exact formatting may differ, but the information must be present and consistent.

Avoid logging passwords, JWTs, authorization headers, or other secrets.

---

### 3.4 Live WebSocket logs — REQUIRED

The WebSocket stream should expose the same meaningful operational events as terminal logging, in a compact UI-safe form.

Verify and improve:

- Consistent event categories.
- Timestamp generated from the actual event time rather than arbitrary UI receipt time where possible.
- Client IP/source visibility for DNS events.
- Query name/type visibility.
- Cache path and upstream path.
- Final response code/status.
- Latency.
- CNAME traversal information where useful.
- Authentication/admin events without secrets.
- Clear reconnect/disconnect status.
- Correct behavior when the broadcast channel lags or closes.
- Bounded retained log history.
- No unbounded memory growth from clients.

The browser log representation must be readable, filterable, and safe to display.

---

### 3.5 Cache UI correctness — REQUIRED

The current cache page only refreshes when the page/section is loaded. Its `ttl_remaining` value therefore becomes stale while the page remains open.

Fix the cache UI so that:

- TTL countdown updates visibly every second without requiring a page reload.
- Entries disappear automatically when their TTL reaches zero or after the next authoritative refresh confirms expiration.
- The UI periodically refreshes authoritative cache state from the API.
- Refreshes do not reset the visible countdown backwards due to stale responses.
- Cache hit/miss/stat values update while the dashboard is open.
- Cache clear/delete operations update the table immediately and reconcile with the server.
- Loading, empty, error, and disconnected states are explicit.
- Concurrent refreshes cannot overwrite newer UI state with an older response.

The browser countdown is presentation only; the server remains authoritative for expiration.

Add regression coverage for this behavior at the frontend level.

---

### 3.6 Dashboard/live status correctness — REQUIRED

Audit the dashboard for stale or misleading state.

Verify:

- Uptime updates correctly.
- Cache hit/miss counters update correctly.
- Cache size updates correctly.
- Record count updates after CRUD operations.
- WebSocket status accurately reflects connected/reconnecting/disconnected states.
- Logs continue updating after navigation.
- Leaving/re-entering a section does not create duplicate polling timers or WebSocket connections.
- Logout cancels all timers and closes the WebSocket.
- Expired authentication returns the user to login rather than silently failing API calls.
- Failed API calls display actionable errors rather than being swallowed.

The current frontend contains several empty `catch` paths; V1 should replace silent failures with visible, appropriately scoped error handling.

---

### 3.7 Frontend architecture and production UI — REQUIRED

The existing UI is functional but is still the legacy static HTML/JavaScript implementation. V1 should move to the agreed production frontend architecture:

```text
React + TypeScript
        |
       Vite
        |
   static dist/
        |
        v
      Axum
        |
   +----+----+
   |         |
 REST API  WebSocket
```

Requirements:

- React + TypeScript + Vite production build.
- Static assets served by Axum; no Node runtime in production.
- Preserve existing backend API contracts unless a deliberate V1 correction is required.
- Explicit application state for auth, records, cache, stats, settings, logs, and WebSocket state.
- Proper polling/subscription lifecycle management.
- WebSocket reconnect with bounded backoff.
- Accessible semantic controls and keyboard navigation.
- Responsive desktop/tablet/narrow layouts.
- Loading, empty, error, unauthorized, expired-session, reconnecting, and disconnected states.
- Search/filtering where already expected by the UI design.
- Confirmation dialogs and clear success/error feedback.
- Production build with deterministic hashed assets.
- Frontend lint/type-check/build verification in CI.

Do not introduce a large frontend framework/component dependency unless it solves a concrete V1 requirement.

---

### 3.8 UI specification / prototype — REQUIRED

The repository remains the authoritative UI specification. Figma is optional and must not block implementation.

Maintain:

```text
docs/ui/
├── README.md
├── design-system.css
└── prototype.html
```

The prototype must cover the actual V1 workflows and states, including:

- Login/authentication failure.
- Dashboard.
- DNS record list/create/edit/delete.
- Cache list with live TTL behavior.
- Live logs.
- WebSocket disconnected/reconnecting state.
- Settings.
- Empty/loading/error states.
- Responsive layouts.

---

### 3.9 DNS correctness and ownership — REQUIRED

Complete the remaining correctness boundary work:

- `allowed_zones` configuration.
- Canonical name normalization.
- Exact-zone and subdomain matching.
- Case-insensitive and trailing-dot handling.
- Rejection of unrelated zones.
- Enforcement on create and update.
- Regression tests for all boundary cases.

Add/retain explicit protocol tests for:

- Query-name normalization.
- Record-type separation.
- Authoritative response flags.
- Authority-section behavior.
- TTL propagation.
- UDP and TCP behavior.
- Positive, NODATA, NXDOMAIN, and SERVFAIL responses.
- CNAME chains and loops.

---

### 3.10 API/auth/WebSocket hardening — REQUIRED

Verify:

- Every protected REST endpoint rejects unauthenticated access.
- Authenticated operations respect the intended resource/zone boundary.
- WebSocket authentication is enforced independently.
- Expired/tampered JWTs are rejected.
- Malformed JSON and oversized requests are rejected cleanly.
- Error payloads are consistent and do not leak internal details.
- Destructive/admin operations are audited.
- Failed login attempts are logged without credentials.
- WebSocket reconnect/disconnect cycles do not leak resources.

---

### 3.11 Configuration and lifecycle — REQUIRED

Add/verify tests for:

- Missing required configuration.
- Malformed values.
- Invalid addresses/ports.
- Invalid TTLs.
- Invalid auth configuration.
- `allowed_zones` normalization.
- Startup failure behavior.
- SIGINT/SIGTERM/Ctrl+C behavior as applicable.
- Coordinated DNS/HTTP/WebSocket shutdown.
- Restart behavior.

Security-sensitive startup failures must fail closed.

---

### 3.12 Dependency/security gate — REQUIRED

Before release:

- Re-run `cargo audit` against the final lockfile.
- Resolve or formally disposition `RUSTSEC-2023-0071`.
- Confirm the shipped feature graph does not use the vulnerable RSA implementation.
- Keep the AWS-LC-RS JWT backend intentional if it remains the chosen design.
- Review all other advisories and CI security findings.
- Ensure native build prerequisites such as NASM are explicitly provisioned in CI where AWS-LC requires them.
- Review GitHub Actions dependencies.
- Decide whether SBOM/signing/provenance is required for V1.

The RSA advisory cannot simply be ignored because `cargo tree` does not show a reachable path; the final lockfile and dependency-resolution explanation must be recorded.

---

### 3.13 CI and release verification — REQUIRED

CI must verify at minimum:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy -- -D warnings`
- Complete `cargo test`
- Stress smoke profile.
- Linux build/test.
- Windows build/test.
- `cargo audit`.
- Release-profile build.
- CodeQL/security analysis where configured.
- Frontend install, type-check, lint, build, and browser smoke tests.

Native dependencies required by the selected Rust crates must be installed explicitly rather than assumed to exist on the runner.

---

### 3.14 Packaging, deployment, and documentation — REQUIRED

Before release:

- Reproducible release build from a clean checkout.
- Versioned archives.
- SHA-256 checksums.
- Container image with a minimal multi-stage build and non-root runtime.
- Container healthcheck.
- Documented persistent data/config/log locations.
- Linux systemd deployment.
- Windows deployment procedure if Windows remains a supported production target.
- Backup/restore.
- Upgrade/rollback.
- HTTPS reverse-proxy deployment.
- Firewall and port documentation.
- Operational troubleshooting.
- README/config/security/deployment docs matching the shipped implementation.

---

## 4. V1 acceptance gate

MyDNS is **not V1-ready** until all of the following are true:

1. `cargo fmt --check` passes.
2. `cargo check` passes.
3. `cargo clippy -- -D warnings` passes.
4. The complete test suite passes reliably.
5. Stress smoke tests pass and no unresolved stability issue remains.
6. DNS positive/NODATA/NXDOMAIN/SERVFAIL behavior is correct.
7. UDP and TCP behavior is verified for supported records.
8. CNAME chains and loops are safe.
9. Upstream failures are deterministic.
10. Zone/ownership enforcement is implemented and tested.
11. Cache persistence, expiration, deduplication, invalidation, restart, clearing, and concurrency are verified.
12. Cache UI TTL/countdown and refresh behavior is correct.
13. Dashboard statistics and live status remain synchronized without duplicate timers/connections.
14. Terminal DNS logging provides source IP, query, type, resolution path, result, and latency.
15. WebSocket logs provide equivalent useful operational visibility without leaking secrets.
16. WebSocket lifecycle and reconnect behavior are reliable.
17. Protected REST and WebSocket surfaces enforce authentication/authorization.
18. Configuration/startup/shutdown behavior is tested.
19. Dependency/security advisories have explicit final dispositions.
20. CI reproduces the required quality/security gates on supported platforms.
21. Production frontend is implemented as React/TypeScript/Vite static assets served by Axum.
22. Browser smoke tests cover the critical management workflows.
23. Repository/test/portfolio/UI directories are intentional and free of generated artifacts.
24. Native and container deployment procedures are reproducible.
25. Release artifacts are versioned, checksummed, and verified.
26. Documentation matches the actual implementation.
27. Final clean-tree review finds no secrets, debug artifacts, stale generated files, or accidental runtime data.

Only after all 27 conditions are satisfied should the V1 tag be created.

---

## 5. Execution order

Complete the work in this order:

1. **Repository/test/portfolio layout correction and hygiene.**
2. **Stress-test infrastructure and reliability verification.**
3. **Terminal DNS observability and structured logging.**
4. **WebSocket event/log reliability and lifecycle fixes.**
5. **Cache UI live TTL/countdown and refresh correctness.**
6. **Dashboard state synchronization and error handling.**
7. **Zone/ownership enforcement and remaining DNS correctness.**
8. **API/auth/configuration/lifecycle hardening.**
9. **Final dependency/security disposition.**
10. **React/TypeScript/Vite production frontend implementation and UI integration.**
11. **Browser smoke/accessibility/responsive verification.**
12. **CI cross-platform and security gates.**
13. **Packaging, deployment, and clean-machine verification.**
14. **Documentation and final repository audit.**
15. **Full V1 acceptance run.**
16. **Tag and publish V1.**

This sequence is intentionally finite. Do not turn it into an endless list of enhancements.

---

## 6. Post-V1 boundary

Once the acceptance gate passes and V1 is released, this document is complete.

Later performance work, UX refinement, additional features, issues discovered in deployment, and feedback-driven improvements belong to a separate post-V1 plan. They must not silently expand the V1 release scope.
