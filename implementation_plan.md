# MyDNS Production Readiness — Implementation Plan

## Background

MyDNS is a feature-complete Rust DNS server + management dashboard. The project is on the `production-readiness` branch. The current baseline passes `cargo fmt`, `cargo check`, and `cargo clippy -- -D warnings` cleanly. The intermittent `database is locked` failure in `test_concurrent_cache_upserts_remain_deduplicated` is the top-priority blocker.

This plan works strictly in priority order from the `production-readiness.md` document, running format/check/clippy after every phase and updating the doc at each milestone.

---

## Phase 0 — Hygiene (immediate, blocking everything else)

> [!CAUTION]
> The working tree already has stale test WAL/SHM artifacts (`test_cache_*.db-shm`, `test_cache_*.db-wal`). These must be removed before any commits and `.gitignore` must be verified to prevent recurrence.

### Tasks

1. **Clean stale test artifacts** — delete all `test_cache_*.db`, `test_*.db`, `*.db-shm`, `*.db-wal` from the repo root.
2. **Verify `.gitignore`** — ensure `*.db`, `*.db-shm`, `*.db-wal`, `*.db-journal`, `test_*.db`, `logs/` are all excluded.
3. **Remove `check_err.txt`** — stale debug artifact in repo root; should be gitignored or deleted.

---

## Phase 1 — SQLite Concurrency Gate (P0 blocker)

The core issue: `insertCache` uses `ON CONFLICT DO UPDATE` which is an upsert. Under concurrent tokio tasks sharing the same pool with WAL mode, SQLite can still emit `SQLITE_BUSY` when write transactions contend, even with a 5-second busy timeout, because **the timeout applies per connection, and SQLx's pool may acquire multiple connections that contend at the write serialisation point.**

### Root cause analysis

- `db::init` opens a `SqlitePool` with default pool size (likely `max_connections = 10`).
- Concurrent upserts each get their own connection from the pool.
- SQLite WAL allows one writer at a time — if 32 concurrent writes land simultaneously, those beyond the first must wait; if the busy timeout is misconfigured or not respected at the pool level, they fail.
- The fix is to set `max_connections = 1` for the pool **or** enforce a write serialisation semaphore, **or** set a larger pool-level busy timeout using `SqliteConnectOptions::busy_timeout`.

### Proposed fix

- Set `max_connections(1)` on the `SqlitePoolOptions` for the WAL-mode write path. This is safe for SQLite and eliminates all `SQLITE_BUSY` from concurrent writers because all writes are serialised through the single connection.
- Alternatively (better for read throughput): keep multi-connection pool but add a `Mutex`-protected write connection for all mutating DB operations. For simplicity and correctness, start with `max_connections(1)`.
- After the fix: run the full `cargo test` suite 5+ times to confirm stability.

#### [MODIFY] [db/mod.rs](file:///d:/projects/MyDNS/src/db/mod.rs)
Switch from `SqlitePool::connect_with` to `SqlitePoolOptions::new().max_connections(1)`.

---

## Phase 2 — P0 Correctness & Security Blockers

### 2a — Zone/Ownership enforcement
Currently any authenticated user can create records for any domain. We need a configurable zone whitelist.

#### [MODIFY] [config.rs](file:///d:/projects/MyDNS/src/config.rs)
Add `allowed_zones: Vec<String>` config field (comma-separated in `config.ini`, default = allow all).

#### [MODIFY] [web/validation.rs](file:///d:/projects/MyDNS/src/web/validation.rs)
Add `validate_zone(name, allowed_zones)` check — reject names not under any allowed zone when the whitelist is non-empty.

#### [MODIFY] [web/records_api.rs](file:///d:/projects/MyDNS/src/web/records_api.rs)
Thread `AppConfig::allowed_zones` into create/update validation.

### 2b — Unix privilege dropping (fail-closed)
`dropPrivileges()` exists but is **never called** from `main.rs`. It drops only UID (not GID/supplemental groups), and only drops to `nobody` with no config option.

