# MyDNS Production Readiness Plan

Branch: `production-readiness`

## 1. Purpose

This document is the release plan for the first production version of MyDNS.

The objective is to take the current feature-complete development server through one deliberate production-readiness pass and ship a defensible **v1**. The scope is intentionally finite: correctness, security, reliability, testing, the production UI, packaging, deployment, and documentation required for the first release.

This is **not an ongoing agile backlog**. Once the v1 acceptance criteria are satisfied and the release is tagged, this document is complete. Future improvements, performance work, feature requests, and issues raised from real-world use can be evaluated separately after v1.

---

## 2. Current verified state

The current `production-readiness` branch has already closed the following major areas:

- DNS UDP and TCP serving.
- Configured DNS bind-host handling.
- Positive answers, NODATA, NXDOMAIN, and SERVFAIL differentiation.
- Authoritative CNAME chasing with bounded loop protection.
- Wire-level DNS coverage for the supported record types.
- Upstream timeout, NXDOMAIN, and SERVFAIL handling.
- Persistent cache storage and restart persistence.
- Positive and negative cache entries.
- TTL expiration and pruning.
- Persistent cache deduplication.
- CNAME-dependent cache invalidation.
- SQLite concurrent-write reliability for the current test model.
- Record validation for names, supported types, values, TTLs, and MX priority.
- Effective-state validation for record updates.
- Authentication and protected-route coverage currently present in the integration suite.
- Security headers, audit logging, and bounded request/resource controls.
- Unix privilege-handling work and graceful shutdown work already present on the branch.
- Documented HTTPS deployment through a TLS-terminating reverse proxy.
- JWT configuration using `jsonwebtoken`'s AWS-LC-RS backend rather than its RustCrypto RSA backend.

### Latest local verification

The latest verification run completed successfully for:

- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

The complete test suite currently passes, including:

- 29 library tests.
- 1 authentication coverage test.
- 5 persistent-cache tests.
- 8 DNS integration tests.
- 7 HTTP/API integration tests.
- 3 upstream integration tests.
- 2 validation API tests.

`cargo audit` still reports `RUSTSEC-2023-0071` for `rsa 0.9.10`. Current dependency-tree checks do not show `rsa` as a reachable dependency of the selected MyDNS feature graph, while the lockfile still contains the package entry. This must be explicitly reviewed and documented before the v1 release gate is closed; it must not be silently ignored.

---

## 3. Remaining work for v1

The remaining work is grouped by release concern rather than by sprint or agile iteration. Work should be completed in the order below because later release tasks depend on earlier correctness and security gates.

### 3.1 DNS correctness and ownership

#### Zone / ownership enforcement — next implementation item

The management API must not allow an authenticated administrator or future user boundary to create or modify records outside the configured DNS ownership boundary.

Implement:

- `allowed_zones` configuration.
- Canonical DNS-name normalization before ownership checks.
- Exact-zone matching.
- Subdomain matching.
- Case-insensitive handling.
- Trailing-dot handling.
- Rejection of unrelated zones.
- Enforcement on both record creation and update.
- Tests covering exact matches, subdomains, unrelated names, case differences, and trailing dots.

This must be enforced at the API validation boundary, not only by the UI.

#### DNS protocol edge cases

Add explicit tests for:

- Query-name case normalization.
- Trailing-dot normalization.
- Record-type separation.
- Authoritative response flags.
- Authority-section behavior.
- TTL propagation.

The existing record-type and upstream integration coverage should remain intact while these cases are added.

---

### 3.2 Web/API hardening

Complete the remaining API behavior checks:

- Standardize HTTP status codes and structured error payloads.
- Ensure errors do not expose internal implementation details, filesystem paths, credentials, tokens, or database information.
- Verify every protected REST route rejects unauthenticated requests.
- Verify every authenticated operation enforces the intended authorization boundary.
- Verify WebSocket authentication independently from REST authentication.
- Verify session-expiration behavior.
- Verify destructive operations require the intended authorization.

Add tests for:

- Malformed JSON.
- Oversized requests.
- Invalid authentication credentials.
- Expired/tampered JWTs.
- Unauthorized resource access.
- Invalid record mutations.
- WebSocket authentication and rejection.

---

### 3.3 Configuration and lifecycle verification

Add automated coverage for startup configuration and lifecycle behavior:

