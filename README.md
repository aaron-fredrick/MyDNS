<div align="center">

# ⬡ MyDNS

[![CI status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/test.yml/badge.svg)](https://github.com/aaron-fredrick/MyDNS/actions/workflows/test.yml)
[![CodeQL status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/codeql.yml/badge.svg)](https://github.com/aaron-fredrick/MyDNS/actions/workflows/codeql.yml)
![Version](https://img.shields.io/badge/version-v0.1.1--dev-blue?style=flat-square)

</div>

**MyDNS** is a Rust-based DNS server and management service built around authoritative DNS, upstream resolution, persistent caching, and a web dashboard.

The project is currently being prepared for its first proper production release: **V1.0.0**. The V1 scope is deliberately focused on finishing and hardening the existing product rather than expanding it indefinitely.

> **Status:** V1 production-readiness work in progress. The `production-readiness` branch is the active release-hardening branch.

## What MyDNS Provides

### DNS server

- UDP and TCP DNS serving.
- Authoritative records including A, AAAA, CNAME, MX, NS, PTR, and TXT.
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
- Current UI is embedded with the Rust service; the frontend is being structured for a dedicated React + TypeScript implementation as part of the V1 architecture.

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
