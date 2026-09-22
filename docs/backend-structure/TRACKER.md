# Backend Structure Refactor Tracker

This is the execution tracker for `docs/backend-structure/`.

A phase is complete only when its acceptance criteria are satisfied, validation evidence is recorded, and the implementation commit is recorded.

Allowed statuses: `TODO`, `IN PROGRESS`, `BLOCKED`, `DONE`.

| Phase | Status | Commit | Validation | Notes |
|---|---|---|---|---|
| 1 — Database boundaries | TODO | — | — | Split persistence ownership |
| 2 — Configuration boundaries | TODO | — | — | Separate types from format parsing |
| 3 — Web server boundaries | TODO | — | — | Separate lifecycle/routes/frontend serving |
| 4 — DNS/runtime ownership | TODO | — | — | Verify runtime ownership without unnecessary rewrite |
| 5 — API/application boundary | TODO | — | — | Keep API thin |
| 6 — Final verification | TODO | — | — | Final tree/dependency/naming/documentation check |

## Agent handoff record

### Phase 1
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

### Phase 2
- Status: TODO
- Started: —
- Completed: —
- Commit: —
- Validation: —
- Notes: —

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
