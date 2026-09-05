# MyDNS Production-Readiness Static Audit

**Audit target:** `production-readiness`  
**Audited commit:** `13b77db897641f139ece1d336fc8890abed9511d`  
**Scope:** source, tests, CI/release configuration, and the current authoritative DNS resolution architecture.

## Executive Summary

MyDNS has progressed materially since the previous audit. Several findings from the earlier audit are now addressed in the current branch and are intentionally removed from the active finding list:

- authoritative queries now bypass memory/persistent upstream caches
- the record index now distinguishes `Found`, `Nodata`, `Miss`, and `ServFail`
- external authoritative CNAME chains now return the authoritative CNAME instead of becoming NXDOMAIN
- cache persistence now stores each record using its actual RR type
- zone removal now deletes associated authoritative records transactionally
- `ANY` has explicit handling in the authoritative record index
- authoritative index lookup now preserves `AA` on the direct authoritative path

The branch is still **not production-ready**. The most important remaining problem is that the record index models **record existence**, while the authoritative server also needs to model **zone/owner existence and zone apex metadata**. This is directly relevant to the current `mydns.local` behavior: a zone can exist in the zone trie while the authoritative record index contains zero entries. In that state, a failed SOA/NS/other RR lookup can still be converted into NXDOMAIN even though the zone itself exists.

### Current finding summary

- **Critical:** 0
- **High:** 6
- **Medium:** 10
- **Low:** 3

### Release blockers

1. Correct authoritative DNS semantics for zone apex, owner-name existence, NODATA, NXDOMAIN, SOA, NS, and ANY.
2. Harden or disable the custom iterative recursive resolver until its DNS protocol/security gaps are closed.
3. Complete persistent-cache correctness and transactional/error-handled writes.
4. Complete lifecycle, configuration, platform deployment, and release packaging verification.
5. Produce actual stress/security/browser evidence rather than relying on source-level claims.

---

## Critical / High Findings

### [HIGH] Authoritative zone existence is not represented separately from record-index owner existence

**Location**

- `src/dns/handler.rs`
- `src/dns/record_index.rs`
- `src/dns/zone_trie.rs`

**Problem**

The authoritative path first determines whether the query falls inside a configured zone using `ZoneTrie`. The record index, however, only contains `(owner-name, RR-type)` entries loaded from `dns_records`.

`RecordIndex::name_exists()` therefore means "some DNS record exists for this owner name", not "this owner name exists in the authoritative zone".

A configured zone may legitimately exist with zero ordinary records. The current startup state can therefore be:

```text
ZoneTrie:
    mydns.local -> authoritative zone exists

RecordIndex:
    entry_count = 0
```

After an index miss, `processResolution()` still contains an authoritative fallback that returns NXDOMAIN.

**Failure scenario**

1. Create/configure authoritative zone `mydns.local`.
2. Do not create an ordinary `dns_records` row for the zone apex.
3. Query `mydns.local SOA`, `NS`, `MX`, `AAAA`, or another unsupported type.
4. Zone matching succeeds but the record index has no owner entry.
5. The handler can classify the result as authoritative NXDOMAIN.

**Impact**

Existing authoritative zones/owners can incorrectly produce NXDOMAIN instead of the required zone-specific answer or NODATA. This is the architectural explanation for the live SOA/NS failure seen with an authoritative zone whose record index is empty.

**Required behavior**

The resolver must distinguish:

```text
1. Does an authoritative zone cover the name?
2. Does the owner name exist in that zone?
3. Does the requested RR type exist at that owner?
```

Only step 2 = false should produce NXDOMAIN. Step 2 = true and step 3 = false produces NODATA, except for RR types that the zone model requires/synthesizes such as SOA/NS.

**Confidence**

Confirmed by current source/control-flow inspection and current runtime behavior.

**Recommended fix**

Make authoritative owner/zone metadata explicit. At minimum, zone apex existence and SOA/NS data must not depend on the presence of an arbitrary ordinary record in `RecordIndex`.

---

### [HIGH] Authoritative apex SOA and NS semantics are incomplete

**Location**

- `src/dns/handler.rs`
- `src/dns/record_index.rs`
- zone persistence/modeling