#### [MODIFY] [src/privileges.rs](file:///d:/projects/MyDNS/src/privileges.rs)
- Drop GID and supplemental groups before UID on Unix.
- Make target user/group configurable via `config.ini` (`run_as_user`, `run_as_group`).
- Return `Err` (fail-closed) if `setuid`/`setgid` fails.

#### [MODIFY] [src/main.rs](file:///d:/projects/MyDNS/src/main.rs)
- Call `dropPrivileges()` after DNS sockets are bound (before spawning the HTTP task).

### 2c — Graceful shutdown / OS signal handling
HTTP server uses `CancellationToken` for graceful shutdown. DNS server cancels the token on exit, which triggers HTTP shutdown. However:
- No OS signal handling (`SIGINT`, `SIGTERM` on Unix; `Ctrl+C` on Windows) is wired up.
- Shutdown is not symmetric: if the HTTP server exits first, DNS keeps running.

#### [MODIFY] [src/main.rs](file:///d:/projects/MyDNS/src/main.rs)
- Add a `tokio::signal` listener task that cancels the token on `SIGINT`/`SIGTERM`/`Ctrl+C`.
- Make HTTP exit also trigger `cancel()` so both sides die together.

### 2d — HTTPS / TLS decision
> [!IMPORTANT]
> Decision required: Built-in TLS (rustls/axum-server-tls) or documented reverse-proxy topology?

**Recommended: document the reverse-proxy approach** (nginx/caddy in front) for simplicity, correctness, and cert management. Add a TLS section to docs. The HTTP server should enforce a `Strict-Transport-Security` header when in production mode.

