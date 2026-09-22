# Phase 1 — Database Boundaries

## Objective

Split the overloaded database modules by persistence responsibility without changing behavior.

## Current issue

`db/records.rs` currently mixes DNS records, cache persistence, zones, users, and development cleanup. `db/mod.rs` also mixes database initialization, migrations, and settings.

## Target ownership

### `db/records.rs`
Own `DnsRecord`, `CreateRecord`, `UpdateRecord`, record row conversion, all record CRUD, and `delete_dev_records`.

### `db/zones.rs`
Own `Zone`, zone row conversion, zone listing/add/remove/seeding, and `create_apex_soa_and_ns`.

### `db/cache.rs`
Own `CacheRow`, cache row conversion, and all persistent cache operations: get/insert/list/delete, CNAME dependent lookup, name/zone invalidation, clear, and prune.

### `db/users.rs`
Own `find_user_hash` and `seed_admin`.

### `db/settings.rs`
Own `get_setting` and `set_setting`.

### `db/migrations.rs`
Own the existing migration/schema/index/constraint execution currently performed by `run_migrations`.

### `db/blocklist.rs`
Remain the owner of blocklist persistence. Do not move the runtime `BlocklistIndex` from `dns/blocklist.rs`.

### `db/mod.rs`
Become the database entry point: module declarations/re-exports, pool initialization, and top-level `init`. It must not become another dumping ground.

## Procedure

1. Inspect current modules and every call site.
2. Move definitions/functions with minimal body changes.
3. Update imports.
4. Move only tests that are coupled to module-private implementation; do not redesign the test suite.
5. Search the repository for all moved symbols.
6. Validate formatting/compile/tests through the strongest available mechanism.
7. Update `TRACKER.md`.
8. Create exactly one logical commit.

## Non-goals

No SQL redesign, schema change, API redesign, DNS change, generic repository/service layer, or domain-model layer.

## Acceptance criteria

- [ ] Each database file has one clear persistence responsibility.
- [ ] `db/mod.rs` is a small entry point.
- [ ] Existing SQL/schema behavior is unchanged.
- [ ] Callers use the correct owning module.
- [ ] No generic `models/`, `repositories/`, or `services/` layer was added.
- [ ] Relevant validation passes or CI evidence is recorded.
- [ ] Tracker is updated.
- [ ] Exactly one logical commit represents the phase.
