# MyDNS Production-Readiness Static Audit

## Executive Summary

MyDNS has a solid prototype foundation, but it is **not production-ready yet**.

Findings:

- **Critical:** 0
- **High:** 9
- **Medium:** 13
- **Low:** 2

The largest blockers are:

- insecure/incomplete custom recursive DNS resolution
- authoritative-zone and cache inconsistencies
- persistent CNAME cache corruption
- zone deletion leaving records active
- Unix privilege/startup races
- Windows installer configuration failure
- incomplete release packaging

Windows and Linux both have production-impacting issues. Generic Unix support is weaker than Linux because several implementations depend on Linux-specific files.

---
x
## Critical / High Findings

### [HIGH] Recursive resolver accepts unvalidated UDP responses

**Location**

- `upstream.rs:85`
- `raw_dns_query`

**Problem**

The resolver sends a UDP query and accepts the first datagram returned by `recv_from`. It does not validate:

- sender address
- DNS transaction ID
- question name
- question type
- opcode
- response/request relationship

**Failure scenario**

In recursive mode, an attacker or compromised network device sends a forged UDP response before the real root/TLD response arrives.

**Impact**

Recursive queries can return forged records, false NXDOMAIN responses, or attacker-selected delegation data.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Validate source address, transaction ID, question section, opcode, and relevant response flags. Prefer Hickory’s validated resolver transport instead of the custom UDP path.

---
x
### [HIGH] Recursive glue records are not bailiwick-validated

**Location**

- `upstream.rs:405`
- `resolve_iterative`

**Problem**

Every A/AAAA record in the additional section is treated as a next-hop nameserver. The implementation does not check whether the glue belongs to the delegated nameserver domain.

**Failure scenario**

A referral contains unrelated or malicious additional-section addresses.

**Impact**

The resolver may send subsequent queries to an attacker-controlled server.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Implement delegation and bailiwick validation. Ignore unrelated additional records and validate nameserver ownership before using glue.

---
x //DNSSEC validation is not implemented
### [HIGH] Recursive mode is incomplete for production DNS

**Location**

- `upstream.rs:272`

**Problem**

The custom iterative resolver lacks several production DNS requirements:

- no TCP fallback when the UDP response is truncated
- no DNSSEC validation
- only A records are used when resolving unglued nameservers
- no robust referral validation
- limited nameserver selection
- fixed ten-level recursion limit
- incomplete IPv6 nameserver support
- no response authentication

**Failure scenario**

A domain uses an IPv6-only nameserver, returns a truncated response, requires DNSSEC validation, or provides a complex delegation.

**Impact**

Valid domains can return SERVFAIL or incorrect data. Recursive mode is also vulnerable to poisoning.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Use a mature recursive resolver implementation, or substantially harden this code before advertising recursive mode as production-capable.

---
x
### [HIGH] Authoritative external CNAMEs can incorrectly return NXDOMAIN

**Location**

- `record_index.rs:120`
- `handler.rs:137`

**Problem**

The record index follows a CNAME target. If the target is outside the local record index, it returns `Miss`. The handler then sees that the original name belongs to an authoritative zone and returns authoritative NXDOMAIN.

**Failure scenario**

Configured zone:

```text
alias.example.com CNAME external.example.net
```

A query for `alias.example.com A` is made.

**Impact**

The valid CNAME is rejected as NXDOMAIN instead of returning the CNAME answer.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by control-flow analysis.

**Recommended fix**

Return the authoritative CNAME record even when the target is not locally available. Only append target records when they are known.

---
x
### [HIGH] Persistent upstream CNAME answers are stored under the wrong type

**Location**

- `handler.rs:426`
- `saveToAllCaches`

**Problem**

Every returned record is persisted using the original query type instead of the actual record type.

For an A query returning:

```text
CNAME + A
```

both records can be stored as type `A`.

**Failure scenario**

An upstream CNAME response is cached, the process restarts, and the persistent cache is reloaded.

**Impact**

The CNAME record is lost or cannot be parsed as an A/AAAA record. CNAME behavior differs before and after restart.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Persist each record using its actual record type and preserve complete answer chains transactionally.

