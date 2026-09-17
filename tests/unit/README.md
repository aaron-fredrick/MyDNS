# Unit Tests

Unit tests should be deterministic and isolated from the running MyDNS process.

For Rust, prefer colocated `#[cfg(test)]` modules for true unit tests because they can exercise private implementation details. Do not put network, process, database-integration, or end-to-end tests here.

Frontend unit tests live under `tests/unit/frontend/` and use Node's built-in test runner against the TypeScript source. They cover pure frontend utilities and API-client behavior without requiring a browser or a running server.

Current frontend coverage includes:

- uptime formatting
- API request headers and bearer-token handling
- JSON mutation bodies
- authentication expiry/redirect behavior

Add component tests here when UI components gain behavior that cannot be adequately covered by pure utility/API tests.
