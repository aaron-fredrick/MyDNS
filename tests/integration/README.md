# Integration Tests

Cross-module tests that exercise MyDNS subsystems together without requiring the complete installed product workflow.

Planned areas:

- DNS resolver and local-DNS behavior
- blocklist persistence and runtime behavior
- cache and persistence
- HTTP API and authentication
- upstream resolution
- database/repository contracts

Existing top-level Rust integration targets should remain where Cargo currently discovers them. This directory is the destination structure for future migration once test targets are explicitly wired.
