# MyDNS v1.0.0 Production Readiness Plan

Branch: `production-readiness`

## Purpose

This document defines the **goals, requirements, verification work, and acceptance targets for MyDNS v1.0.0**.

It is the finite V1 release plan. Work required to meet these targets should be logged here as implementation progresses. The document is not an ongoing agile backlog.

MyDNS v1.0.0 is complete only when the requirements and acceptance gate in this document are satisfied. Post-V1 improvements, issues discovered after release, and feedback should be handled separately after v1.0.0.

## V1.0.0 release target

The target is a credible first production release that is:

- Functionally correct for the supported DNS and management workflows.
- Stable under expected concurrent load.
- Observable from both the terminal and management UI.
- Correctly synchronized between backend state and live UI state.
- Securely authenticated and configured.
- Reproducibly buildable and deployable.
- Documented sufficiently for operation and troubleshooting.

## Current baseline

The branch currently has working coverage for DNS UDP/TCP serving, supported record handling, positive/NODATA/NXDOMAIN/SERVFAIL behavior, CNAME chaining/loop protection, upstream failure handling, persistent cache behavior, validation, authentication, API integration, and lifecycle/security functionality.

The current local quality baseline is green for:

- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

`cargo audit` still reports `RUSTSEC-2023-0071` for `rsa 0.9.10`; this requires explicit final security disposition before release.

The remaining work below is the V1 release scope.

---

## 1. Requirements and work log

Use this section to record implementation progress against the requirements below. Keep entries factual and tied to the V1 target.

| ID | Area | Requirement | Status | Evidence / Notes |
|---|---|---|---|---|
| V1-001 | Repository | Correct test, stress-test, UI, and portfolio/demo directory layout. | TODO | |
| V1-002 | Repository | Remove generated/runtime/debug artifacts from the production repository. | TODO | |
| V1-003 | Stress | Establish deterministic bounded DNS/API/WebSocket stress smoke tests. | TODO | |
| V1-004 | Stress | Verify concurrency, cache pressure, upstream failures, shutdown, and restart behavior. | TODO | |
| V1-005 | Observability | Add structured terminal DNS request/response tracing. | TODO | |
| V1-006 | Observability | Include client IP/port, FQDN, type, transport, cache/resolution path, result, TTL, and latency. | TODO | |
| V1-007 | WebSocket | Stream useful DNS operational events to the dashboard. | TODO | |
| V1-008 | WebSocket | Verify reconnect, disconnect, backpressure, bounded history, and resource cleanup. | TODO | |
| V1-009 | Cache UI | Implement live TTL countdown without page reload. | TODO | |
| V1-010 | Cache UI | Reconcile countdown with authoritative backend refresh without stale responses moving state backwards. | TODO | |
| V1-011 | Dashboard | Keep uptime, cache, record, WebSocket, and log state synchronized. | TODO | |
| V1-012 | Dashboard | Replace silent frontend failures with explicit loading/error/disconnected handling. | TODO | |
| V1-013 | DNS | Complete allowed-zone ownership enforcement and normalization tests. | TODO | |
| V1-014 | API/Auth | Complete REST/WebSocket authentication, authorization, input, and error handling verification. | TODO | |
| V1-015 | Lifecycle | Verify configuration validation, startup failure, shutdown, and restart behavior. | TODO | |
| V1-016 | Security | Resolve or formally disposition all release-blocking dependency/security advisories. | TODO | |
| V1-017 | Frontend | Implement the agreed React + TypeScript + Vite production frontend. | TODO | |
| V1-018 | UI | Maintain the repository UI specification/prototype for V1 workflows and states. | TODO | |
| V1-019 | CI | Reproduce Rust, frontend, stress smoke, security, and cross-platform gates in CI. | TODO | |
| V1-020 | Release | Produce reproducible release artifacts, deployment procedures, and matching documentation. | TODO | |

Status values should remain simple: `TODO`, `IN PROGRESS`, `BLOCKED`, `DONE`.

---

## 2. Detailed V1 requirements

### 2.1 Repository, test, and portfolio layout — REQUIRED

The final repository must have an intentional separation between:

- Production Rust source under `src/`.
- Normal integration tests under `tests/`.
- Stress/load testing material separated from ordinary deterministic tests where appropriate.
- UI specification/prototype material under `docs/ui/`.
- Portfolio/demo material in its explicitly intended location.
- Documentation under `docs/`.

Do not commit generated databases, WAL/SHM/journal files, runtime logs, build output, screenshots, temporary metadata, or debug dumps as application artifacts.

