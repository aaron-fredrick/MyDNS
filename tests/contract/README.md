# Contract tests

Contract tests verify that the frontend API client and Rust HTTP API keep their externally visible route surface compatible.

The current check is intentionally lightweight: it extracts frontend `/api/v1/...` paths and verifies that corresponding Axum routes exist in the backend. This is a foundation for a stronger schema-based contract later.
