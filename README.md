# MyDNS

A high-performance, concurrent DNS server and web management dashboard written in Rust.

## Features

- **DNS Server** — UDP + TCP on port 53, handling A, AAAA, CNAME, and MX records.
- **Smart Resolution Pipeline** — Local cache → SQLite DB → Upstream DNS → NXDOMAIN.
- **Configurable Upstream Chain** — Cloudflare (1.1.1.1) and auto-detected router/gateway DNS, with switchable priority order.
- **TTL-Aware Cache** — In-memory HashMap cache with background expiry pruning.
- **Web Dashboard** — Modern dark-mode SPA served from the binary itself (no external files).
- **Real-Time Logs** — Every DNS query, cache hit/miss, and CRUD event is streamed to the dashboard via WebSocket.
- **JWT Auth** — Argon2-hashed admin password, JWT-protected API routes.
- **Fate-Sharing Shutdown** — If either the DNS or HTTP server exits, the other shuts down gracefully.
- **Privilege Safety** — Checks for Administrator/root on startup; optional privilege drop to `nobody` after socket bind (Unix).

---

## Quick Start

### Prerequisites

- Rust stable (1.75+)
- Run as **Administrator** (Windows) or **root/sudo** (Linux/macOS)

### Build & Run

```sh
# Build
cargo build --release

# Run (elevated)
sudo ./target/release/mydns          # Linux/macOS
# or right-click → Run as Administrator (Windows)
```

The server starts on:
- **DNS**: `0.0.0.0:53`
- **Dashboard**: `http://localhost:8080`

### Default Admin Credentials

| Field    | Default        |
|----------|----------------|
| Username | `admin`        |
| Password | `changeme123`  |

> **Change the password** by setting `ADMIN_PASSWORD=<new>` before the first run, or update the hash directly in the database.

---

## Configuration (Environment Variables)

| Variable            | Default           | Description                                  |
|---------------------|-------------------|----------------------------------------------|
| `DNS_PORT`          | `53`              | UDP/TCP port for the DNS server              |
| `HTTP_PORT`         | `8080`            | HTTP port for the dashboard                  |
| `DB_PATH`           | `mydns.db`        | Path to the SQLite database file             |
| `JWT_SECRET`        | *(auto-generated)*| HMAC secret for JWT signing                  |
| `ADMIN_USERNAME`    | `admin`           | Admin username (seeded on first run)         |
| `ADMIN_PASSWORD`    | `changeme123`     | Admin password (hashed on first run)         |
| `RESOLVER_PRIORITY` | `cloudflare_first`| `cloudflare_first` or `router_first`         |
| `CLOUDFLARE_DNS`    | `1.1.1.1:53`      | Primary/secondary upstream DNS address       |
| `ROUTER_DNS`        | *(auto-detected)* | Router gateway DNS override                  |

---

## Upstream Resolver Priority

| Mode              | Order                                  | Best for                          |
|-------------------|----------------------------------------|-----------------------------------|
| `cloudflare_first`| 1.1.1.1 → Router DNS                  | Most public domains *(default)*   |
| `router_first`    | Router DNS → 1.1.1.1                  | ISP-specific or private domains   |

Priority and upstream IPs can be changed live from the **Settings** panel in the dashboard without restarting.

---

## API Reference

All routes are under `/api/v1/`. Protected routes require `Authorization: Bearer <token>`.

| Method | Route               | Auth | Description                     |
|--------|---------------------|------|---------------------------------|
| POST   | `/auth/login`       | No   | Obtain a JWT                    |
| GET    | `/records`          | Yes  | List all DNS records            |
| POST   | `/records`          | Yes  | Create a DNS record             |
| PUT    | `/records/:id`      | Yes  | Update a DNS record             |
| DELETE | `/records/:id`      | Yes  | Delete a DNS record             |
| GET    | `/stats`            | No   | Uptime, cache metrics           |
| GET    | `/settings`         | Yes  | Read resolver settings          |
| PUT    | `/settings`         | Yes  | Update resolver settings (live) |
| GET    | `/ws`               | No   | WebSocket — live log stream     |

---

## Testing

```sh
# Unit tests (cache TTL logic + DNS record builder)
cargo test --lib

# Integration tests (requires a running server on port 8181)
cargo test --test integration
```

---

## Project Structure

```
src/
├── main.rs          — Entry point, orchestration, tracing setup
├── config.rs        — AppConfig, ResolverPriority
├── state.rs         — AppState (Arc-shared across all tasks)
├── privileges.rs    — Elevation check + privilege drop (cross-platform)
├── db/              — SQLite pool, schema migrations, CRUD
├── cache/           — TTL cache + background pruner + unit tests
├── dns/             — hickory-server handler, upstream resolver, server bootstrap
└── web/             — Axum router, auth, CRUD API, settings, stats, WebSocket
src/assets/
└── dashboard.html   — Embedded dark-mode SPA
tests/
└── integration.rs   — HTTP API integration tests
```