### 2.2 Stress and reliability — REQUIRED

Verify the real server under concurrent/sustained activity, covering at minimum:

- High-rate concurrent DNS queries.
- Mixed query types and names.
- Concurrent cache hits/misses and population.
- Cache expiration/pruning under load.
- CNAME chains under concurrency.
- Upstream success, timeout, NXDOMAIN, and SERVFAIL.
- Concurrent authenticated HTTP CRUD operations.
- Repeated WebSocket connection/disconnection.
- Log broadcast pressure and slow consumers.
- Graceful shutdown during active DNS/API/WebSocket traffic.
- Startup/shutdown/restart cycles.
- Memory/task/socket/file-descriptor behavior.
- SQLite stability.

Provide a bounded stress smoke profile suitable for CI and a heavier release-verification profile.

### 2.3 Terminal DNS observability — REQUIRED

DNS activity must be understandable from the terminal without a debugger.

Each useful request flow should expose, where applicable:

- Client/source IP and source port.
- Requested FQDN.
- Query type.
- UDP/TCP transport.
- Normalized query name.
- Resolution path: memory cache, persistent cache, authoritative DB, special record, or upstream.
- Cache hit/miss.
- Upstream destination.
- CNAME hops.
- Final result: positive, NODATA, NXDOMAIN, or SERVFAIL.
- Answer count.
- Effective TTL.
- Latency.
- Timeout/error reason.

Example shape:

```text
DNS RX  client=192.168.1.20:53142 query=example.com. type=A
DNS    cache=MISS
DNS    source=UPSTREAM target=1.1.1.1:53
DNS    result=NOERROR answers=1 ttl=287 latency=18ms
DNS TX  client=192.168.1.20:53142 query=example.com. type=A
```

Do not log passwords, JWTs, authorization headers, or other secrets.

### 2.4 Live WebSocket logs — REQUIRED

The dashboard log stream must expose meaningful operational events, including DNS source, query/type, resolution path, result, latency, and relevant CNAME/upstream information.

Verify:

- Correct event timestamps.
- Bounded retained history.
- Safe behavior for slow consumers.
- Reconnect/disconnect state.
- No resource leaks.
- No secret leakage.
- Consistent event categories.

### 2.5 Cache UI correctness — REQUIRED

The cache UI must not display a stale `ttl_remaining` value while the page remains open.

Requirements:

- Visible TTL countdown updates every second.
- Expired entries disappear automatically or after authoritative refresh confirms expiration.
- Backend refresh remains authoritative.
- Stale API responses cannot move displayed state backwards.
- Cache hit/miss/stat values update while open.
- Clear/delete operations update immediately and reconcile with the backend.
- Loading, empty, error, and disconnected states are explicit.
- Concurrent refreshes cannot overwrite newer UI state.

The browser countdown is presentation only; server-side expiration remains authoritative.

### 2.6 Dashboard state synchronization — REQUIRED

Verify correct live behavior for:

- Uptime.
- Cache hits/misses and size.
- Record count.
- WebSocket connection state.
- Live logs.
- Navigation between sections.
- Logout/session expiry.
- API failures.

Navigation must not create duplicate timers or WebSocket connections. Logout must clean up active polling and WebSocket resources.

### 2.7 DNS correctness and ownership — REQUIRED

Complete and verify:

- `allowed_zones` configuration.
- Canonical name normalization.
- Exact-zone and subdomain matching.
- Case-insensitive/trailing-dot handling.
- Rejection of unrelated zones.
- Enforcement on create/update.
- Regression coverage for boundaries.

Retain protocol coverage for UDP/TCP, supported records, authoritative responses, TTLs, positive/NODATA/NXDOMAIN/SERVFAIL, CNAME chains, and loops.

### 2.8 API, authentication, and lifecycle — REQUIRED

Verify protected REST and WebSocket surfaces, JWT validation, authorization/zone boundaries, malformed/oversized input handling, consistent safe errors, audit behavior, failed-login logging, startup configuration validation, graceful shutdown, and restart behavior.

Security-sensitive startup failures must fail closed.

### 2.9 Dependency and security gate — REQUIRED

Before release:

- Run `cargo audit` against the final lockfile.
- Explicitly resolve or disposition `RUSTSEC-2023-0071`.
- Confirm the shipped JWT feature graph uses the intended AWS-LC-RS backend and does not use the vulnerable RSA implementation at runtime.
- Ensure required native build tools such as NASM are provisioned in CI.
- Review remaining dependency and CI security findings.