**Problem**

The current authoritative record index only loads `dns_records`. No separate authoritative-zone metadata path was found that guarantees an SOA and NS RRset for every configured zone apex.

The current handler's `querySpecialRecords()` only provides synthetic loopback PTR and dashboard A/AAAA behavior; it does not provide zone SOA/NS records.

**Failure scenario**

A configured zone exists but has no explicit SOA/NS rows. Queries for:

```text
mydns.local SOA
mydns.local NS
```

can fall through to the authoritative NXDOMAIN fallback.

**Impact**

The server can claim to host an authoritative zone while failing to provide the core apex metadata expected from an authoritative DNS zone.

**Required decision**

Define one authoritative zone model:

- either SOA and NS are mandatory records created/stored when a zone is created
- or MyDNS synthesizes them from zone metadata/configuration
- or the product explicitly supports incomplete test zones, in which case the response semantics and operational limitations must be documented

For a production authoritative zone, SOA and NS should be available at the apex.

**Confidence**

Confirmed that the current handler/index path does not synthesize them; exact intended product policy still needs to be finalized.

**Recommended fix**

Introduce explicit zone apex metadata and test SOA/NS over UDP and TCP, including restart behavior.

---

### [HIGH] Custom recursive resolver remains unsafe for production

**Location**

- `src/dns/upstream.rs`
- iterative resolution path

**Problem**

The custom iterative resolver still has protocol/security gaps including:

- UDP response identity/source validation requirements
- referral/glue bailiwick validation
- robust referral handling
- TCP fallback for truncated UDP responses
- complete IPv4/IPv6 nameserver resolution
- DNSSEC validation
- robust nameserver selection/failure handling

**Impact**

Recursive mode can be poisoned or fail on valid DNS deployments. This is a release blocker if recursive mode is advertised as production-capable.

**Confidence**

Confirmed by source inspection from the current branch; no evidence was found that these requirements have been fully closed.

**Recommended fix**

Either use a mature validating recursive resolver implementation or explicitly classify the custom recursive mode as experimental and disable it by default for production until the complete protocol/security test suite passes.

---

### [HIGH] Persistent cache writes are still non-transactional and persistence errors are ignored

**Location**

- `src/dns/handler.rs::saveToAllCaches`
- `src/db/records.rs::insert_cache`

**Problem**

The CNAME type-corruption issue is fixed: records are now persisted using `r.record_type()`.

However, `saveToAllCaches()` still inserts each returned record independently and discards the result with `let _ = ...`. There is no transaction spanning a complete multi-record answer.

**Failure scenario**

A CNAME response contains multiple records and a database error occurs during one of the inserts.

**Impact**

Only part of the answer can survive restart, with no surfaced persistence failure. This can make cache behavior differ before and after restart.

**Confidence**

Confirmed by current source inspection.

**Recommended fix**

Persist a complete answer atomically, report/count persistence failures, and add restart tests for CNAME + target records.

---

### [HIGH] Authoritative cache semantics are still insufficiently separated from recursive cache semantics

**Location**

- `src/dns/handler.rs`
- cache implementation

**Problem**

The current branch correctly skips memory and persistent upstream caches when the query is inside an authoritative zone. That closes the previous cache-precedence finding.

However, authoritative records are still inserted into the same in-memory cache with an `is_authoritative` flag, while NODATA and zone metadata are not represented equivalently. This makes cache behavior dependent on whether an authoritative result was represented as a positive RRset.

**Impact**

Future mutations, deletion/invalidation, NODATA handling, and special zone records can still diverge from ordinary authoritative semantics. The implementation needs an explicit rule that authoritative state is authoritative source data, not merely a cache entry carrying a boolean.

**Confidence**

Confirmed design risk; previous direct `AA` loss finding is considered fixed on the current positive authoritative path.

**Recommended fix**

Prefer treating the authoritative index/zone metadata as the source of truth and keeping recursive cache state separate. If authoritative results are cached for performance, define invalidation and authority semantics for positive, NODATA, SOA, NS, and ANY responses.

---

### [HIGH] DNS protocol regression coverage is incomplete for the authoritative RR matrix

**Location**

- `src/dns/record_index.rs` tests
- DNS integration tests

