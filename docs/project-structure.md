# MyDNS Repository Structure

## Purpose

This document defines the intended repository structure for MyDNS v1.0.0.

The goal is a clean separation of concerns without introducing unnecessary abstraction. Rust follows normal Cargo/module conventions; the frontend is a separate React + TypeScript + Vite Node project; tests are separated by purpose; operational documentation is separated from portfolio material.

This structure is part of the V1 release target and should be implemented before the final release audit.

---

## Target repository layout

```text
MyDNS/
├── .cargo/
│   └── config.toml
├── .github/
│   ├── ISSUE_TEMPLATE/
│   └── workflows/
│       ├── codeql.yml
│       ├── test.yml
│       └── release.yml
│
├── docs/
│   ├── production-readiness-v1.0.0.md
│   ├── project-structure.md
│   ├── architecture.md
│   ├── configuration.md
│   ├── security.md
│   ├── deployment.md
│   ├── https-deployment.md
│   ├── operations.md
│   ├── troubleshooting.md
│   └── ui/
│       ├── README.md
│       ├── architecture.md
│       ├── screens/
│       └── states/
│
├── frontend/
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   ├── public/
│   └── src/
│       ├── main.tsx
│       ├── app/
│       ├── components/
│       ├── features/
│       │   ├── auth/
│       │   ├── dashboard/
│       │   ├── records/
│       │   ├── cache/
│       │   ├── logs/
│       │   └── settings/
│       ├── hooks/
│       ├── lib/
│       ├── services/
│       ├── state/
│       ├── types/
│       └── styles/
│
├── portfolio/
│   ├── README.md
│   ├── case-study.md
│   └── media/
│       ├── screenshots/
│       └── diagrams/
│
├── scripts/
│   ├── dns-smoke-test.ps1
│   ├── stress-smoke.*
│   ├── build-release.*
│   └── deployment/              # only where deployment helpers are needed
│
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── config.rs
│   ├── state.rs
│   ├── privileges.rs
│   ├── cache/
│   │   ├── mod.rs
│   │   └── tests.rs
│   ├── db/
│   │   ├── mod.rs
│   │   └── records.rs
│   ├── dns/
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   ├── server.rs
│   │   └── upstream.rs
│   └── web/
│       ├── mod.rs
│       ├── server.rs
│       ├── auth.rs
│       ├── validation.rs
│       ├── error.rs
│       ├── records_api.rs
│       ├── cache_api.rs
│       ├── stats_api.rs
│       ├── settings_api.rs
│       ├── dashboard.rs
│       └── ws.rs
│
├── stress-tests/
│   ├── README.md
│   ├── dns/
│   ├── api/
│   └── websocket/
│
├── tests/
│   ├── auth_coverage.rs
│   ├── cache_persistence.rs
│   ├── dns_integration.rs
│   ├── integration.rs
│   ├── upstream_integration.rs
│   └── validation_api.rs
│
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
└── config.ini.example
```

The exact workflow filenames and helper-script names can vary as implementation settles; the separation of responsibilities should not.

---

## 1. Rust backend

`src/` is the production application crate. It should follow Cargo's conventional module layout and avoid artificial layers such as `controllers/`, `services/`, `repositories/`, and `models/` unless a real architectural boundary emerges that justifies them.

### `main.rs`

Process entry point only:

- Load and validate configuration.
- Construct shared state and dependencies.
- Bind DNS/HTTP listeners.
- Coordinate startup and shutdown.
- Handle OS lifecycle signals.

Business logic should not accumulate here.

### `lib.rs`

Expose the reusable application modules needed by tests and binaries. Keep this as the module boundary, not as a dumping ground for implementation.

### `config.rs`

Own configuration parsing, defaults, validation, and configuration-related types. It should not start servers or access HTTP/DNS handlers.

### `state.rs`

Own shared application state and handles passed between subsystems. Keep state composition here rather than creating a global singleton registry.

