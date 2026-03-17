# MyDNS

![CI status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/test.yml/badge.svg)
![CodeQL status](https://github.com/aaron-fredrick/MyDNS/actions/workflows/codeql.yml/badge.svg)
![Version](https://img.shields.io/badge/version-0.1.1--dev-blue)

A high-performance, restart-resilient DNS server with a modern management dashboard. Designed for speed, security, and extreme visibility.

## Features

- **Proper Caching (Restart-Resilient)**:
  - **Recursive CNAME Chasing**: Automatically follows CNAME chains in the persistent cache, resolving complex domains without upstream hits after restarts.
  - **Multi-Owner Caching**: Individually caches every record in a response under its specific owner name.
  - **Negative Caching**: Caches NXDOMAIN results to prevent repetitive upstream lookups.
- **Modern Dashboard**:
  - Live query logs with **Premium Aesthetics** (row-level coloring).
  - DNS record management (A, AAAA, CNAME, MX, PTR).
  - Real-time stats (Uptime, Cache Hits/Misses, Cache Size).
  - Secure authentication (Argon2 + JWT).
- **Resolver Logic**:
  - Context-aware resolution for `mydns.local`.
  - Configurable upstream priority (Cloudflare First vs. Router First).
  - Clean, modular architecture following **Command Query Separation (CQS)**.
- **Embedded Performance**: Written in Rust for maximum speed and safety.

## Quick Start

1. **Clone and Setup**:
   ```powershell
   git clone https://github.com/aaron-fredrick/MyDNS
   cd MyDNS
   copy .env.example .env
   ```
2. **Configure**: Edit `.env` to set your desired ports and credentials.
3. **Run**:
   ```powershell
   cargo run --release
   ```
4. **Access Dashboard**: Open `http://localhost:8080` (or your configured port).

## Development

- **Build**: `cargo build --release`
- **Test**: `cargo test`
- **Code Standards**: The project follows strict Rust conventions. Pull requests must adhere to the 100% CQS rule for handler logic.

## License

Custom License. Free for public/personal use. **Commercial, production, or commercial-public use requires attribution to the author**. See [LICENSE](LICENSE) for details.