**Problem**

The record-index unit tests now cover NODATA classification for missing SOA/NS/other types and explicit CNAME behavior. This is progress, but unit tests do not prove the complete wire-level authoritative behavior of the running server.

The required matrix needs explicit integration coverage for at least:

```text
A       existing / missing
AAAA    existing / missing
CNAME   existing / missing
NS      apex answer
MX      existing / NODATA
TXT     existing / NODATA
SOA     apex answer
PTR     existing / NODATA
SRV     existing / NODATA
CAA     existing / NODATA
NAPTR   existing / NODATA
DS      delegation/DNSSEC policy
DNSKEY  DNSSEC enabled/disabled policy
RRSIG   DNSSEC enabled/disabled policy
HTTPS   existing / NODATA
SVCB    existing / NODATA
ANY     available RRsets / documented policy
```

Also required:

```text
existing owner + missing type  -> NOERROR + AA + zero answers
missing owner                  -> NXDOMAIN + AA
existing external CNAME        -> authoritative CNAME, not NXDOMAIN
zone apex SOA/NS               -> actual zone metadata
```

**Confidence**

Confirmed coverage gap. Live SOA/NS behavior demonstrates that the existing unit tests are not sufficient to validate the running architecture.

**Recommended fix**

Add wire-level UDP/TCP integration tests and a table-driven authoritative RR semantics suite.

---

## Medium Findings

### [MEDIUM] NODATA is still not negatively cached

**Location**

- `src/dns/handler.rs`

**Problem**

The handler now correctly distinguishes `IndexResolution::Nodata`, but NODATA responses from the upstream path are not persisted as negative cache entries. NXDOMAIN is handled separately.

**Impact**

Repeated upstream NODATA queries can repeatedly traverse the upstream resolver instead of using DNS negative caching semantics.

**Recommended fix**

Implement separate negative-cache representation for NXDOMAIN and NODATA, including appropriate SOA-derived negative TTL semantics where applicable.

---

### [MEDIUM] RD=0 behavior is still incorrect/underspecified

**Location**

- `src/dns/handler.rs`

**Problem**

For a non-authoritative miss with `RD=0`, the current handler returns NXDOMAIN rather than a documented non-recursive policy response.

**Impact**

NXDOMAIN asserts that the queried name does not exist; that is not equivalent to saying MyDNS will not perform recursion.

**Recommended fix**

Define the server's RD=0 policy and implement/test it consistently, normally using REFUSED when recursion is unavailable/not requested and the server is not authoritative.

---

### [MEDIUM] RA is advertised regardless of actual recursion capability/policy

**Location**

- `src/dns/handler.rs`

**Problem**

The response metadata sets `recursion_available = true` unconditionally.

**Impact**

Clients can be told that recursion is available even when the selected resolver mode, configuration, or failure state does not actually provide it.

**Recommended fix**

Set RA according to configured/available recursive capability and test combinations of authoritative, forwarding, recursive, and disabled-recursion modes.

---

### [MEDIUM] Settings persistence and resolver-mode restoration remain incomplete

**Location**

- settings API/startup restoration

**Problem**

The production-readiness requirements still call for persisted resolver mode restoration, atomic settings updates, and a clear operation for nullable router DNS. The current audit does not have evidence that all three have been completed.

**Recommended fix**

Verify the full settings transaction and restart path, then remove this finding only after an automated integration test proves persisted settings equal the active resolver state after restart.

---

### [MEDIUM] SQLite remains a single-connection bottleneck

**Location**

- database pool configuration

**Problem**

The application still uses a single SQLite connection for DNS persistence, API operations, migrations, and background maintenance.

**Impact**

Concurrent DNS/cache writes and management operations can queue behind one connection. This may be acceptable for a small local deployment but is not proven for the intended production load.

**Recommended fix**

Keep SQLite if it is a product requirement, but establish bounded concurrency and timeout targets through stress testing. Increase the pool only after measuring SQLite contention and transaction behavior.

---

### [MEDIUM] WebSocket reliability is not release-proven

**Location**

- WebSocket backend/frontend

**Problem**