---
x
### [HIGH] Removing a zone leaves its records actively served

**Location**

- `zones_api.rs:111`
- `handler.rs:318`

**Problem**

Zone deletion removes only the `zones` table row and rebuilds the zone trie. Existing DNS records remain in SQLite and the in-memory record index.

The record index lookup itself does not require the name to still belong to an active zone.

**Failure scenario**

1. Add `example.local` as a zone.
2. Add `host.example.local A ...`.
3. Remove the zone.
4. Query `host.example.local`.

**Impact**

The record can continue being served immediately and after restart, despite the zone being removed.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Define zone-removal semantics. Either delete associated records and invalidate cache entries, or make record serving require active zone ownership.

---
x
### [HIGH] Cached data can bypass authoritative-zone enforcement

**Location**

- `handler.rs:133`

**Problem**

Memory and persistent cache lookup occur before the authoritative-zone check.

**Failure scenario**

1. An upstream answer for `host.example.com` is cached.
2. `example.com` is later added as an authoritative zone.
3. The same name is queried again.

**Impact**

The cached upstream answer can be returned instead of the authoritative result. This violates the documented rule that authoritative zones must never forward or use upstream data.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Check active zone ownership before reading recursive/upstream caches. Invalidate affected cache entries whenever zones change.

---
>
### [HIGH] Authoritative answers lose the `AA` flag after caching

**Location**

- `handler.rs:203`
- `handler.rs:345`

**Problem**

An authoritative record-index result is cached in the ordinary memory cache. The initial result is marked authoritative, but a later cache hit returns `Positive(..., false)`.

**Failure scenario**

Two consecutive queries request the same authoritative record.

**Impact**

The first response has `AA=1`; later responses incorrectly have `AA=0`.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Separate authoritative records from recursive cache entries, or store origin/authority metadata in cache entries and preserve it when constructing responses.

---

### [HIGH] Unix privileged HTTP binding races with privilege dropping

**Location**

- `main.rs:143`
- `server.rs:30`

**Problem**

DNS and HTTP startup tasks run concurrently. DNS binds sockets and drops privileges before HTTP necessarily binds.

**Failure scenario**

Both DNS and HTTP use ports below 1024. DNS starts first and drops from root to `nobody`; HTTP then fails to bind.

**Impact**

Startup becomes nondeterministic and valid configurations can fail.

**Platform**

Linux and generic Unix

**Confidence**

Confirmed by control-flow analysis.

**Recommended fix**

Bind all listeners before dropping privileges, or coordinate startup so privilege dropping occurs only after both servers have successfully bound.

---

### [HIGH] Windows installed configuration is not discovered

**Location**

- `mod.rs:183`
- `main.rs:22`
- `mydns-windows-x64.iss:17`

**Problem**

The installer creates:

```text
%ProgramData%\MyDNS\config\mydns.toml
```

The application only searches the current working directory for:

```text
config.toml
config.ini
```

**Failure scenario**

The user launches the installed executable from `Program Files`.

**Impact**

Startup fails with “Configuration file not found.” Logs and database paths also resolve relative to the working directory instead of the documented ProgramData directories.

**Platform**

Windows

**Confidence**

Confirmed by source and installer comparison.

**Recommended fix**

Use platform-aware paths, install the expected filename, or support an explicit configuration path passed by a launcher/service.

---

## Medium Findings

### [MEDIUM] Resolver mode is persisted but not restored

**Location**

- `settings_api.rs:54`
- `main.rs:69`

**Problem**

The settings API stores `resolver_mode` in SQLite, but startup reloads only resolver priority, Cloudflare DNS, and router DNS.

**Failure scenario**

The UI changes from forwarding to recursive, then the process restarts.

**Impact**

The application returns to the mode in the configuration file, not the mode saved through the dashboard.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Reload and validate `resolver_mode` during startup, or clearly make the setting configuration-only.

---

### [MEDIUM] NODATA is not negatively cached

**Location**

- `handler.rs:160`

**Problem**

