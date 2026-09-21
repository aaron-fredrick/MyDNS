<div align="center">

# ⬡ MyDNS

[![CI status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/ci.yml/badge.svg)](https://github.com/aaron-fredrick/MyDNS/actions/workflows/ci.yml)
[![CodeQL status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/codeql.yml/badge.svg)](https://github.com/aaron-fredrick/MyDNS/actions/workflows/codeql.yml)
![Version](https://img.shields.io/badge/version-v0.1.1--dev-blue?style=flat-square)

</div>

**MyDNS** is a Rust-based DNS server and management service built around Local DNS, domain blocking, upstream resolution, persistent caching, and a web dashboard.

The project is currently being prepared for its first proper production release: **V1.0.0**. The V1 scope is deliberately focused on finishing and hardening the existing product rather than expanding it indefinitely.

> **Status:** V1 production-readiness work in progress. The `dev` branch is the active release-hardening branch.

## What MyDNS Provides

### DNS server

- UDP and TCP DNS serving.
- Local DNS records including A, AAAA, CNAME, MX, NS, PTR, and TXT.
- Correct handling of NXDOMAIN, NODATA, and SERVFAIL responses.
- CNAME-chain resolution with loop detection.
- Configurable upstream resolver behaviour.

### Persistent cache

- Positive and negative DNS caching.
- Cache persistence across application restarts.
- Record-level ownership of cached responses.
- Expiration and pruning of stale entries.
- Cache invalidation when dependent records change.

### Management API and dashboard

- Authenticated DNS record management.
- JWT-based authentication with Argon2 password hashing.
- Runtime statistics and cache visibility.
- WebSocket support for live dashboard updates.
- The UI is implemented as a React + TypeScript + Vite application under `src/frontend/`; Vite builds static assets into `out/web/`, which Rust embeds and serves at runtime.

### Security and reliability

- Input and DNS record validation.
- Zone/ownership validation support.
- Security response headers.
- Request body limits.
- Unix privilege-dropping support.
- Graceful shutdown and OS signal handling.
- Structured application logging and audit logging.

## Architecture

MyDNS is intentionally split into clear backend responsibilities:

```text
MyDNS/
├── src/                 # Rust backend
│   ├── cache/           # DNS cache and persistence
│   ├── config.rs        # Configuration
│   ├── db/              # SQLite persistence
│   ├── dns/             # DNS protocol and resolution
│   ├── privileges.rs    # Process privilege handling
│   └── web/             # HTTP API, auth, WebSocket and UI serving
├── tests/               # Integration and correctness tests
├── stress-tests/        # Pressure/concurrency testing
├── scripts/              # Repeatable development/maintenance scripts
├── docs/                 # Engineering and release documentation
├── .github/              # CI and repository automation
└── frontend/             # Dedicated web frontend (V1 implementation)
```

The detailed repository layout and ownership rules are documented in [`docs/project-structure.md`](docs/project-structure.md).

## Quick Start

### Requirements

- Rust stable toolchain.
- Cargo.
- NASM on Windows for the AWS-LC-RS dependency used by JWT cryptography.
- SQLite support provided through SQLx.

### Clone and configure

```powershell
git clone https://github.com/aaron-fredrick/MyDNS.git
cd MyDNS
copy .env.example .env
```

Edit `.env` and/or the documented configuration to set ports, credentials, upstream DNS servers, and other runtime options.

### Run

Development:

```powershell
cargo run
```

Release build:

```powershell
cargo run --release
```

The dashboard/API is normally available at `http://localhost:8080` unless configured otherwise.

## Development Verification

The core Rust quality gates are:

```powershell
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
cargo audit
```

`cargo audit` is part of the V1 security review. At the current dependency state, RustSec reports **RUSTSEC-2023-0071 affecting `rsa 0.9.10` with no fixed upstream version available**. The dependency is not part of MyDNS's active `jsonwebtoken` feature path; the advisory is therefore tracked as a supply-chain/dependency investigation item rather than silently ignored.

## CI Workflows

GitHub Actions is organized into reusable **capability workflows** and **policy workflows**. Policy workflows decide which capabilities run for each type of change; capability workflows contain the actual checks.

### Capability checks

| Capability | Backend | Frontend | Platform |
|---|---|---|---|
| Lint (Light) | rustfmt + Clippy, warnings allowed | TypeScript typecheck | Linux |
| Lint (Full) | rustfmt + Clippy, warnings treated as errors | TypeScript typecheck | Linux |
| Static (Light) | `cargo check --lib --all-features` | `npm run typecheck` | Linux |
| Static (Full) | `cargo check --all-targets --all-features` | `npm run typecheck` | Linux |
| Build (Release) | Rust release build | Vite production build | Linux, Windows, macOS × x64/ARM64 |
| Build (Check) | Rust check | Frontend production build | Linux, Windows, macOS × x64/ARM64 |
| Tests (Unit) | Rust unit tests + coverage | Frontend unit tests + coverage | Linux |
| Tests (Component) | Rust component tests | Frontend component gate | Linux |
| Tests (Integration) | Rust integration tests | — | Linux |
| Tests (Contract) | Frontend/backend API contract | Frontend/backend API contract | Linux |
| Tests (Smoke) | Platform smoke checks | — | Linux, Windows, macOS × x64/ARM64 |
| Tests (E2E) | End-to-end API/DNS checks | End-to-end API/DNS checks | Linux, Windows, macOS × x64/ARM64 |
| Tests (Extended) | Extended Rust test suite | — | Linux |
| Security (Audit) | `cargo audit` | `npm audit` | Linux |
| Security (CodeQL) | Rust analysis | JavaScript/TypeScript analysis | Linux |

Platform-matrix capabilities reuse the release build artifacts for smoke and E2E validation rather than rebuilding the application.

### CI policies

| Policy | Trigger | Scope |
|---|---|---|
| Branch | Push to a non-`dev`/`main` branch with no open PR | Light lint, light static checks, unit tests |
| Pull Request | PR targeting a non-`dev`/`main` branch | Full lint/static, release build, unit/component/integration/contract/smoke tests, build check |
| Dev | Push to or PR targeting `dev` | Full lint/static, release build, unit/component/integration/contract/smoke/E2E tests, security, CodeQL, coverage |
| Main | Push to or PR targeting `main` | Same full release-gate coverage as Dev |
| Nightly | Scheduled daily run | Full lint/static, release build, all standard tests, extended tests, security and CodeQL |

The canonical workflow entry point is [`.github/workflows/ci.yml`](.github/workflows/ci.yml). Reusable policies live under [`.github/workflows/ci-policy-*.yml`](.github/workflows/), and reusable capabilities live under [`.github/workflows/ci-*.yml`](.github/workflows/).

Capability job names follow:

```text
<Job> (<Area>) - <Scope/Level> [<OS> <Arch>]
```

The `[OS Arch]` suffix is used only for jobs running on the platform/architecture matrix.

## Production Readiness / V1.0.0

The V1 release is being tracked in one canonical document:

[`docs/production-readiness-v1.0.0.md`](docs/production-readiness-v1.0.0.md)

The V1 work covers the remaining correctness, reliability, security, observability, UI, stress-testing, deployment, CI, and documentation requirements needed before declaring the product ready.

The goal is **a coherent, properly tested V1 release**, not an endlessly expanding feature backlog. Further improvements can be assessed after V1 based on real usage, reported issues, and feedback.

## Documentation

- [`docs/production-readiness-v1.0.0.md`](docs/production-readiness-v1.0.0.md) — V1 goals, requirements, work log, and release gate.
- [`docs/project-structure.md`](docs/project-structure.md) — repository structure and architectural ownership.
- [`docs/`](docs/) — additional engineering documentation as the project matures.

## License

Custom License. Free for public/personal use. **Commercial, production, or commercial-public use requires attribution to the author**. See [`LICENSE`](LICENSE) for details.

---

## 💖 Support the Project

If you find MyDNS useful, please consider supporting its development:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/aaronfredrick)
