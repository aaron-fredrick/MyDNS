# Phase 6 — Final Structure Verification

## Objective

Perform the final architecture check and make only cleanup required to satisfy the documented rules. Do not redesign the backend again.

## Verify the tree

The tree should clearly communicate API, web, DNS, cache, database, configuration, state, observability, and privilege ownership.

## Verify naming

- folders represent subsystems/boundaries;
- files represent cohesive responsibilities;
- names are specific enough to explain ownership;
- generic dumping grounds such as `utils.rs`, `helpers.rs`, `manager.rs`, or `common.rs` were not introduced without a concrete reason.

## Verify dependencies

```text
API/Web → runtime/application → persistence/infrastructure
DNS    → runtime state/config/DB as required
DB     → must not depend on API/Web
Cache  → must not depend on API/Web
```

Presentation-specific types must not leak downward into persistence/runtime modules.

## Verify duplication

Search for unnecessary duplicate `DnsRecord`, zone, cache, and configuration representations. Keep multiple representations only where a real boundary requires them.

## Verify tests and docs

- Existing test organization remains intentional.
- Tests were not rewritten just to make the tree look different.
- No generated artifacts were added.
- `docs/project-structure.md` matches reality.
- No obsolete documentation contradicts the final tree.
- All tracker entries contain evidence.

## Acceptance criteria

- [ ] All six phases are DONE with evidence.
- [ ] Repository structure documentation is accurate.
- [ ] No unnecessary MVC-style layer exists.
- [ ] Relevant Rust validation has run or CI evidence is recorded.
- [ ] No unintended behavior changes were introduced.
- [ ] Final branch contains only intended structural work.

After this phase, future structure changes must be justified by a concrete ownership or maintenance problem.