The production-readiness requirements require reconnect, disconnect state, backpressure/slow-consumer handling, bounded history, and cleanup. The current audit has no CI evidence demonstrating all of these behaviors.

**Impact**

The management UI can silently stop receiving operational logs after a disconnect or broadcast-lag event.

**Recommended fix**

Add browser/integration tests for reconnect, slow consumers, server restart, network loss, logout cleanup, and bounded resource use.

---

### [MEDIUM] Required operational telemetry is not fully proven

**Location**

- DNS handler logging
- metrics layer
- dashboard API

**Problem**

The readiness requirements call for source, query, type, transport, resolution path, result, answer count, TTL, latency, upstream destination/health, and dashboard-ready aggregate metrics including P50/P95/P99.

The branch contains logging/metrics infrastructure, but the audit has no current executable evidence that every required field and aggregate is consistently populated under all resolution paths.

**Recommended fix**

Add deterministic telemetry tests and a dashboard API contract test. Treat metrics as backend authoritative state; the frontend should only render/format it.

---

### [MEDIUM] Release packaging and deployment evidence remains incomplete

**Location**

- `.github/workflows/release.yml`
- release/build scripts
- deployment documentation

**Problem**

The readiness requirements call for reproducible clean-checkout builds, versioned archives, SHA-256 checksums, installer/package verification, persistent data/config/log locations, health checks, deployment procedures, backup/restore, upgrade/rollback, and documentation matching the shipped artifacts.

Current source configuration does not provide sufficient evidence that the entire distribution contract is generated and verified automatically.

**Recommended fix**

Make release CI build the frontend from a clean checkout, produce the complete platform artifacts, generate checksums, inspect archive contents, and run installation/startup smoke tests.

---

### [MEDIUM] Cross-platform installation/configuration paths remain a release risk

**Location**

- configuration loading
- Windows installer
- platform-specific startup/discovery

**Problem**

The prior Windows installer/configuration mismatch remains a deployment concern unless an installed-layout smoke test proves that the installer-created configuration/data/log locations are actually discovered by the executable.

Linux/generic Unix also need explicit verification of configuration, database, log, privilege, and service paths.

**Recommended fix**

Add Windows installed-layout smoke testing and Linux systemd deployment testing. Do not consider a successful portable binary build equivalent to an installed deployment test.

---

### [MEDIUM] Unix privilege-drop lifecycle still needs deterministic verification

**Location**

- startup/server lifecycle

**Problem**

The previous startup race finding remains relevant until listener binding and privilege dropping are proven deterministic. DNS and HTTP listeners must both be available before privileges are reduced if either requires privileged ports.

**Impact**

Valid configurations can fail nondeterministically, and post-drop SQLite/log permissions can also affect runtime persistence.

**Recommended fix**

Add root/capability/non-root integration tests and verify listener binding, database writes, log writes, shutdown, and restart after privilege changes.

---

### [MEDIUM] Frontend production workflow lacks complete browser-level evidence

**Location**

- `frontend/`
- CI

**Problem**

The React + TypeScript + Vite implementation exists, but the V1 gate requires browser smoke coverage for login, dashboard, records, cache, settings, logs, WebSocket connectivity, session expiry, and error states.

**Recommended fix**

Add deterministic browser smoke tests to CI and include at least one clean production-style run against the Rust-served static frontend.

---

## Low Findings

### [LOW] Port zero is not explicitly prohibited for production deployment

Allowing port `0` can produce an ephemeral listener that is difficult to discover operationally. Reject it for production configuration or expose the actual bound address consistently.

### [LOW] Frontend failure-state consistency remains a quality gap

Some frontend failures still need explicit loading, empty, error, stale-data, reconnecting, and disconnected states across all management views.

### [LOW] Generic Unix gateway discovery is platform-specific

The gateway discovery implementation relies on Linux-specific route information in the Unix branch. Either provide per-OS implementations or clearly constrain automatic gateway discovery to supported platforms.

---

## Confirmed Improvements / Removed From Active Findings

The following previous audit findings are **no longer active findings on the current branch**, based on source inspection:

