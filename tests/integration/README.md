# Integration Tests

Cross-module tests that validate subsystem contracts without requiring a fully installed product.

Current targets cover:

- DNS UDP/TCP wire behavior
- authoritative zone semantics and apex SOA/NS
- Local DNS resolution
- blocklist enforcement and cache/blocklist precedence
- persistent cache lifecycle
- management API CRUD and stats
- authentication/authorization
- record validation
- upstream NXDOMAIN/SERVFAIL/timeout behavior

The Rust test targets are explicitly registered in `Cargo.toml` so they can remain inside this category directory while still participating in normal Cargo and coverage runs.
