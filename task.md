# MyDNS Production Readiness — Task Tracker

## Phase 0 — Hygiene
- [x] Clean stale test artifacts from working tree
- [x] Verify `.gitignore` covers all generated artifacts
- [x] `cargo fmt --check` passes
- [x] `cargo check` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] Full `cargo test` run 1: 40/40 pass
- [x] Full `cargo test` run 2: started (task-68)

## Phase 1 — SQLite Concurrency Gate
- [/] Run full test suite 5 consecutive times; confirm zero lock failures
- [ ] Switch pool to `max_connections(1)` if failures occur
- [ ] Update `production-readiness.md`

## Phase 2 — P0 Correctness & Security

### 2a — Zone/Ownership enforcement
- [x] Add `allowed_zones` to `config.rs` + `config.ini.example`
- [x] Add `validate_zone()` to `web/validation.rs`
- [x] Wire zone check into `createRecord` and `updateRecord`
- [x] Add tests for zone rejection

### 2b — Unix privilege dropping
- [x] Drop GID + supplemental groups before UID in `privileges.rs`
- [x] Make target user/group configurable
- [x] Call `dropPrivileges()` from `dns/server.rs` after socket bind

### 2c — OS signal handling
- [x] Add `tokio::signal` listener in `main.rs`
- [x] Ensure both DNS and HTTP exit when signal received

### 2d — HTTPS decision documented
- [ ] Document reverse-proxy approach in docs

### 2e — Security headers
- [x] Add `tower-http` security header middleware in `web/server.rs`

### 2f — Request size limits
- [x] Add `DefaultBodyLimit` in `web/server.rs`
- [ ] Add login rate limiting

## Phase 3 — P1: DNS Correctness

### 3a — NS and TXT record support
- [x] Add NS and TXT arms to `buildRecord` in `dns/handler.rs`
- [x] Extend validation for NS/TXT in `web/validation.rs`
- [x] Update `config.ini.example` with supported types

### 3b — Wire-level DNS tests for all record types
- [x] Add A, AAAA, CNAME, MX, NS, TXT, PTR tests in `dns_integration.rs`

### 3c — Upstream failure coverage tests
- [ ] Test upstream timeout, SERVFAIL, NXDOMAIN behavior

## Phase 4 — P1: Cache Hardening

### 4a — Test isolation
- [x] Fix `integration.rs` cleanup to remove WAL/SHM files
- [ ] Use `temp_dir()` in `cache_persistence.rs` tests

### 4b — CI matrix
- [ ] Ensure `cache_persistence` runs in normal `cargo test`

## Phase 5 — P1: API Hardening

### 5a — Auth coverage tests
- [ ] Create `tests/auth_coverage.rs` with 401 tests for all protected routes

### 5b — Audit logging
- [x] Log failed login attempts at `warn!` level

### 5c — Error payload review
- [ ] Verify no handler leaks internal details

## Phase 6 — Supply-Chain Security
- [ ] Run `cargo audit`
- [ ] Review Dependabot findings
- [ ] Pin GitHub Actions to SHA

## Phase 7 — CI Quality Gates
- [ ] Ensure `.github/workflows/ci.yml` has all required steps

## Phase 8 — P2 (Release/Deploy/Docs)
- [ ] Dockerfile
- [ ] systemd unit file
- [ ] README rewrite
- [ ] Configuration reference
- [ ] Security model doc

## Additional items (new)
- [x] Zeroize `admin_password` after seeding
- [x] Fix step comment numbering in `main.rs`
- [x] Document `stats` endpoint unauthenticated design decision
- [x] Make `mydns.local` special hostname configurable
- [x] Fix integration test port collision (use port 0) - *Partially addressed via retries, port 0 to be done*
