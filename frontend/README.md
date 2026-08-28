# MyDNS frontend

The MyDNS dashboard is a React + TypeScript single-page application built with Vite. Node.js is development/build tooling only; the production runtime remains the Rust/Axum MyDNS service.

## Development

```powershell
cd frontend
npm install
npm run dev
```

Vite runs the UI on `http://localhost:5173` and proxies `/api` and `/ws` to the Rust server on `http://127.0.0.1:8080`.

## Production build

```powershell
cd frontend
npm run typecheck
npm run build
```

The build is emitted to `src/web/dist/`. Do not edit generated files there.

## Rust serving model

The Rust HTTP server serves the generated Vite assets directly. It does not start Node.js in production. By default it serves `frontend/dist`; set `MYDNS_WEB_ROOT` when the built assets live elsewhere.

```powershell
$env:MYDNS_WEB_ROOT = 'C:\ProgramData\MyDNS\web'
```

The Rust API remains under `/api/v1`, while `/ws` remains the resolver event WebSocket. Unknown API paths return 404; non-API frontend paths fall back to the SPA `index.html` so React Router can handle them.

## Build/deploy sequence

```text
npm install
    -> npm run typecheck
    -> npm run build
    -> frontend/dist
    -> cargo build --release
    -> MyDNS Rust process
    -> serves frontend/dist + /api/v1 + /ws
```

The existing static HTML prototype remains in the repository as a visual/reference artifact while the React application becomes the maintained frontend implementation.
