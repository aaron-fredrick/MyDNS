# Phase 3 — Web Server Boundaries

## Objective

Separate HTTP server lifecycle, route/middleware construction, and embedded frontend serving.

## Target

```text
web/
├── mod.rs
├── server.rs
├── routes.rs
├── frontend.rs
├── auth.rs
├── validation.rs
└── ws.rs
```

### `web/server.rs`

Own listener binding, startup, graceful shutdown, and HTTP server lifecycle.

### `web/routes.rs`

Own Axum router construction, API route registration, middleware composition, CORS, and security-header layers.

### `web/frontend.rs`

Own embedded frontend assets, root response, static asset responses, MIME handling, and frontend fallback behavior.

### Existing modules

Keep `auth.rs` for authentication/session/rate limiting, `validation.rs` for HTTP-facing validation, and `ws.rs` for WebSocket transport.

## Boundary

`src/frontend/` is the React/TypeScript frontend application. `src/mydns/web/` is only the Rust HTTP/WebSocket serving boundary.

## Rules

- API resource handlers stay in `api/v1/`.
- DNS logic stays in `dns/`.
- Do not create controllers.
- Preserve authentication, CORS, security, asset-serving, and shutdown behavior.
- Move code; do not redesign the web layer.

## Acceptance criteria

- [ ] `server.rs` is lifecycle-focused.
- [ ] `routes.rs` owns router/middleware construction.
- [ ] `frontend.rs` owns embedded asset serving.
- [ ] API handlers remain resource-oriented.
- [ ] HTTP/frontend behavior is unchanged.
- [ ] Relevant validation passes or CI evidence is recorded.
- [ ] Tracker is updated.
- [ ] Exactly one logical commit represents the phase.
