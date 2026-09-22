# Phase 2 — Configuration Boundaries

## Objective

Separate application configuration concepts from format-specific parsing while keeping the public configuration API understandable and stable.

## Target

```text
config/
├── mod.rs
├── types.rs
├── toml.rs
├── ini.rs
└── tests.rs
```

### `config/types.rs`

Own `AppConfig`, `ResolverMode`, `ResolverPriority`, and format-independent defaults such as root hints.

### `config/toml.rs`

Own TOML-only structs, TOML parsing/file loading, and conversion into `AppConfig`.

### `config/ini.rs`

Own INI parsing and legacy compatibility, including conversion into `AppConfig`.

### `config/mod.rs`

Own public exports and the clean top-level configuration API. It must not contain another format-specific implementation dump.

### `config/tests.rs`

Own the existing configuration tests after extraction, without changing their intent.

## Rules

- `AppConfig` is an application concept, not a TOML representation.
- TOML/INI implementation details must not leak into DNS, state, API, or web modules.
- Do not duplicate shared validation/parsing logic unnecessarily.
- Preserve existing `AppConfig::from_*` behavior/signatures where practical.
- Do not introduce a generic configuration framework.

## Acceptance criteria

- [ ] Configuration types have a clear owner.
- [ ] TOML parsing is isolated.
- [ ] INI parsing is isolated.
- [ ] `config/mod.rs` is an entry point.
- [ ] Existing tests still cover current behavior.
- [ ] Relevant validation passes or CI evidence is recorded.
- [ ] Tracker is updated.
- [ ] Exactly one logical commit represents the phase.
