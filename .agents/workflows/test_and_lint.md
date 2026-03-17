---
description: Workflows for testing and linting MyDNS
---

# Testing and Linting Workflow

This project follows strict coding standards (camelCase for functions, snake_case for variables) and mandates Command Query Separation (CQS). 

## 1. Running Tests
To verify DNS resolution, caching, and API integrity, run all tests:

```powershell
# Run all unit and integration tests
cargo test
```

### Specific Test Modules
- **Unit (Cache):** `cargo test --lib cache::tests`
- **Unit (DNS):** `cargo test --lib dns::tests`
- **Integration:** `cargo test --test integration`

## 2. Linting and Formatting
To ensure code quality and adherence to global rules:

```powershell
# Run clippy for static analysis
cargo clippy -- -D warnings

# Format code according to rustfmt.toml
cargo fmt
```

## 3. Pre-Commit Checklist
Before committing any changes, ensure:
1. All functions are in `camelCase`.
2. `#[allow(non_snake_case)]` is used only where explicitly required by global rules (e.g., for `camelCase` function names in Rust).
3. `cargo check` passes without warnings.
4. All unit tests in `src/dns/tests.rs` and `src/cache/tests.rs` pass.
5. Integration tests pass against a local database.

// turbo
## 4. Automatic Verification
You can run the full suite with:
```powershell
cargo test && cargo clippy -- -D warnings
```
