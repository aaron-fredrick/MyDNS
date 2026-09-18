# MyDNS Frontend Implementation

## Architecture

MyDNS uses React + Vite + TypeScript for the management UI and Rust/Axum for the runtime server.

- Node.js is a build-time dependency only.
- React is compiled by Vite into the production `web/` directory for packaging.
- Rust serves the generated `web/` static assets at runtime.
- Rust continues to own DNS, authentication, APIs, cache, upstream resolution, metrics, and WebSocket logs.
- There is no Node/Express server in production.

## Repository layout

```text
MyDNS/
├── frontend/
│   ├── src/
│   │   ├── api.ts
│   │   ├── main.tsx
│   │   └── styles.css
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── tsconfig.app.json
│   ├── tsconfig.node.json
│   └── vite.config.ts
├── web/                 # generated production frontend; not source code
├── src/web/server.rs
├── Cargo.toml
└── package.json
```

## Root developer commands

Run all frontend commands from the repository root:

```powershell
npm install
npm run typecheck
npm run build
```

Development server:

```powershell
npm run dev
```

Vite proxies `/api` and `/ws` to the local Rust HTTP server during development.

## Production build

The release sequence is:

```text
npm install
    ↓
npm run typecheck
    ↓
npm run build
    ↓
web/
    ↓
cargo build --release
    ↓
MyDNS distribution
```

`web/` and `node_modules` are ignored by Git. A generated `package-lock.json` should be committed after the first local `npm install` so future builds can use reproducible `npm ci` installs.

## Rust serving model

API routes remain under `/api/v1` and the WebSocket remains at `/ws`. Unknown API paths return `404` rather than the SPA entry point.

Non-API browser paths are handled by the generated Vite assets. If a client-side route does not map to a concrete asset, Rust serves `index.html`, allowing React Router to handle the route.

## Security boundary

The existing Rust authentication implementation remains authoritative. HTTP API requests use the Bearer JWT. Browser WebSocket authentication uses the `mydns-auth.<token>` WebSocket subprotocol because browser WebSocket clients cannot set an arbitrary `Authorization` header.

Security headers and release CORS handling remain in the Rust server.

## Distribution relationship

The production frontend is packaged separately from the Rust executable as `web/`. V1 distribution and installation layout is documented in `docs/v1-distribution.md`. The portable package and installer must ship the matching Rust binary and frontend build together while keeping configuration, database data, and logs persistent across upgrades.