- Missing required configuration.
- Malformed configuration values.
- Invalid bind addresses.
- Invalid ports.
- Invalid TTL/configuration ranges.
- Missing or invalid authentication configuration.
- `allowed_zones` parsing and normalization.
- Startup failure behavior.
- Graceful shutdown.
- OS termination signal handling.
- DNS and HTTP shutdown coordination.

The service must fail closed when required security or privilege configuration is invalid.

---

### 3.4 WebSocket reliability

The live management channel must be production-safe before the UI depends on it.

Verify:

- Authenticated connection succeeds.
- Unauthenticated connection is rejected.
- Client disconnect is cleaned up correctly.
- Server shutdown closes connections cleanly.
- Slow/lagging clients cannot consume unbounded resources.
- Reconnection works after a dropped connection.
- Repeated connect/disconnect cycles do not leak resources.

---

### 3.5 Dependency and supply-chain security

The v1 release must have an explicit security disposition for all dependency findings.

Required actions:

- Re-run `cargo audit` against the final lockfile.
- Investigate the reported `rsa 0.9.10` / `RUSTSEC-2023-0071` entry.
- Confirm the actual selected dependency graph using Cargo's normal and feature graphs.
- If the package remains only as an unused/optional lockfile resolution, document that fact and why the vulnerable implementation is not reachable by MyDNS.
- If it becomes reachable, remove or replace the dependency before release where technically possible.
- Review Dependabot/security findings and either upgrade or document a justified exception.
- Keep direct dependency requirements intentional.
- Pin or otherwise verify GitHub Actions dependencies used for release/security workflows.
- Decide whether an SBOM is required for the v1 release and implement it if required.

A known advisory may be accepted only with a documented technical justification and confirmation that the vulnerable code path is not part of the shipped application.

---

### 3.6 CI release gates

The repository must enforce the same basic checks used for local v1 verification.

CI must run:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy -- -D warnings`
- Complete `cargo test`
- Linux verification.
- Windows verification.
- Dependency/security auditing.
- Release-profile build verification.
- CodeQL/security analysis where configured.

Required checks must gate merges to `main` before the production tag is created.

The CI configuration must not rely on a developer machine having NASM or another undeclared native build prerequisite. Native build dependencies required by crates such as AWS-LC must be installed or provisioned explicitly by the relevant CI job.

---

## 4. Production frontend

The v1 dashboard will be a real production web application, not a second server runtime.

### Architecture

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
   |         |
   +----+----+
        |
      MyDNS
```

Development uses Node tooling only for frontend development and builds. Production runs the compiled static frontend from the Rust/Axum service and does not require Node at runtime.

### UI implementation

- Migrate the existing `src/assets` dashboard to React + TypeScript + Vite.
- Keep API behavior aligned with the actual Rust backend.
- Use native `fetch` for HTTP operations and native WebSocket handling for live streams.
- Implement explicit WebSocket reconnect/backoff and connection-state UI.
- Keep application state deliberately lightweight.
- Use project-owned design tokens/CSS rather than locking the application to a large component framework.
- Use semantic HTML and accessible controls.
- Provide visible focus states and sensible keyboard navigation.
- Respect reduced-motion preferences.
- Provide responsive desktop, tablet, and narrow-screen management layouts.
- Implement authentication, dashboard, DNS records, cache, live logs/status, settings, search/filtering, validation, confirmation dialogs, toasts, and empty/loading/error states.
- Ensure unauthorized, expired-session, disconnected, and reconnecting states are represented in the UI.
- Add frontend linting, type-checking, and production-build verification to CI.
- Add browser smoke tests covering authentication, navigation, CRUD, protected routes, and WebSocket reconnect behavior.
- Build deterministic production assets with hashed filenames.
- Do not expose development-only endpoints or debugging behavior in the production build.

### Repository-owned UI specification

Figma is not a release dependency. The repository remains the authoritative source for the v1 visual specification.

Create and maintain:

```text
 docs/ui/
 ├── README.md
 ├── design-system.css
 └── prototype.html
```

The prototype must represent the approved v1 workflows and states and must use the actual MyDNS terminology and data model.

Figma may be used later as a secondary design surface, but Figma availability or MCP quota must not block v1 implementation.

---

## 5. Release engineering

Before tagging v1:

- Define the supported release targets.
- Produce versioned release archives.
- Include required configuration examples and documentation.
- Generate SHA-256 checksums.
- Publish from version tags.
- Verify artifacts on clean machines/containers.
- Generate release notes/changelog information.
- Confirm version consistency across the application, package, and release artifacts.
- Decide and document SBOM/signing/provenance requirements.

