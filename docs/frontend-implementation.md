# MyDNS Frontend Implementation

## Architecture

MyDNS uses React + TypeScript + Vite for the management UI and Rust/Axum for the runtime server.

- Frontend source: `src/frontend/`
- Node.js is a build-time dependency only.
- Vite builds the frontend into `out/web/`.
- Rust embeds `out/web/` with `rust-embed` and serves it at runtime.
- There is no Node/Express server in production.
- Rust remains authoritative for DNS, authentication, REST APIs, cache state, metrics, and WebSocket events.

## Repository layout

```text
src/frontend/
├── package.json
├── package-lock.json
├── index.html
├── vite.config.ts
├── tsconfig.json
├── tsconfig.app.json
├── tsconfig.node.json
└── src/
    ├── main.tsx
    ├── api.ts
    ├── app/
    ├── components/
    ├── hooks/
    ├── pages/
    ├── styles/
    ├── assets/
    └── utils/
```

The root `package.json` exposes the frontend commands through the npm workspace:

```text
npm run dev
npm run typecheck
npm run build
```

The workspace is `src/frontend`, not a root-level `frontend/` directory.

## Development

Start the Rust backend separately, then run:

```powershell
npm run dev
```

Vite listens on port 5173 and proxies:

- `/api` → `http://127.0.0.1:8080`
- `/ws` → `ws://127.0.0.1:8080`

## Production build

The release boundary is:

```text
src/frontend/
    │
    │ npm run build
    ▼
out/web/
    │
    │ rust-embed
    ▼
MyDNS executable
```

The production runtime therefore does not require Node.js.

Generated `out/web/` content and `src/frontend/dist/` output must not be committed.

## Rust serving model

Rust owns:

- `/api/v1/*` REST endpoints
- `/ws` WebSocket
- static frontend serving
- SPA fallback for client-side routes
- security headers
- production CORS policy

Unknown API paths remain HTTP 404 and are never rewritten to the SPA entry point.

## Frontend responsibilities

The React application owns presentation and interaction:

- authentication screens
- dashboard rendering
- DNS record CRUD
- zones
- cache inspection and controls
- blocklist management
- settings
- logs
- loading/empty/error states
- WebSocket connection state

The frontend may perform display-only calculations such as countdown rendering and formatting. It must not become the authoritative source for cache expiration, latency percentiles, availability, error rate, or other operational metrics.

## Backend contract

The frontend communicates with typed functions in `src/frontend/src/api.ts`.

Current REST areas include:

- authentication
- statistics
- records
- zones
- cache
- blocklist
- settings

The backend API and WebSocket contracts should remain documented and tested independently of the UI.

## Authentication

The Rust authentication implementation is authoritative.

REST requests use:

```text
Authorization: Bearer <JWT>
```

The browser WebSocket uses the `mydns-auth.<token>` subprotocol because browser WebSocket clients cannot set arbitrary Authorization headers.

Session expiry must clear the browser token and return the user to the login screen.

## V1 completion criteria

The frontend is production-ready when:

1. `npm run typecheck` passes.
2. `npm run build` produces `out/web/`.
3. Rust embeds and serves the generated assets.
4. All V1 workflows use the real backend API.
5. Loading, empty, error, unauthorized, expired-session, reconnecting and disconnected states are explicit.
6. Cache TTL display remains presentation-only and reconciles with authoritative backend state.
7. WebSocket connections are cleaned up during navigation/logout.
8. Browser/E2E tests cover the critical workflows.
