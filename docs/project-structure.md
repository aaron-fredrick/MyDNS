# MyDNS Repository Structure

## Purpose

This document defines the repository structure for MyDNS V1.0.0. The repository should be understandable from the tree alone, with clear boundaries between the Rust DNS/backend application, the React/TypeScript/Vite frontend, verification, operations, and durable documentation.

The structure intentionally avoids unnecessary enterprise-style layers. A module or directory exists because it owns a real responsibility.

## Target layout

```text
MyDNS/
├── .cargo/
│   └── config.toml
├── .github/
│   └── workflows/
├── docs/
│   ├── production-readiness-v1.0.0.md
│   ├── project-structure.md
│   ├── frontend-implementation.md
│   ├── https-deployment.md
│   ├── v1-distribution.md
│   └── ui/
├── scripts/
├── src/
│   ├── src/frontend/             # React + TypeScript + Vite application
│   │   ├── src/
│   │   ├── index.html
│   │   ├── package.json
│   │   ├── package-lock.json
│   │   ├── tsconfig*.json
│   │   └── vite.config.ts
│   ├── lib.rs
│   ├── main.rs
│   └── mydns/
│       ├── api/
│       ├── cache/
│       ├── config/
│       ├── db/
│       ├── dns/
│       ├── error/
│       ├── observability/
│       ├── privileges/
│       ├── state/
│       └── web/
├── tests/
├── Cargo.toml
├── Cargo.lock
├── package.json
├── package-lock.json
├── README.md
└── LICENSE
```

Do not create placeholder directories simply to match this diagram. The tree may grow when a genuine responsibility requires it.

## Rust backend

`src/` is the production application crate and follows normal Cargo/module conventions.

- `main.rs` — process entry point, configuration loading, dependency construction, listener startup, lifecycle and shutdown coordination. Business logic should not accumulate here.
- `lib.rs` — reusable crate/module boundary used by the binary and integration tests.
- `config.rs` — configuration parsing, defaults, validation and configuration types.
- `state.rs` — shared application state and subsystem handles.
- `privileges.rs` — platform-specific privilege handling.

### `cache/`

Owns cache lookup, insertion/update, expiration, pruning, invalidation and cache statistics. Persistent storage remains the responsibility of `db/`.

### `db/`

Owns SQLite persistence and database-specific operations. Database code must not depend on Axum request/response types or frontend concerns.

### `dns/`

Owns the DNS protocol and resolution path:

- `handler.rs` — query processing and DNS response construction.
- `server.rs` — UDP/TCP listener lifecycle and DNS integration.
- `upstream.rs` — upstream resolution and upstream failure handling.
- `tests.rs` — focused DNS module tests.

### `web/`

Owns the HTTP/WebSocket transport boundary:

- `server.rs` — router, middleware, static-file serving and web lifecycle.
- `auth.rs` — authentication/session/JWT operations.
- `validation.rs` — API/domain input validation and zone ownership rules.
- `error.rs` — safe HTTP error representation.
- `*_api.rs` — resource-specific HTTP handlers.
- `ws.rs` — WebSocket connection/event transport.
- `dashboard.rs` — transitional static-dashboard integration.

The web layer may depend on application/domain functionality. Lower layers must not depend on Axum.

Do not introduce `controllers/`, `services/`, `repositories/`, `models/`, or similar layers unless a real architectural boundary requires them.

## Frontend

The production UI is a React + TypeScript + Vite application under `src/frontend/`.

- `src/frontend/src/app/` — application shell, routing and protected-route handling.
- `src/frontend/src/pages/` — user-facing screens.
- `src/frontend/src/components/` — reusable UI components.
- `src/frontend/src/hooks/` — reusable React hooks.
- `src/frontend/src/styles/` — global/component/page styling.
- `src/frontend/src/api.ts` — typed REST client and authentication token handling.
- `src/frontend/vite.config.ts` — development proxy and production output configuration.

Vite writes production assets to `out/web/`. Rust embeds `out/web/` with `rust-embed`; Node is not required at runtime.

The frontend is therefore intentionally colocated under `src/` but remains a separate Node build project. It is not a Rust module and must not be confused with `src/mydns/web/`, which owns the Rust HTTP/WebSocket server.

