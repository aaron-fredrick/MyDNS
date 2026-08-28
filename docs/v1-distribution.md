# MyDNS V1 Distribution Plan

## Production distribution model

MyDNS V1 will support two primary distribution forms:

1. **Portable package** — a self-contained archive for users who want to extract, configure, and run MyDNS without an installer.
2. **Installer/package** — a platform-native installation experience that installs the same MyDNS application and web assets while managing service registration and persistent application directories.

Both distributions must use the same Rust binary, React frontend build, configuration model, and runtime behaviour. The installer must not introduce separate application logic.

## Target installation/package layout

The canonical installed layout is:

```text
MyDNS installation
├── bin/
│   └── mydns
│
├── web/
│   ├── index.html
│   └── assets/
│
├── config/
│   └── mydns.toml
│
├── data/
│   └── mydns.db
│
└── logs/
```

The portable archive may initially use this same layout. A platform installer may place application files and persistent data in platform-appropriate locations; on Windows, application files are expected under the installation directory while `config/`, `data/`, and `logs/` should live under the platform's persistent application-data location so upgrades do not overwrite user data.

## Frontend packaging

The React/Vite frontend is source-controlled under `frontend/` and is built by Node.js tooling. Node.js is a build-time dependency only and is not required to run MyDNS in production.

The production frontend output is packaged as `web/` containing `index.html` and `assets/`. Rust/Axum serves those static files. The frontend should not be embedded into the Rust binary for the V1 distribution model unless a future distribution target explicitly requires a single-file executable.

Build flow:

```text
frontend/ source
      ↓
Node.js + Vite
      ↓
web/
      ↓
MyDNS distribution
      ├── bin/mydns
      └── web/
```

## V1 release targets

Priority targets:

- Windows x64 portable ZIP
- Windows x64 installer
- Linux x64 portable tarball

Additional package formats such as `.deb`, `.rpm`, Docker images, and ARM64 builds can be added during V1.x when the packaging requirements are established.

## Upgrade principle

Application binaries and web assets are versioned and released together, but persistent configuration, database data, and logs must survive application upgrades.

```text
Application files  → replace during upgrade
config/data/logs   → preserve during upgrade
```

This separation should be reflected in installer/package implementation and documented before the V1 release candidate.