### `privileges.rs`

Own platform-specific privilege handling. Unix-specific code stays behind this boundary rather than leaking throughout the application.

### `cache/`

Own cache semantics: lookup, insert/update, expiration, pruning, invalidation, and cache statistics. Persistence belongs in `db/`.

### `db/`

Own SQLite access and persistence. Database code must not know about Axum request/response types or frontend concerns.

### `dns/`

Own the DNS protocol and resolution path:

- `handler.rs` — query processing and DNS response construction.
- `server.rs` — UDP/TCP listener lifecycle and DNS server integration.
- `upstream.rs` — upstream resolution and upstream failure handling.

### `web/`

Own the HTTP/WebSocket transport boundary:

- `server.rs` — router, middleware, static-file serving, and web server lifecycle.
- `auth.rs` — authentication/session/JWT operations.
- `validation.rs` — API/domain input validation and zone ownership rules.
- `error.rs` — safe HTTP error representation.
- `*_api.rs` — resource-specific HTTP handlers.
- `ws.rs` — WebSocket connection and event transport.
- `dashboard.rs` — dashboard/static frontend integration during the transition.

The web layer may call application/domain functionality, but lower layers must not depend on Axum.

---

## 2. Frontend

The production UI is a standalone Node project under `frontend/`.

### Technology

- React.
- TypeScript.
- Vite.
- A conventional package manager lockfile; use npm unless the project deliberately standardizes on another manager.

Node is a **build-time** dependency. The production MyDNS server should serve the generated static frontend and must not require Node.js at runtime.

### Organization

Use feature-oriented modules rather than a monolithic `App.tsx` or a directory per HTTP endpoint.

- `components/` — reusable presentational UI primitives.
- `features/` — domain/application areas such as records, cache, logs, and settings.
- `services/` — REST and WebSocket transport clients.
- `state/` — shared application state where it genuinely crosses feature boundaries.
- `hooks/` — reusable React lifecycle/state hooks.
- `types/` — shared frontend/domain API types.
- `lib/` — small generic utilities.
- `styles/` — application-wide styling and design tokens.

The cache TTL countdown belongs to cache feature state/presentation, while authoritative expiration remains a backend concern.

The frontend must explicitly model loading, empty, error, unauthorized, expired-session, reconnecting, and disconnected states.

---

## 3. Tests

### `tests/`

Cargo integration tests are deterministic black-box/regression tests. They remain part of the normal `cargo test` suite.

Current responsibilities:

- `dns_integration.rs` — wire-level DNS behavior.
- `upstream_integration.rs` — upstream success/failure behavior.
- `cache_persistence.rs` — persistence, restart, expiration, deduplication, and concurrency behavior.
- `integration.rs` — HTTP/API end-to-end behavior.
- `validation_api.rs` — validation behavior through the API.
- `auth_coverage.rs` — protected-route authentication coverage.

### `stress-tests/`

Stress tests are deliberately outside ordinary integration tests. They are allowed to run for longer, create sustained concurrency, and collect resource/performance observations.

There should be two practical profiles:

1. **Smoke** — bounded, deterministic enough for CI/release gating.
2. **Release/heavy** — longer concurrency and pressure testing before publishing V1.

Stress tooling must report failures clearly and must not silently turn a correctness failure into a performance statistic.

---

## 4. Documentation

`docs/` is for durable engineering and operational knowledge.

### Required V1 documents

- `production-readiness-v1.0.0.md` — the finite V1 release requirements, acceptance gate, and work log.
- `project-structure.md` — this repository architecture/layout contract.
- `architecture.md` — system-level runtime/data-flow architecture.
- `configuration.md` — complete configuration reference.
- `security.md` — authentication, authorization, secrets, headers, dependency/security posture, and security boundaries.
- `deployment.md` — production deployment and packaging.
- `https-deployment.md` — HTTPS/reverse-proxy deployment.
- `operations.md` — normal operating procedures, logs, health, backup/restore, and lifecycle.
- `troubleshooting.md` — diagnosis of common runtime, DNS, database, frontend, and deployment failures.

