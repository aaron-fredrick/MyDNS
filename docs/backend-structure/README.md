# MyDNS Backend Structure Refactor

This folder is the implementation plan for restructuring the Rust backend by **real subsystem ownership**.

## Core rule

> A developer or AI agent should be able to infer a file's responsibility from its path and name without opening it.

This is **not** an MVC migration. Do not introduce generic `models/`, `controllers/`, `services/`, `repositories/`, `managers/`, or `utils/` layers just for architectural appearance.

Types belong beside the subsystem that owns their meaning:

- database representations → `db/`
- runtime DNS structures → `dns/`
- runtime cache structures → `cache/`
- HTTP request/response representations → `api/`
- application configuration → `config/`
- shared runtime composition → `state/`

Create a separate domain module only when a domain concept has independent behavior that genuinely needs to exist across multiple boundaries. Do not create one speculatively.

## Target backend layout

```text
src/mydns/
├── api/
│   └── v1/
│       ├── blocklist.rs
│       ├── cache.rs
│       ├── records.rs
│       ├── settings.rs
│       ├── stats.rs
│       └── zones.rs
├── cache/
├── config/
│   ├── mod.rs
│   ├── types.rs
│   ├── toml.rs
│   ├── ini.rs
│   └── tests.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   ├── settings.rs
│   ├── users.rs
│   ├── records.rs
│   ├── zones.rs
│   ├── cache.rs
│   ├── blocklist.rs
│   └── tests/
├── dns/
│   ├── server.rs
│   ├── blocklist.rs
│   ├── record_index.rs
│   ├── zone_trie.rs
│   └── handler/
│       ├── mod.rs
│       ├── local.rs
│       ├── cache.rs
│       ├── blocklist.rs
│       ├── records.rs
│       └── upstream/
│           ├── mod.rs
│           ├── forward.rs
│           └── recursive.rs
├── error/
├── observability/
├── privileges/
├── state/
└── web/
    ├── server.rs
    ├── routes.rs
    ├── frontend.rs
    ├── auth.rs
    ├── validation.rs
    └── ws.rs
```

The diagram is a target, not a mandate to create empty files. A file exists only when it owns a meaningful responsibility.

## Phase order

| Phase | Area | Result | Status |
|---|---|---|---|
| 1 | Database | Separate persistence by resource/concern | TODO |
| 2 | Configuration | Separate config types from TOML/INI loading | TODO |
| 3 | Web | Separate lifecycle, routes, and frontend serving | TODO |
| 4 | DNS/runtime | Verify ownership after structural splits | TODO |
| 5 | API boundary | Keep HTTP handlers thin and correctly placed | TODO |
| 6 | Final verification | Verify tree, dependencies, naming, and documentation | TODO |

Do phases in order unless the phase document explicitly says otherwise.

## Rules for every phase

1. Preserve behavior unless the phase explicitly requires a behavior change.
2. Prefer moving code over rewriting code.
3. Do not add abstraction layers without a concrete ownership problem.
4. Do not create one file per function.
5. Do not change the unit-test strategy as part of this refactor; update tests only when imports/module ownership require it.
6. Keep public paths stable when that is cheap and does not undermine ownership.
7. Validate the phase before marking it complete.
8. One phase = one logical commit.
9. Do not mix unrelated fixes into a structure commit.
10. If validation cannot be run locally, record that fact and rely on CI rather than claiming success.

## AI-agent continuation protocol

A new agent must:

1. Read this README.
2. Read the current phase document.
3. Inspect the actual branch/tree and compare it with `dev`.
4. Read every file named by the phase before editing.
5. Check `TRACKER.md`.
6. Implement only the current phase.
7. Search for callers/references after moving code.
8. Run the phase's validation or inspect CI.
9. Update the tracker with evidence and commit SHA.
10. Stop for handoff to the next phase.

Never assume the target tree is already implemented because it appears in this document.

## Completion definition

The refactor is complete when persistence, configuration, web transport, DNS/runtime, and API ownership are clear; no unnecessary MVC-style layers exist; behavior is preserved; and the tracker contains evidence for every completed phase.
