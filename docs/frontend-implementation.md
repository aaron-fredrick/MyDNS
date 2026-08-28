# MyDNS frontend implementation

This document records the V1 frontend implementation agreed by the production-readiness plan.

## Architecture

```text
Browser
   │
   ├── React + TypeScript SPA
   │      └── Vite production build
   │
   ▼
Rust / Axum HTTP server
   ├── /api/v1/*  authoritative REST API
   ├── /ws        operational WebSocket
   └── static SPA assets
```

Node.js is build/development tooling only. It is not a production API server and is not required at runtime.

## Frontend stack

- React 19
- TypeScript
- Vite 8
- React Router 8 declarative routing
- Recharts for dashboard telemetry charts
- Plain CSS for the MyDNS design system; no framework is introduced merely to reproduce the existing prototype

The dependency versions are pinned in `frontend/package.json` and should be kept reproducible with a committed npm lockfile once dependencies are installed locally.

## Source layout

```text
frontend/
├── index.html
├── package.json
├── tsconfig*.json
├── vite.config.ts
└── src/
    ├── main.tsx
    ├── api.ts
    └── styles.css
```

The first implementation intentionally keeps the source compact while the prototype is migrated. As the UI grows, page and component modules can be split without changing the backend boundary.

## Build output

Vite writes production assets to:

```text
src/web/dist/
```

This directory is generated and ignored by Git.

## Rust serving

Axum serves `frontend/dist` by default. A different deployment location can be selected with:

```text
MYDNS_WEB_ROOT=<path>
```

The Rust server uses the SPA `index.html` as the not-found fallback for non-API frontend paths. `/api/v1` has its own 404 fallback so an invalid API route never receives the HTML application shell.

## Development workflow

Run the Rust API/server, then:

```powershell
cd frontend
npm install
npm run dev
```

Vite proxies `/api` and `/ws` to the Rust HTTP server. This gives HMR during UI development without adding a Node runtime dependency to MyDNS itself.

## Production workflow

```text
npm install
npm run typecheck
npm run build
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

The CI test and release workflows now build the frontend before compiling/testing or packaging the Rust application.

## Design constraints

The React implementation must preserve the approved MyDNS prototype and brand system:

- Compass logo/mark; no unrelated icon or cyan palette.
- Dark-first MyDNS visual language with the approved blue brand accent.
- Light-theme support remains part of the design-system migration.
- Inter for interface text and JetBrains Mono for technical values.
- Existing spacing, card, table, navigation, form, and responsive hierarchy.
- Dashboard metric labels and explanatory context remain readable rather than decorative.
- Backend remains authoritative for cache hit rate, latency, availability, errors, and other operational metrics.

The existing HTML/CSS/JS prototype remains in the repository as a visual/reference artifact during migration and is not the production frontend.