### 2e — Security headers
Add `tower-http::set-header` middleware to attach:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Content-Security-Policy` (dashboard-appropriate policy)
- `Strict-Transport-Security` (when in production mode)

#### [MODIFY] [src/web/server.rs](file:///d:/projects/MyDNS/src/web/server.rs)
Add security header layer.

### 2f — Request size limits & rate limiting
Login endpoint has no rate limiting or body size cap.

#### [MODIFY] [src/web/server.rs](file:///d:/projects/MyDNS/src/web/server.rs)
Add `tower::limit::ConcurrencyLimitLayer` and `axum::extract::DefaultBodyLimit` (e.g., 64 KiB).

---

## Phase 3 — P1: DNS Correctness

### 3a — NS and TXT record support
`buildRecord` in `dns/handler.rs` returns `None` for any type not in `{A, AAAA, CNAME, MX, PTR}`. NS and TXT records are used in real zones.

#### [MODIFY] [src/dns/handler.rs](file:///d:/projects/MyDNS/src/dns/handler.rs)
Add `RecordType::NS` and `RecordType::TXT` arms to `buildRecord`.

#### [MODIFY] [src/web/validation.rs](file:///d:/projects/MyDNS/src/web/validation.rs)
Extend `validate_record_type` to accept `NS` and `TXT`; add value validators for each.

### 3b — Authoritative wire-level DNS tests
Add tests in `tests/dns_integration.rs` covering:
- A, AAAA, CNAME, MX, NS, TXT, PTR record types over UDP and TCP
- NODATA, NXDOMAIN, SERVFAIL response codes
- Case normalization and trailing-dot normalization
- CNAME chains (multi-hop, loop detection)

### 3c — Upstream failure coverage
Add tests verifying behaviour when the upstream resolver:
- Times out
- Returns SERVFAIL
- Returns NXDOMAIN

---

## Phase 4 — P1: Cache & Persistence Hardening

### 4a — Test isolation (deterministic temp dirs)
Test databases are created in the **current working directory** (repo root). The `.gitignore` must cover them, and cleanup must be failure-safe (the `Drop` impl in `cache_persistence.rs` already handles this, but `integration.rs` only removes the `.db` file, not the WAL/SHM files).

#### [MODIFY] [tests/integration.rs](file:///d:/projects/MyDNS/tests/integration.rs)
Update `start_test_server` to also remove `*.db-shm` and `*.db-wal` in cleanup.

#### [MODIFY] [tests/cache_persistence.rs](file:///d:/projects/MyDNS/tests/cache_persistence.rs)
Use `std::env::temp_dir()` as the base directory for test databases to keep repo root clean.

### 4b — CI matrix integration for cache_persistence
The `cache_persistence` test target must run in the normal `cargo test` suite, not only as a focused target.

---

## Phase 5 — P1: Web/API Hardening

### 5a — Authorization tests for every protected route

#### [NEW] [tests/auth_coverage.rs](file:///d:/projects/MyDNS/tests/auth_coverage.rs)
Test that `GET/POST /records`, `PUT/DELETE /records/:id`, `GET/PUT /settings`, `GET/DELETE /cache`, and `/ws` all return `401` without a token and `200`/`404` with a valid token.

### 5b — Audit logging
Login attempts (success and failure) and all destructive/admin operations should emit structured audit log entries without embedding credentials.

`auth.rs::login` already logs success. Add failure logging.

#### [MODIFY] [src/web/auth.rs](file:///d:/projects/MyDNS/src/web/auth.rs)
Log failed login attempts at `warn!` level (username only, never password).

### 5c — Standardise error payloads
`ApiError` already uses `{"error": "..."}` consistently. Verify no handler leaks stack traces or internal details.

---

## Phase 6 — P1: Dependency & Supply-Chain Security

### 6a — cargo audit
Run `cargo audit` and review every advisory. Document dispositions in `production-readiness.md`.

### 6b — GitHub Actions hardening
#### [MODIFY] [.github/workflows/*.yml](file:///d:/projects/MyDNS/.github)
- Pin all Action versions to their SHA256 digest.
- Add `cargo audit` step.
- Add `cargo deny` or equivalent.

---

## Phase 7 — P1: CI Quality Gates

### 7a — CI workflow
#### [MODIFY / NEW] `.github/workflows/ci.yml`
Ensure the workflow runs:
1. `cargo fmt --check`
2. `cargo check`
3. `cargo clippy -- -D warnings`
4. `cargo test` (full suite) on Linux and Windows
5. `cargo audit`
6. Release profile build verification

---

## Phase 8 — P2: Release Engineering, Containerisation, Native Deployment, Docs

These will be addressed sequentially once P0/P1 gates are closed.

### 8a — Dockerfile (multi-stage, non-root)
### 8b — systemd unit file
### 8c — README rewrite
### 8d — Configuration reference doc
### 8e — Security model doc

---

## Additional Plans (New — not in original doc)

> [!NOTE]
> The following items were identified during code review and are not in the original production-readiness checklist.

1. **`config.rs` — admin password retention**: After seeding the admin user, `admin_password` remains in memory for the life of the process. Zeroize it after first use.
2. **`main.rs` — step numbering bug**: Steps 5, 6, 6, 6, 6 — fix the step comment numbering (cosmetic but indicates copy-paste errors).
3. **`cache/mod.rs` — eviction strategy**: The current LRU-approximation (remove the first HashMap entry when size ≥ 5000) is not deterministic. Consider using an LRU structure (`lru` crate) or documenting the approximation.
4. **`web/server.rs` — `stats` route is unauthenticated**: The `/api/v1/stats` endpoint returns cache hit/miss counts and uptime without authentication. Decide if this is intentional (fine for a local dashboard) and document it; at minimum add a comment.
5. **`dns/handler.rs` — `mydns.local` special record**: Hard-coded hostname `mydns.local` should be configurable in `config.ini`.
6. **Integration test port collision**: `rand::random::<u16>() % 10000 + 20000` can still collide under parallel test execution. Use port `0` (OS-assigned) and extract the actual bound port from the listener.

---

## Verification Plan

After each phase:
1. `cargo fmt --check` — must pass.
2. `cargo check` — must pass.
3. `cargo clippy -- -D warnings` — must pass.
4. `cargo test` — must pass with 0 failures across 5 consecutive runs (for the concurrency phase).
5. Update `docs/production-readiness.md` checkboxes.

### Final gate
- Full test suite green on Windows (primary dev platform) and Linux (CI).
- Working tree clean of `*.db`, `*.db-shm`, `*.db-wal`, `*.db-journal`, `logs/` after test run.
- Reproducible container and native deployment.
- Tag the production-ready commit.