- **Authoritative cache precedence:** authoritative-zone queries now skip memory and persistent upstream caches before authoritative lookup.
- **External authoritative CNAME:** the index now returns the accumulated CNAME chain when the final target is not locally indexed, instead of turning the original query into NXDOMAIN.
- **CNAME persistent-cache record type:** `saveToAllCaches()` now persists each record using its actual `RData` record type.
- **Zone deletion database cleanup:** zone removal now deletes associated records in a transaction.
- **Record-index NODATA classification:** `IndexResolution::Nodata` is now explicitly implemented for an existing indexed owner with a missing RR type.
- **Authoritative positive AA preservation:** the current direct authoritative index path returns `Positive(..., true)` and stores the authority bit with the in-memory cache entry.
- **ANY index handling:** `RecordIndex` now has explicit ANY behavior rather than treating ANY as an ordinary RR type.

These are removed from the active severity counts, but the corresponding behaviors still require integration/regression verification where noted above.

---

## DNS Correctness Assessment

### Authoritative

**Improved but not complete.** The branch now has the correct conceptual `Found/Nodata/Miss/ServFail` split for indexed records and avoids upstream caches inside authoritative zones. The remaining architectural defect is that owner existence is inferred from ordinary record rows rather than from authoritative zone/owner metadata. This breaks empty-zone and apex semantics and can still produce NXDOMAIN incorrectly.

Required wire behavior:

```text
Existing owner + existing RR type  -> NOERROR + AA + answer
Existing owner + missing RR type   -> NOERROR + AA + zero answers (NODATA)
Missing owner                       -> NXDOMAIN + AA
Authoritative zone                 -> never forward upstream
Zone apex SOA/NS                   -> actual authoritative zone data
ANY                                -> documented available-RRset policy
```

### SOA / NS / zone apex

Not release-ready. The zone model must guarantee the apex SOA and NS semantics expected of an authoritative zone. They must not depend on an arbitrary `dns_records` row existing merely to make the owner appear in `RecordIndex`.

### CNAME

The previous external-CNAME failure is addressed in the index. Regression tests still need to prove:

- local CNAME + local target
- local CNAME + external target
- multi-hop chain
- loop → SERVFAIL
- CNAME query itself
- restart after persistent caching

### ANY

Explicit index support now exists. The product must still define and test the wire-level policy. ANY must not be interpreted as "every possible RR type"; it should return the available RRsets according to the server's documented policy.

### Recursive

Not production-safe until response validation, referral/bailiwick handling, TCP fallback, IPv6 nameserver resolution, DNSSEC policy, and stress/security tests are complete.

### Forwarding

Basic forwarding/failure handling exists, but settings persistence and non-recursive semantics require final verification.

### DNSSEC

Validation/signing behavior is not complete enough for a production DNSSEC claim. DS/DNSKEY/RRSIG behavior must be explicitly scoped and tested rather than implicitly treated as ordinary missing RR types.

---

## Persistence Assessment

### Positive controls

- Parameterized SQL queries.
- SQLite WAL configuration.
- Cache identity normalization/indexing.
- Transactional zone removal.
- Actual record type is now preserved for persisted upstream answers.

### Remaining risks

- Multi-record cache persistence is not atomic.
- Cache persistence errors are ignored.
- NODATA negative caching is not implemented.
- Authoritative zone metadata is not sufficiently separated from ordinary record rows.
- Settings persistence requires end-to-end restart verification.
- Platform-specific filesystem/privilege behavior remains under-tested.

---

## Concurrency / Async Assessment

### Positive controls

- Tokio cancellation/graceful-shutdown mechanisms exist.
- CNAME traversal is bounded and loop-detected.
- In-memory cache is protected by async locking.
- Cache entry limits exist.

### Remaining risks

- Single SQLite connection under concurrent load.
- Multi-record persistence is sequential/non-transactional.
- WebSocket slow-consumer behavior terminates connections without complete reconnect proof.
- Startup/privilege ordering is not release-proven.
- Settings mutation can still require atomicity verification.

---

## Security Assessment

### Positive controls

- Argon2 password hashing.
- JWT validation/expiration.
- Parameterized SQL.
- Request body limits.
- Security response headers.
- Unix supplemental-group handling during privilege dropping.

### Remaining concerns