## Tests

`tests/` contains Cargo integration/regression tests and remains part of the normal `cargo test` suite.

- `dns_integration.rs` — wire-level DNS behavior.
- `upstream_integration.rs` — upstream success/failure behavior.
- `cache_persistence.rs` — persistence, restart, expiration, deduplication and concurrency behavior.
- `integration.rs` — HTTP/API end-to-end behavior.
- `validation_api.rs` — validation through the HTTP API.
- `auth_coverage.rs` — protected-route authentication coverage.

`stress-tests/` is deliberately separate from correctness tests. It may run sustained concurrency and resource/performance testing. V1 should have a bounded smoke profile and a heavier release profile, with correctness failures reported explicitly.

## Documentation

`docs/` contains durable engineering and operational knowledge. It is not a scratchpad or task list.

The V1 documentation set is:

- `production-readiness-v1.0.0.md` — finite V1 release requirements, acceptance gate and work log.
- `project-structure.md` — repository structure and ownership rules.
- `architecture.md` — system/runtime/data-flow architecture.
- `configuration.md` — configuration reference.
- `security.md` — authentication, authorization, secrets and security posture.
- `deployment.md` — production deployment and packaging.
- `https-deployment.md` — HTTPS/reverse-proxy deployment.
- `operations.md` — operating procedures, logging, health, backup/restore and lifecycle.
- `troubleshooting.md` — diagnosis of runtime, DNS, database, frontend and deployment failures.

`docs/ui/` is the repository-authoritative UI specification covering information architecture, screens, states, responsive behavior, cache live updates and DNS log presentation. Figma can support design work, but V1 must not depend on Figma being available.

`docs/backend-structure/` is the repository-authoritative phased plan for the Rust backend structure refactor. It defines subsystem ownership, phase-by-phase implementation boundaries, acceptance criteria, and an AI-agent handoff tracker.

## Scripts

`scripts/` contains repeatable project operations that do not belong inside the application, such as DNS smoke tests, stress-test invocation, release packaging, clean-tree checks and deployment helpers.

## Generated and local-only files

These must not be committed:

```text
target/
src/frontend/node_modules/
src/frontend/dist/
*.db
*.db-shm
*.db-wal
*.db-journal
logs/
*.log
metadata.json
screenshots/temp/
local configuration containing secrets
IDE/editor state
```

`.gitignore` is the executable policy; this section documents the intent. Tests should use temporary database locations where practical so failed tests do not leave project files behind.

## Frontend build boundary

The old standalone `frontend/` layout described by earlier documentation is obsolete. The authoritative source location is `src/frontend/`.

The build boundary is:

```text
src/frontend/
    │ Vite
    ▼
out/web/
    │ rust-embed
    ▼
MyDNS binary
```

Generated frontend output is not source and must not be committed.

## Root-level policy

The repository root should remain intentionally small. Every root-level file must have a repository-wide purpose.

There is one V1 planning/release source of truth:

```text
docs/production-readiness-v1.0.0.md
```

Temporary task lists, duplicate implementation plans, generated metadata and personal development notes do not belong in the repository.

`README.md`, `SECURITY.md`, `CONTRIBUTING.md`, `LICENSE`, `Cargo.toml`, `Cargo.lock`, `config.ini.example` and `SPONSORS.md` are retained because they have repository/GitHub-wide purposes.

## Abstraction rule

```text
DNS protocol/transport -> src/dns/
HTTP/WebSocket        -> src/web/
Application state     -> src/state.rs + domain modules
Persistence           -> src/db/
Caching               -> src/cache/
Configuration         -> src/config.rs
OS privileges         -> src/privileges.rs
Presentation          -> frontend/
Verification          -> tests/ + stress-tests/
Documentation         -> docs/
Automation            -> scripts/
```

Do not restructure code solely to make the tree look sophisticated. Introduce a module when it represents a meaningful responsibility, boundary, or independently testable concern.

The V1 repository structure is complete when the frontend/backend boundary is explicit, Rust modules have clear ownership, tests have clear purposes, generated/local files are excluded, documentation has one release-plan source of truth, and obsolete planning artifacts are removed.
