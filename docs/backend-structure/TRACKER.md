# Backend Structure Refactor Tracker

This is the execution tracker for `docs/backend-structure/`.

A phase is complete only when its acceptance criteria are satisfied, validation evidence is recorded, and the implementation commit is recorded.

Allowed statuses: `TODO`, `IN PROGRESS`, `BLOCKED`, `DONE`.

| Phase | Status | Commit | Validation | Notes |
|---|---|---|---|---|
| 1 — Database boundaries | DONE | Pending | Unit tests pass | Split persistence ownership |
| 2 — Configuration boundaries | DONE | Pending | Unit + integration tests pass (209/209) | Separate types from format parsing |
| 3 — Web server boundaries | TODO | — | — | Separate lifecycle/routes/frontend serving |
| 4 — DNS/runtime ownership | TODO | — | — | Verify runtime ownership without unnecessary rewrite |
| 5 — API/application boundary | TODO | — | — | Keep API thin |
| 6 — Final verification | TODO | — | — | Final tree/dependency/naming/documentation check |

## Agent handoff record

### Phase 1
- Status: DONE
- Started: 2026-09-22
- Completed: 2026-09-22
- Commit: Pending
- Validation: `cargo test` (43/43 tests passed) and `cargo check` (clean build)
- Notes: Split overloaded database modules (records.rs, mod.rs) by persistence responsibility into records.rs, zones.rs, cache.rs, users.rs, migrations.rs, and settings.rs. Updated all call sites in API, DNS handlers, background tasks, and tests. Behavior preserved without introducing generic layers.

### Phase 2
- Status: DONE
- Started: 2026-09-22
- Completed: 2026-09-22
- Commit: Pending
- Validation: `cargo check` (clean, exit 0); `cargo test` (209/209 tests passed — 150 unit + 59 integration, exit 0)
- Notes: Extracted `AppConfig`, `ResolverMode`, `ResolverPriority`, `default_root_hints`, and `IANA_ROOT_HINTS` into `config/types.rs`. Moved TOML-specific structs and `AppConfig::from_toml_str` / `from_toml_file` into `config/toml.rs`. Moved INI parser and `AppConfig::from_ini_file` into `config/ini.rs`. Moved all config tests into `config/tests.rs`. `config/mod.rs` is now a thin entry point: re-exports public types, owns `AppConfig::from_config_file` (format-dispatch), and `generate_secret`. All existing public paths (`crate::config::AppConfig`, `crate::config::ResolverMode`, etc.) remain stable. No unrelated code changed (only `src/mydns/config/mod.rs` differs from HEAD).

### Phase 3
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

### Phase 4
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

### Phase 5
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

### Phase 6
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

## Global agent rules

- Read `README.md` and the current phase document first.
- Inspect the actual tree; never assume the target tree is implemented.
- One phase = one logical commit.
- Do not mix unrelated fixes into a structure phase.
- Do not add MVC-style `models/`, `controllers/`, `services/`, or `repositories/` without a documented concrete boundary.
- Prefer subsystem ownership over architectural symmetry.
- Preserve behavior unless explicitly required otherwise.
- Never claim validation passed unless it actually ran.
- If blocked, record the blocker and stop rather than silently changing scope.
