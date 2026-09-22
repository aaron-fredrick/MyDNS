# Phase 4 — DNS and Runtime Ownership Review

## Objective

Review the DNS/runtime boundaries after the first three structural phases. This is a verification/correction phase, not a reason to rewrite working DNS code.

## Preserve the current meaningful boundaries

- `dns/blocklist.rs` → runtime blocklist index/data structure.
- `dns/handler/blocklist.rs` → request-time blocklist resolution policy.
- `dns/record_index.rs` → runtime local record index.
- `dns/zone_trie.rs` → runtime zone ownership lookup.
- `dns/handler/local.rs` → local resolution behavior.
- `dns/handler/cache.rs` → DNS cache resolution policy.
- `dns/handler/records.rs` → generic DNS response/record construction.
- `dns/handler/upstream/forward.rs` → forwarding behavior.
- `dns/handler/upstream/recursive.rs` → iterative/raw recursive behavior.
- `dns/server.rs` → DNS listener lifecycle.
- `state/mod.rs` → shared runtime dependency container.

## Review for

- SQL/database persistence inside DNS modules.
- Axum-specific types inside DNS runtime code.
- configuration-file parsing inside DNS modules.
- business logic hidden in `AppState`.
- duplicated cache/blocklist/zone lookup logic.
- misleading names or ownership left over from previous refactors.

## AppState rule

Keep `AppState` as shared runtime state and subsystem handles. Do not split it simply because it is large.

## Acceptance criteria

- [ ] DNS runtime boundaries remain understandable.
- [ ] Runtime indexes are not mixed with request handlers.
- [ ] DNS does not depend on Axum presentation types.
- [ ] `AppState` contains state/handles rather than business logic.
- [ ] Any changes are small and behavior-preserving.
- [ ] Relevant validation passes or CI evidence is recorded.
- [ ] Tracker is updated.
- [ ] Exactly one logical commit represents the phase.
