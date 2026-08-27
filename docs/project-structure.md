# MyDNS Repository Structure

## Purpose

This document defines the repository structure for MyDNS V1.0.0. The repository should be understandable from the tree alone, with clear boundaries between the Rust DNS/backend application, the future Node/React frontend, verification, operations, and durable documentation.

The structure intentionally avoids unnecessary enterprise-style layers. A module or directory exists because it owns a real responsibility.

## Target layout

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
├── frontend/                    # React + TypeScript + Vite V1 frontend
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
├── scripts/
│   └── repeatable development/release/deployment helpers
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
│   │   ├── tests.rs
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
├── stress-tests/
│   ├── README.md
│   ├── dns/
│   ├── api/
│   └── websocket/
├── tests/
│   ├── auth_coverage.rs
│   ├── cache_persistence.rs
│   ├── dns_integration.rs
│   ├── integration.rs
│   ├── upstream_integration.rs
│   └── validation_api.rs
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
└── config.ini.example
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

The production UI is a separate Node project under `frontend/` using React, TypeScript and Vite. Node is a build-time dependency; the production MyDNS runtime should not require Node merely to run the DNS server.

Use feature-oriented organization rather than a monolithic `App.tsx` or a directory mirroring every backend endpoint:

- `components/` — reusable UI primitives.
- `features/` — user-facing areas such as records, cache, logs, settings and authentication.
- `services/` — REST and WebSocket clients.
- `state/` — genuinely cross-feature application state.
- `hooks/` — reusable React hooks.
- `types/` — API/domain-facing TypeScript types.
- `lib/` — generic utilities.
- `styles/` — global styling and design tokens.

The backend remains authoritative for cache expiration. The cache feature may display a live TTL/countdown, while live DNS logs should consume structured backend events rather than parse terminal output.

The frontend must explicitly model loading, empty, error, unauthorized, expired-session, reconnecting and disconnected states.

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

## Scripts

`scripts/` contains repeatable project operations that do not belong inside the application, such as DNS smoke tests, stress-test invocation, release packaging, clean-tree checks and deployment helpers.

## Generated and local-only files

These must not be committed:

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
IDE/editor state
```

`.gitignore` is the executable policy; this section documents the intent. Tests should use temporary database locations where practical so failed tests do not leave project files behind.

## Legacy frontend transition

The current branch contains a hand-written dashboard under `src/assets/` (`dashboard.html`, `app.js`, `style.css`). This is transitional code, not a second permanent frontend architecture.

The V1 migration path is:

1. Build the React/TypeScript/Vite frontend under `frontend/`.
2. Reproduce and improve the required V1 workflows and UI states.
3. Build the frontend with Vite.
4. Serve the generated assets through Rust/Axum.
5. Verify browser behavior against the real backend APIs and WebSocket events.
6. Remove `src/assets/` and its transitional integration once React fully replaces it.

Do not maintain duplicate application logic between the legacy dashboard and React frontend in the final V1 state.

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