- Custom recursive DNS poisoning/referral risks.
- Missing DNSSEC validation.
- Statistics exposure must be intentionally scoped/authenticated.
- CORS policy needs production verification.
- Reverse-proxy client attribution for login rate limiting needs explicit trusted-proxy policy.
- DNS amplification/resource behavior needs stress testing.
- Administrator password-strength policy remains a deployment concern.

No new SQL-injection or path-traversal defect was established by this audit.

---

## Test Coverage Gaps

The release suite should add or retain deterministic tests for:

### Authoritative DNS

- zone apex SOA answer
- zone apex NS answer
- empty authoritative zone behavior
- existing owner + missing RR type → NODATA
- missing owner → NXDOMAIN
- every supported RR type in the authoritative matrix
- ANY behavior
- CNAME local target
- CNAME external target
- CNAME loop → SERVFAIL
- authoritative UDP and TCP parity
- trailing-dot/case normalization
- zone/subdomain boundary matching
- no upstream access from authoritative zones
- authoritative behavior after cache population
- authoritative behavior after zone creation/deletion/restart

### Recursive DNS

- source/transaction/question validation
- referral and glue bailiwick
- truncated UDP → TCP fallback
- IPv4/IPv6 nameserver resolution
- nameserver failover
- recursion depth/loop behavior
- DNSSEC validation/policy
- malformed upstream responses

### Cache/persistence

- positive TTL expiry
- NODATA negative caching
- NXDOMAIN negative caching
- CNAME + target atomic persistence
- persistence error reporting
- restart/reload parity
- invalidation after record mutation
- invalidation after zone removal
- authoritative zone creation over pre-existing upstream cache data

### API/UI/lifecycle

- full protected-route authentication/authorization
- settings atomicity and restart
- WebSocket reconnect/backpressure/cleanup
- browser smoke tests
- graceful shutdown under active traffic
- startup failure propagation
- Unix privilege/capability combinations
- Windows installed-layout startup
- release artifact contents

---

## Current Quality Gate Status

The previous audit's historical local command results should not be treated as the current branch's status. The current branch was last advanced by commit `13b77db897641f139ece1d336fc8890abed9511d`.

Before declaring the branch release-ready, record fresh CI evidence for:

- `cargo fmt --check`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- frontend type-check/lint/build
- stress smoke profile
- `cargo audit`
- Linux build/test
- Windows build/test
- browser smoke tests
- release-profile build
- release artifact inspection

Do not copy old pass/fail claims into a new release decision without a current CI run.

---

## Release Priority

### P0 — release blockers

1. **Fix authoritative zone/owner semantics.** Separate zone existence, owner existence, and RR-type existence.
2. **Implement/define SOA and NS apex behavior.**
3. **Add authoritative wire-level RR matrix tests**, especially SOA, NS, NODATA, NXDOMAIN, CNAME, ANY, UDP/TCP.
4. **Harden or explicitly disable custom recursive mode** until its security/protocol requirements are satisfied.
5. **Complete current CI/security/stress evidence** rather than relying on historical results.

### P1 — required before production deployment

1. Make multi-record cache persistence transactional and observable.
2. Implement NODATA negative caching.
3. Correct/document RD=0 and RA behavior.
4. Complete settings atomicity/restart behavior.
5. Complete WebSocket reconnect/backpressure/browser verification.
6. Complete platform installation and privilege lifecycle testing.
7. Complete release packaging/checksum/installer/archive verification.
8. Complete dashboard/terminal telemetry verification.

### P2 — operational hardening

1. Measure SQLite concurrency and tune bounded pooling if justified.
2. Improve frontend failure states.
3. Finish generic Unix platform-specific discovery.
4. Expand DNSSEC support only if it is part of the supported product contract.

---

## Audit Conclusion

The previous audit was materially stale relative to the current `production-readiness` branch. Several of its highest-severity findings have been addressed and are removed from the active list.

The **current primary DNS defect is more fundamental**: the implementation has improved record-type classification, but authoritative DNS semantics require an explicit model of zone and owner existence plus zone-apex metadata. Until that is fixed and covered by wire-level tests, the authoritative resolver cannot be considered production-correct.

The branch should therefore remain **NOT PRODUCTION READY**.