The advisory must not be dismissed solely because `cargo tree` does not show an active dependency path; the final lockfile state and reason must be documented.

### 2.10 Production frontend — REQUIRED

Implement the agreed architecture:

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

The production deployment must not require a Node runtime.

The frontend must provide explicit application state and lifecycle handling for authentication, records, cache, statistics, settings, logs, and WebSocket connectivity.

Required UI states include loading, empty, error, unauthorized, expired session, reconnecting, and disconnected.

### 2.11 UI specification/prototype — REQUIRED

The repository remains the authoritative UI specification. Figma is optional and must not block V1 implementation.

Maintain the agreed UI material under `docs/ui/` and cover the real V1 workflows and states: login, dashboard, DNS record CRUD, cache/live TTL, live logs, WebSocket failure/reconnect, settings, loading/empty/error states, and responsive layouts.

### 2.12 CI and release verification — REQUIRED

CI must cover:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy -- -D warnings`
- Complete `cargo test`
- Stress smoke profile.
- Linux build/test.
- Windows build/test.
- `cargo audit`.
- Release-profile build.
- Frontend install/type-check/lint/build.
- Browser smoke tests.

Native dependencies must be explicitly installed on runners.

### 2.13 Packaging, deployment, and documentation — REQUIRED

Before release, verify:

- Clean-checkout reproducible build.
- Versioned release archives.
- SHA-256 checksums.
- Minimal multi-stage container and non-root runtime.
- Healthcheck.
- Persistent data/config/log locations.
- Linux systemd deployment.
- Windows deployment procedure if supported.
- Backup/restore.
- Upgrade/rollback.
- HTTPS reverse-proxy deployment.
- Firewall/port documentation.
- Troubleshooting documentation.
- README/config/security/deployment docs match the shipped implementation.

---

## 3. V1.0.0 acceptance gate

**MyDNS v1.0.0 must not be tagged or published until every item below is satisfied.**

1. Formatting, check, clippy, and complete test suite pass.
2. Stress smoke testing passes with no unresolved stability defect.
3. DNS positive, NODATA, NXDOMAIN, and SERVFAIL behavior is correct.
4. UDP/TCP behavior is verified.
5. CNAME chains and loops are safe.
6. Upstream failures are deterministic.
7. Zone ownership/enforcement is implemented and tested.
8. Cache persistence, expiration, pruning, deduplication, invalidation, restart, clear, and concurrency are verified.
9. Cache UI TTL countdown and authoritative refresh are correct.
10. Dashboard state stays synchronized without duplicate timers/connections.
11. Terminal DNS logs expose source, query, type, resolution path, result, and latency.
12. WebSocket logs expose equivalent useful operational information safely.
13. WebSocket lifecycle/reconnect behavior is reliable.
14. Protected REST/WebSocket surfaces enforce authentication and authorization.
15. Configuration/startup/shutdown/restart behavior is tested.
16. Security/dependency advisories have final dispositions.
17. CI reproduces the required quality/security gates on supported platforms.
18. React/TypeScript/Vite production frontend is implemented and served as static assets by Axum.
19. Browser smoke tests cover critical management workflows.
20. Repository/test/portfolio/UI layout is intentional and clean.
21. Release artifacts and deployment procedures are reproducible.
22. Documentation matches the shipped implementation.
23. Final clean-tree review finds no secrets, debug artifacts, stale generated files, or accidental runtime data.

Only after all 23 conditions are satisfied should the `v1.0.0` tag be created.

---

## 4. Execution sequence

The V1 work should be completed in this order:

1. Repository/test/portfolio layout and hygiene.
2. Stress-test infrastructure and reliability verification.
3. Terminal DNS observability/logging.
4. WebSocket event and lifecycle correctness.
5. Cache UI live TTL/countdown and refresh correctness.
6. Dashboard synchronization and frontend error handling.
7. DNS ownership/correctness completion.
8. API/auth/configuration/lifecycle hardening.
9. Dependency/security disposition.
10. React/TypeScript/Vite production frontend.
11. Browser smoke, accessibility, and responsive verification.
12. CI/cross-platform/security gates.
13. Packaging/deployment/clean-machine verification.
14. Documentation and final repository audit.
15. Full v1.0.0 acceptance run.
16. Tag and publish v1.0.0.

This is the V1 completion sequence, not a recurring sprint cycle.

---

## 5. Post-V1 boundary

After v1.0.0 is released, this document is considered complete.

New improvements, performance tuning, UX changes, feature requests, deployment feedback, and issues discovered in real use should be evaluated separately against the released V1 rather than appended indefinitely to this plan.