NXDOMAIN responses are persisted as negative cache entries, but NODATA responses are returned without caching.

**Failure scenario**

A valid name exists but has no record of the requested type. The same query is repeated frequently.

**Impact**

Every request repeatedly contacts upstream DNS, contrary to the documented positive/negative caching behavior.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Cache NXDOMAIN and NODATA separately, preserving response-code semantics and appropriate SOA TTLs.

---

### [MEDIUM] Non-recursive clients receive NXDOMAIN incorrectly

**Location**

- `handler.rs:162`

**Problem**

When `RD=0` and the name is not local, the server returns NXDOMAIN.

**Failure scenario**

A validating resolver or client sends a non-recursive query for a name the server does not know.

**Impact**

The client may interpret the response as proof that the domain does not exist. REFUSED or another policy response would be more accurate.

**Platform**

Windows, Linux, Unix

**Confidence**

Strongly supported.

**Recommended fix**

Return REFUSED for unsupported recursion requests, or document and test the chosen policy.

---

### [MEDIUM] SQLite single connection serializes all work

**Location**

- `mod.rs:17`

**Problem**

The global pool is configured with `max_connections(1)`. DNS cache persistence, API writes, pruning, migrations, and reads all queue behind the same connection.

**Failure scenario**

Many simultaneous upstream misses trigger cache writes while API CRUD and pruning are active.

**Impact**

Requests can queue for seconds and hit the five-second busy timeout. No stress test proves acceptable behavior.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed design risk.

**Recommended fix**

Benchmark realistic load and consider a small bounded pool with explicit timeouts and transactions.

---

### [MEDIUM] Multi-record cache writes are partial and silently lose errors

**Location**

- `handler.rs:426`

**Problem**

Each answer record is written separately, and each database error is discarded with `let _ =`.

**Failure scenario**

The database becomes locked or the process loses write access halfway through a CNAME/A response.

**Impact**

Only part of the response is persisted. After restart, the cache can contain incomplete data without an obvious error.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Use a transaction for the complete answer and report or count persistence failures.

---

### [MEDIUM] Settings updates are not atomic

**Location**

- `settings_api.rs:54`

**Problem**

Settings are changed in live memory and persisted one field at a time. Resolver reconstruction happens after those mutations.

**Failure scenario**

The first setting saves successfully, a later database write or resolver rebuild fails.

**Impact**

Live configuration, persisted configuration, and active resolver state can disagree.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Parse all values first, construct the new resolver, persist all values in one transaction, then swap state.

---

### [MEDIUM] Router DNS cannot be cleared through the API

**Location**

- `settings_api.rs:76`
- `Settings.tsx:66`

**Problem**

An empty router DNS field is converted to `null` by the frontend, but the backend only handles `Some(String)`. There is no explicit clear operation.

**Failure scenario**

A previously configured router DNS should be removed.

**Impact**

The old router address remains active and persisted.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Use an explicit nullable request field and delete the persisted setting when null is supplied.

---

### [MEDIUM] Statistics are unauthenticated

**Location**

- `server.rs:39`
- `stats_api.rs:8`

**Problem**

`/api/v1/stats` does not require JWT authentication.

**Failure scenario**

The HTTP server is bound to a non-loopback address.

**Impact**

Unauthenticated users can observe uptime, query rates, resolver health, cache metrics, and record counts.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Require authentication or provide a deliberately minimal public health endpoint.

---

### [MEDIUM] Debug builds enable permissive CORS unconditionally

**Location**

- `server.rs:155`

**Problem**

Any debug build uses permissive CORS, even when configured on an external interface.

**Failure scenario**

A debug binary is deployed temporarily on a shared or public host.

**Impact**

Arbitrary origins can make browser requests to the API using available credentials.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Require explicit opt-in for permissive CORS and retain loopback-only development defaults.

---

### [MEDIUM] Login rate limiting breaks behind reverse proxies

**Location**

- `auth.rs:91`

**Problem**

Rate limiting uses the direct TCP peer address. Behind nginx or Caddy, all clients appear to come from the proxy.

**Failure scenario**

