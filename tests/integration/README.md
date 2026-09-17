# Integration Tests

Cross-module tests that validate subsystem contracts without requiring a fully installed product.

Planned coverage: DNS resolution, Local DNS, blocklist, cache, SQLite persistence, management API, authentication/authorization, upstream resolution, and observability contracts.

Keep shared harness code in `tests/support/` as the structure is migrated.