The release process must be reproducible from a clean checkout.

---

## 6. Deployment

### Container deployment

Provide a production container that:

- Uses a minimal multi-stage build.
- Runs without unnecessary root privileges.
- Does not require unrestricted `--privileged` mode.
- Keeps SQLite/configuration persistent outside the image.
- Provides a healthcheck.
- Documents required DNS networking/capabilities.
- Is scanned for known vulnerabilities.

### Native deployment

Provide at least one documented native deployment path, including:

- Linux systemd service.
- Least-privilege execution.
- Restart policy.
- Writable paths.
- DNS UDP/TCP ports.
- Management HTTP/HTTPS ports.
- Firewall requirements.
- Database backup/restore.
- Log locations and rotation.
- Operational health/readiness behavior.

If Windows remains a supported production target, provide the equivalent Windows service/install procedure.

---

## 7. Documentation

The v1 documentation must match the shipped implementation.

Required documentation:

- README quick start.
- Complete configuration reference.
- Supported DNS record types and behavior.
- Authentication/security model.
- Production topology.
- HTTPS deployment.
- Native deployment.
- Container deployment.
- Ports and firewall requirements.
- Database and log locations.
- Backup/restore procedure.
- Upgrade and rollback procedure.
- Release process.
- Operational troubleshooting.

Remove stale setup instructions and ensure examples use the current configuration format.

---

## 8. V1 acceptance gate

MyDNS v1 is ready to release only when all of the following are true:

1. `cargo fmt --check` passes.
2. `cargo check` passes.
3. `cargo clippy -- -D warnings` passes.
4. The complete test suite passes reliably.
5. DNS correctly distinguishes positive answers, NODATA, NXDOMAIN, and SERVFAIL.
6. Supported DNS record types work correctly over UDP and TCP.
7. CNAME chains are bounded and loop-safe.
8. Upstream timeout and failure behavior is deterministic.
9. Zone/ownership enforcement is implemented and tested.
10. Cache persistence, expiration, deduplication, invalidation, restart behavior, clearing, and concurrency are verified.
11. Protected REST and WebSocket surfaces enforce authentication and authorization as intended.
12. Request/resource limits and abuse controls are active.
13. Required security headers and audit logging are active without leaking credentials or tokens.
14. Startup configuration validation and shutdown behavior are tested.
15. Dependency advisories have documented dispositions and no unexplained release blocker remains.
16. CI runs the required quality/security gates on the supported platforms.
17. The frontend is a reproducible React/TypeScript/Vite static build served by Axum.
18. Browser smoke tests cover the critical management workflows.
19. At least one native deployment and one container deployment are documented and reproducible.
20. Release artifacts are versioned, checksummed, and verified.
21. README, configuration, security, deployment, and release documentation match the implementation.
22. A clean test run leaves no generated database, journal, log, or runtime artifacts in the repository working tree.
23. The final production checkout is reviewed for accidental secrets, debug behavior, stale files, and untracked generated artifacts.

---

## 9. Execution order

Complete the v1 work in this order:

1. **Zone/ownership enforcement.**
2. **DNS normalization, response flags, authority behavior, and TTL tests.**
3. **Remaining API authorization/error-contract tests.**
4. **Configuration/startup/shutdown tests.**
5. **WebSocket authentication and reliability tests.**
6. **Final dependency/security review, including the RSA advisory disposition.**
7. **CI quality/security gates and cross-platform verification.**
8. **Repository-owned UI specification and prototype.**
9. **React/TypeScript/Vite frontend implementation.**
10. **Browser smoke/accessibility/responsive verification.**
11. **Release packaging and clean-machine verification.**
12. **Native and container deployment verification.**
13. **Final documentation audit and clean-tree audit.**
14. **Tag and publish MyDNS v1.**

No additional feature expansion is part of this sequence. If a new issue is discovered that is not required to satisfy the v1 acceptance gate, record it separately for post-v1 evaluation rather than expanding the release scope.

---

## 10. Post-v1 boundary

Once the acceptance gate is satisfied, this plan is considered complete.

Future work should be driven by real deployment experience, user feedback, observed operational issues, security findings, performance measurements, and a fresh technical review. Those improvements should be planned independently rather than continuously extending the v1 release checklist.