One client submits five failed logins through the reverse proxy.

**Impact**

The shared proxy address becomes rate-limited for all users.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Rate-limit at the trusted proxy, or process forwarding headers only from explicitly trusted proxy addresses.

---

### [MEDIUM] Generic Unix gateway discovery has no implementation

**Location**

- `upstream.rs:38`

**Problem**

The Unix implementation reads `/proc/net/route`, which is Linux-specific. Non-Linux Unix builds fall back to the same Unix branch but generally lack that file.

**Failure scenario**

MyDNS runs on a Unix-like system without Linux `/proc`.

**Impact**

Automatic router DNS discovery fails and forwarding falls back to Cloudflare only.

**Platform**

Linux works conditionally; generic Unix is incomplete.

**Confidence**

Confirmed by source inspection.

**Recommended fix**

Use a platform-specific route API or make router DNS configuration mandatory outside Linux.

---

### [MEDIUM] WebSocket disconnects silently stop live logs

**Location**

- `ws.rs:28`
- `Logs.tsx:7`

**Problem**

The frontend has no reconnect/backoff or visible disconnected state. The backend closes the connection on broadcast lag or channel errors.

**Failure scenario**

The browser sleeps, the network changes, or a client falls behind the broadcast channel.

**Impact**

Operators see stale logs without knowing the connection is dead.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed.

**Recommended fix**

Add connection state, retry/backoff, user-visible errors, and tests for lag, reconnect, and shutdown.

---

### [MEDIUM] Release script depends on stale prebuilt frontend assets

**Location**

- `build-release.ps1:17`
- `vite.config.ts:11`

**Problem**

The release script builds Rust targets but does not run the frontend build. Rust embeds whatever happens to already exist under `web`.

**Failure scenario**

A clean checkout or stale `web` directory is used for a release build.

**Impact**

The binary can contain no dashboard or an outdated dashboard.

**Platform**

Windows release process

**Confidence**

Confirmed.

**Recommended fix**

Build and verify `index.html` before compiling the Rust binary.

---

### [MEDIUM] Release workflow does not produce the documented deployment packages

**Location**

- `release.yml:54`
- `v1-distribution.md:40`

**Problem**

The release workflow uploads binaries but does not produce the documented portable archives, installers, checksums, configuration files, or deployment documentation.

**Failure scenario**

A user downloads a release artifact expecting the documented portable layout.

**Impact**

The result is not a complete operational distribution.

**Platform**

Windows and Linux

**Confidence**

Confirmed.

**Recommended fix**

Build explicit archives and installer artifacts, include example configuration and documentation, and verify their contents in CI.

---

### [MEDIUM] Required operational telemetry is incomplete

**Location**

- `handler.rs:54`
- `metrics_handler.rs:29`

**Problem**

The documented telemetry requires transport, TX events, answer count, TTL, latency, and full resolution path. The current logs do not consistently provide all of these fields.

**Impact**

Operators cannot reliably reconstruct DNS failures from logs.

**Platform**

Windows, Linux, Unix

**Confidence**

Confirmed against the production-readiness documentation.

**Recommended fix**

Emit structured RX/TX events with transport, request ID, source, query, resolution path, result, answer count, TTL, latency, and error details.

---

## Low Findings

### [LOW] Configuration accepts port zero

**Location**

- `mod.rs:258`

**Problem**

Port `0` is accepted. The OS chooses an ephemeral port, but the service logs the configured value rather than reliably exposing the selected port.

**Impact**

The DNS or HTTP service becomes difficult to discover and operate.

**Platform**

Windows, Linux, Unix

**Recommended fix**

Reject port zero for production configuration or log and expose the actual bound address.

---

### [LOW] Frontend error handling is inconsistent

**Location**

- `Settings.tsx:11`
- `Zones.tsx:12`

**Problem**

Several initial-load failures are sent only to `console.error`, and some promises are not explicitly handled.

**Impact**

The UI can appear empty or stale instead of showing an actionable failure state.

**Platform**

Windows, Linux, Unix

**Recommended fix**

