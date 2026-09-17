# Unit Tests

Reserved for small isolated tests that are useful outside the production module files.

For Rust, prefer colocated `#[cfg(test)]` modules for genuine unit tests because they can access private implementation details. Keep this directory for future black-box/unit-like helpers that do not belong in production modules.

Do not place integration, network, process, or end-to-end tests here.
