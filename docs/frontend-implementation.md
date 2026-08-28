# MyDNS Frontend Implementation

## Architecture

MyDNS uses React + Vite + TypeScript for the management UI and Rust/Axum for the runtime server.

- Node.js is a build-time dependency only.
- React is compiled by Vite into `frontend/dist`.
- Rust embeds `frontend/dist` into the release binary with `rust-embed`.
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
frontend/dist
    ↓
cargo build --release
    ↓
Rust embeds frontend/dist
    ↓
Single MyDNS executable
```

`frontend/dist` and `node_modules` are ignored by Git. A generated `package-lock.json` should be committed after the first local `npm install` so future builds can use reproducible `npm ci` installs.

## Rust serving model

API routes remain under `/api/v1` and the WebSocket remains at `/ws`. Unknown API paths return `404` rather than the SPA entry point.

Non-API browser paths are handled by the embedded Vite assets. If a client-side route does not map to a concrete asset, Rust serves `index.html`, allowing React Router to handle the route.

## Security boundary

The existing Rust authentication implementation remains authoritative. HTTP API requests use the Bearer JWT. Browser WebSocket authentication uses the `mydns-auth.<token>` WebSocket subprotocol because browser WebSocket clients cannot set an arbitrary `Authorization` header.

Security headers and release CORS handling remain in the Rust server.