Add consistent loading, error, disconnected, stale-data, and retry states.

---

## Platform Comparison

| Area | Windows | Linux | Unix | Assessment |
|---|---|---|---|---|
| Privileged ports | Requires elevated token for configured privileged ports | Root or `CAP_NET_BIND_SERVICE` supported | Root-only fallback | Basic model works, but Unix startup race remains |
| Privilege dropping | Not supported | `setresuid`/`setresgid` used | Same APIs assumed | Database/log permissions can break after dropping |
| Configuration paths | Installer path does not match lookup | Relative working-directory paths | Relative working-directory paths | Major deployment problem |
| Gateway discovery | Parses English `ipconfig` output | Reads `/proc/net/route` | Linux-specific implementation | Platform behavior is not equivalent |
| DNS sockets | UDP and TCP bind paths exist | UDP and TCP bind paths exist | Same code path | Basic transport support is present |
| Shutdown signals | Ctrl+C only | SIGINT/SIGTERM | SIGINT/SIGTERM | Reasonable, but service lifecycle is incomplete |
| Release packaging | Installer and portable layout incomplete | Portable package workflow incomplete | Not separately packaged | Not production-complete |

---

## DNS Correctness Assessment

- **Authoritative resolution:** Partially correct, but zone removal, cache precedence, external CNAMEs, and `AA` handling are broken.
- **Recursive resolution:** Not production-safe due to response spoofing, missing bailiwick validation, incomplete referral handling, no TCP fallback, and no DNSSEC.
- **Forwarding:** Basic forwarding and fallback logic works, but configuration persistence is incomplete.
- **Caching:** Basic TTL and persistence work, but origin, authority, NODATA, and CNAME semantics are incorrect.
- **NXDOMAIN:** Locally generated NXDOMAIN behavior exists, but non-recursive misses and external CNAMEs can be wrong.
- **NODATA:** Distinguished in some paths, but not negatively cached.
- **AA:** Incorrectly lost after authoritative results enter the memory cache.
- **RD/RA:** RD is read, but RD=0 misses return NXDOMAIN. RA is always advertised.
- **UDP:** Basic serving works.
- **TCP:** Basic TCP serving works and is tested.
- **Upstream failures:** Forwarding timeout/error handling is present.
- **Timeouts:** Forwarding and raw recursive queries have bounds, but recursive referral behavior is incomplete.
- **IPv4/IPv6:** Local A/AAAA handling exists, but recursive nameserver discovery is incomplete for IPv6.

---

## Persistence Assessment

SQLite schema and parameterized queries are generally sound. WAL mode and cache identity indexes are useful.

Main persistence risks:

- single-connection serialization
- multi-record cache writes are not transactional
- database errors during cache persistence are ignored
- CNAME records are persisted using the query type
- Unix privilege dropping can prevent future WAL/SHM writes
- installed Windows paths do not match documented data locations
- settings are persisted one field at a time
- resolver mode is not restored after restart

The Windows absolute-path SQLite test passed, so no standalone Windows SQLite URL defect is confirmed.

---

## Concurrency / Async Assessment

Positive controls:

- Tokio cancellation tokens are used.
- HTTP shutdown uses graceful shutdown.
- CNAME recursion is bounded.
- The in-memory cache is protected by an async `RwLock`.
- The cache has an entry cap.

Risks:

- DNS and HTTP startup race during privilege dropping.
- A single SQLite connection serializes unrelated operations.
- Database writes occur serially for multi-record answers.
- WebSocket broadcast lag terminates clients.
- Background cache pruning logs database failures and continues.
- Settings updates can leave live and persistent state inconsistent.
- DNS and HTTP task errors cancel the other side but do not clearly propagate failure as a process-level startup failure.

---

## Security Assessment

Positive controls:

- Argon2 password hashing.
- JWT signature and expiration validation.
- Parameterized SQL queries.
- Request body limit.
- CSP, `nosniff`, frame denial, and referrer policy headers.
- Unix supplemental groups are cleared during privilege dropping.

Production concerns:

