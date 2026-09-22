# Phase 5 — API and Application Boundary

## Objective

Verify that the HTTP API is a thin presentation boundary and that operations live in the subsystem that owns them.

## Existing structure

```text
api/v1/
├── blocklist.rs
├── cache.rs
├── records.rs
├── settings.rs
├── stats.rs
└── zones.rs
```

Keep this resource-oriented structure.

## API responsibility

An API handler may deserialize, authenticate/authorize, validate HTTP input, call the owning operation, and map results/errors into HTTP responses.

It should not own substantial SQL, DNS resolution, cache internals, blocklist indexing, or unrelated global-state mutation.

## Domain/model rule

Do **not** create a `models/` directory.

If an API representation differs materially from a DB or runtime representation, keep each representation at its boundary and add explicit conversion only when it improves correctness. If the shapes are genuinely the same and no boundary requires separation, do not duplicate them for architectural aesthetics.

## Review

Inspect each API module for persistence, DNS, cache, blocklist, metrics, and authentication logic that has accidentally become API-owned. Move only clearly misplaced logic.

## Acceptance criteria

- [ ] API modules remain resource-oriented.
- [ ] API handlers are thin.
- [ ] Persistence remains in `db/`.
- [ ] DNS/cache runtime behavior remains in runtime modules.
- [ ] HTTP validation/response mapping remains at the HTTP boundary.
- [ ] No generic service/repository/model layer was introduced.
- [ ] Relevant validation passes or CI evidence is recorded.
- [ ] Tracker is updated.
- [ ] Exactly one logical commit represents the phase.