### UI documentation

`docs/ui/` is the repository-authoritative UI specification. It should describe:

- Information architecture.
- Screen layouts.
- Component/state behavior.
- Loading/empty/error/disconnected/reconnecting states.
- Responsive behavior.
- Cache TTL/live-update behavior.
- Live DNS log presentation.

Figma can remain a design/prototyping tool, but V1 must not depend on Figma being available.

---

## 5. Portfolio

`portfolio/` is intentionally separate from `docs/` because its audience and purpose differ.

It should present the finished project, not document every implementation detail.

### `portfolio/README.md`

Short project overview suitable as a portfolio entry.

### `portfolio/case-study.md`

Curated explanation of:

- Problem and motivation.
- System architecture.
- Engineering decisions.
- DNS/cache/API/security challenges.
- Verification and reliability evidence.
- Final UI.
- Deployment model.

### `portfolio/media/`

Curated screenshots and diagrams only. Do not place runtime assets, generated database files, debug dumps, test fixtures, or temporary captures here.

Portfolio media should be intentionally selected and versioned rather than copied from development output wholesale.

---

## 6. Scripts

`scripts/` contains repeatable project operations that do not belong in the application itself.

Examples:

- DNS smoke testing.
- Stress-test invocation.
- Release packaging.
- Clean-tree verification.
- Deployment helpers.

Platform-specific scripts are acceptable (`.ps1`, `.sh`) where they provide useful developer/deployment automation. The script should document its prerequisites and failure conditions.

---

## 7. Generated and local-only files

These do not belong in version control:

```text
target/
frontend/node_modules/
frontend/dist/
*.db
*.db-shm
*.db-wal
*.db-journal
logs/
*.log
metadata.json
screenshots/temp/
local configuration containing secrets
editor/IDE state
```

The exact ignore rules belong in `.gitignore`; this document defines the policy.

Test databases should use temporary directories where practical so the repository remains clean even after failed tests.

---

## 8. Legacy frontend transition

The current repository contains a hand-written dashboard under `src/assets/`. This is a transitional implementation.

It should **not** become a permanent second frontend architecture. The migration path is:

1. Build the React/TypeScript/Vite frontend under `frontend/`.
2. Reproduce and improve the existing V1 workflows and UI states.
3. Build static assets with Vite.
4. Integrate the generated frontend with Axum's static-file serving.
5. Verify browser behavior against the backend.
6. Remove the legacy HTML/CSS/JS from `src/assets/` once no longer required.

Do not duplicate application logic between the legacy frontend and React frontend during the final V1 state.

---

## 9. Root-level file policy

The repository root should remain intentionally small. Root files must have a repository-wide purpose.

`task.md` and `implementation_plan.md` are legacy planning artifacts currently present on the branch. They should not become competing V1 trackers. Their durable information should be migrated into the V1 release plan or the appropriate architecture/operations document, then the redundant files should be removed as part of the structure cleanup.

The V1 source of truth for release requirements is:

```text
docs/production-readiness-v1.0.0.md
```

This document is the source of truth for repository organization.

---

## 10. Implementation rule

Do not restructure the codebase just to make the directory tree look sophisticated.

The intended abstraction level is:

```text
Transport       -> Web/DNS
Application     -> shared state + domain operations
Persistence     -> DB
Infrastructure  -> cache/config/privileges/upstream
Presentation    -> frontend
Verification    -> tests/stress-tests
Documentation   -> docs
Presentation    -> portfolio
```

A new module should be introduced only when it represents a meaningful responsibility or boundary. A function should remain close to the module that owns its data and invariants.

The V1 structure is considered complete when the repository is understandable from the tree alone, the frontend/backend boundaries are explicit, tests have clear purposes, generated files are excluded, and no duplicate planning/documentation system remains.