- Recursive UDP response poisoning.
- Missing glue bailiwick validation.
- Public unauthenticated statistics.
- Permissive debug CORS.
- Reverse-proxy rate-limit attribution.
- DNS recursion amplification and resource behavior need stress testing.
- Default configuration requires an administrator password but does not enforce password strength.

No obvious SQL injection or path traversal issue was found.

---

## Startup / Shutdown Assessment

Startup order is generally:

1. logging
2. configuration
3. database/migrations
4. settings restoration
5. authentication seed
6. resolver construction
7. zone/index loading
8. background tasks
9. DNS and HTTP tasks

Problems:

- HTTP may start after DNS has already dropped privileges.
- DNS failure cancels HTTP, but failure propagation is mostly logged rather than returned.
- Background task failures do not necessarily terminate the application.
- Database/log flush behavior during process termination is not explicitly coordinated.
- Windows installer does not configure a service lifecycle.
- Unix file ownership after privilege changes is not guaranteed.

---

## Configuration Assessment

The parser correctly handles TOML defaults and required credentials, but:

- configuration is resolved relative to the current directory
- installed Windows configuration is not found
- resolver mode settings are saved but not restored
- router DNS cannot be cleared
- settings updates are not transactional
- ports are not operationally validated
- CORS domain normalization is incomplete
- runtime settings do not cover all configuration fields
- no explicit command-line or environment override exists for deployment paths

---

## Test Coverage Gaps

Important missing tests include:

- recursive response transaction-ID/source validation
- bailiwick and referral poisoning
- recursive TCP fallback and truncation
- DNSSEC behavior
- IPv6-only nameserver resolution
- external authoritative CNAMEs
- persistent CNAME cache restart behavior
- NODATA negative caching
- zone deletion and immediate/restart behavior
- cached data versus newly-created authoritative zones
- `AA`, `RA`, and `RD=0` semantics
- privileged startup ordering on Unix
- root/non-root/capability combinations
- SQLite writes after privilege dropping
- installed Windows layout startup
- localized Windows gateway output
- WebSocket reconnect and lag handling
- concurrent API/SQLite stress
- graceful shutdown during active traffic
- release archive and installer contents
- full protected-route authentication coverage

Current executable checks performed earlier:

- `cargo check --all-targets --all-features`: passed
- `cargo test --all-targets --all-features`: passed, 93 tests
- frontend production build: passed
- strict Clippy: failed on an unused import
- Rust formatting check: failed on formatting drift

---

## Non-Issues / Checked Correctly

- Windows absolute SQLite paths worked in the existing Windows integration tests.
- SQL queries use bound parameters.
- Password hashing and JWT expiry validation are implemented.
- Basic UDP and TCP DNS serving is covered by tests.
- Basic CNAME loop detection is bounded.
- Basic positive cache expiration works.
- Cache persistence across restart works for simple positive and negative entries.
- Zone matching uses label boundaries and lowercase normalization.
- HTTP request bodies are limited.
- Basic API authentication works for the tested routes.
- No macOS-specific deployment findings were included.

---

## Recommended Priority

1. **Fix immediately**
   - Harden or disable custom recursive mode.
   - Fix authoritative-zone/cache ordering.
   - Preserve authoritative metadata and correct CNAME persistence.
   - Fix zone deletion behavior.
   - Fix Windows configuration/data/log path handling.

2. **Fix before production**
   - Coordinate Unix listener binding and privilege dropping.
   - Add transactional cache/settings writes.
   - Implement NODATA negative caching.
   - Correct RD/RA/NXDOMAIN semantics.
   - Protect statistics and fix CORS/rate-limit behavior.

3. **Fix before wider deployment**
   - Add stress, restart, privilege, WebSocket, and platform tests.
   - Build complete release archives and installer artifacts.
   - Improve frontend failure states.

4. **Improve later**
   - Historical metrics/time buckets.
   - More efficient bounded SQLite pooling.
   - Expanded record-type support and DNSSEC if required.

5. **Documentation and CI**
   - Update the production-readiness document to reflect the actual failing Clippy/format gates.
   - Add installer smoke tests and release artifact verification.