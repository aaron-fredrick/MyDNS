# Unit Tests

Reserved for isolated black-box/unit-like tests that do not require a running MyDNS process.

For Rust, prefer colocated `#[cfg(test)]` modules for true unit tests because they can exercise private implementation details. Do not put network, process, database-integration, or end-to-end tests here.
