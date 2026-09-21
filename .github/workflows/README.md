# CI — Local Verification

This directory contains the GitHub Actions workflows used to verify MyDNS. The commands below mirror the important CI gates so contributors can reproduce failures locally before opening a PR.

## Prerequisites

- Rust stable
- Node.js 22+
- npm
- `cargo-audit`
- `cargo-llvm-cov`
- NASM on Windows for AWS-LC-RS

Install the Rust tools once:

```powershell
cargo install cargo-audit
cargo install cargo-llvm-cov
```

## Fast local checks

Run from the repository root:

```powershell
npm ci
npm run typecheck
npm run build

cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo audit
```

## Frontend CI test

```powershell
npx --yes c8@10.1.3 --reporter=lcov --reporter=text --reports-dir=coverage/frontend node --experimental-strip-types --test tests/unit/frontend/*.test.ts
```

## Backend coverage

```powershell
cargo llvm-cov --all-targets --all-features --lcov --output-path lcov.info
```

To generate an HTML report:

```powershell
cargo llvm-cov --all-targets --all-features --html
```

The HTML report is written under `target/llvm-cov/html/`.

## Reproducing CI by job

### Frontend

```powershell
npm ci
npm run typecheck
npx --yes c8@10.1.3 --reporter=lcov --reporter=text --reports-dir=coverage/frontend node --experimental-strip-types --test tests/unit/frontend/*.test.ts
npm run build
```

### Rust format and Clippy

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Rust tests and coverage

```powershell
cargo llvm-cov --all-targets --all-features --lcov --output-path lcov.info
```

### Release build

```powershell
npm ci
npm run build
cargo build --release --verbose
```

## Security audit

```powershell
cargo audit
npm audit --audit-level=high
```

Run the Rust audit against the committed dependency graph. Do not rely on CI to silently regenerate `Cargo.lock`.

If `Cargo.toml` dependency features or versions change, regenerate the lockfile locally and commit the resulting change:

```powershell
cargo generate-lockfile
cargo check --locked
cargo audit
```

Review the resulting dependency graph before committing it, especially when a RustSec advisory is involved.

## Lockfile verification

CI should use `--locked` where practical so the dependency graph tested locally matches the committed `Cargo.lock`.

When a lockfile change is intentional:

```powershell
cargo generate-lockfile
git diff -- Cargo.toml Cargo.lock
cargo check --locked
cargo audit
```

## Full pre-PR sequence

```powershell
npm ci
npm run typecheck
npm run build

cargo fmt --all -- --check
cargo check --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo audit
npm audit --audit-level=high

cargo llvm-cov --all-targets --all-features --lcov --output-path lcov.info
cargo build --release --verbose
```

For DNS, caching, persistence, authentication, WebSocket, or shutdown changes, also run the relevant integration/stress tests documented under `tests/`, `stress-tests/`, and `docs/`.

## CI workflow files

- `ci.yml` — frontend quality, Rust formatting/Clippy, tests/coverage, and release builds.
- `security.yml` — RustSec and npm dependency audits.
- `codeql.yml` — CodeQL analysis.
- `release.yml` — release packaging/build automation.

The workflow definitions are the source of truth if a command in this README changes.
## Test pipeline

The test capability workflows are ordered by policy as Unit -> Component -> Contract -> Integration -> Smoke -> E2E. E2E is enabled by the dev/main/nightly policies and is not required by the normal pull-request policy. Load and fuzz workflows are manual placeholders only.
