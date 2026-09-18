# MyDNS V1 Distribution Plan

## Production distribution model

MyDNS V1 will support two primary distribution forms:

1. **Portable package** — a self-contained archive for users who want to extract, configure, and run MyDNS without an installer.
2. **Installer/package** — a platform-native installation experience that installs the same MyDNS application and web assets while managing service registration and persistent application directories.

Both distributions must use the same Rust binary, React frontend build, configuration model, and runtime behaviour. The installer must not introduce separate application logic.

## Canonical V1 runtime layout

The logical MyDNS installation layout is:

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

Responsibilities:

- `bin/` — MyDNS Rust executable and any directly required runtime binaries.
- `web/` — production React/Vite static build served by MyDNS. It is generated output, not frontend source code.
- `config/` — user/system configuration such as `mydns.toml`.
- `data/` — persistent application state, including the SQLite database.
- `logs/` — runtime/application logs.

The frontend source remains under `frontend/`. Node.js is a build-time dependency only; production deployments do not require Node.js or a Node web server.

## Portable distribution

Portable distributions are archive-based and do not install MyDNS into platform-managed application directories.

### Windows portable

Target artifact:

```text
MyDNS-v1.0.0-windows-x64-portable.zip
```

Expected archive layout:

```text
MyDNS/
├── bin/
│   └── mydns.exe
├── web/
│   ├── index.html
│   └── assets/
├── config/
│   └── mydns.toml
├── data/
│   └── mydns.db
└── logs/
```

The user extracts the archive to a directory of their choice and starts `bin/mydns.exe`. No installer, registry registration, Windows service registration, or Node.js runtime is required for portable operation.

The portable package is intended for developers, administrators, testing, isolated servers, and users who want explicit control over where MyDNS lives. Upgrades are performed by replacing application files while preserving the user's configuration, database, and logs.

The Windows portable release should include clear documentation for required privileges, binding to DNS port 53, HTTP/UI port configuration, firewall rules, configuration location, and clean shutdown.

### Linux portable

Target artifact:

```text
MyDNS-v1.0.0-linux-x64-portable.tar.gz
```

Expected archive layout:

```text
MyDNS/
├── bin/
│   └── mydns
├── web/
│   ├── index.html
│   └── assets/
├── config/
│   └── mydns.toml
├── data/
│   └── mydns.db
└── logs/
```

The archive is extracted by the administrator and `bin/mydns` is executed directly or from an administrator-created service definition. No package-manager registration is required.

The portable Linux package is intended for testing, custom server layouts, air-gapped deployments, and administrators who want to control service management themselves.

## Installer / native package distribution

The installer is a convenience and integration layer around the same MyDNS release. It must install the same application binary and matching frontend build as the portable package.

The installer should handle platform integration rather than application logic.

### Windows installer

Target artifact:

```text
MyDNS-v1.0.0-windows-x64-setup.exe
```

The installer should:

- Install MyDNS application files under a standard installation directory such as `Program Files/MyDNS/`.
- Install `bin/mydns.exe` and the matching `web/` assets.
- Create persistent configuration/data/log directories under an appropriate Windows application-data location such as `ProgramData/MyDNS/`.
- Offer or configure MyDNS as a Windows service where the V1 service model requires it.
- Register start/stop/restart behaviour with the service manager.
- Provide an uninstall path.
- Preserve configuration and persistent database data during normal upgrades/uninstalls unless the user explicitly requests removal.
- Document DNS port 53 and management HTTP/HTTPS port/firewall requirements.
- Avoid requiring Node.js at runtime.

A conceptual installed Windows layout is:

```text
Program Files/
└── MyDNS/
    ├── bin/
    │   └── mydns.exe
    └── web/
        ├── index.html
        └── assets/

ProgramData/
└── MyDNS/
    ├── config/
    │   └── mydns.toml
    ├── data/
    │   └── mydns.db
    └── logs/
```

Application files and persistent data deliberately have separate lifecycles. A MyDNS upgrade may replace the executable and web assets without overwriting the database or administrator configuration.

### Linux native package / service installation

For V1, the primary Linux packaged target is a portable tarball. Native packages such as `.deb` and `.rpm` are planned as V1.x additions unless they become necessary for the initial release.

When native Linux packaging is introduced, the package should follow normal Linux filesystem and service conventions rather than copying the Windows layout literally. The expected model is:

```text
/usr/bin/mydns                 executable
/usr/share/mydns/web/          static React assets
/etc/mydns/mydns.toml         configuration
/var/lib/mydns/               persistent database/state
/var/log/mydns/               logs
```

The package should install and manage a `systemd` service, set appropriate permissions, support clean start/stop/restart, and preserve `/etc/mydns` and `/var/lib/mydns` across application upgrades.

The native Linux package must still contain the same Rust application and matching frontend build as the portable release.

## Portable vs installer responsibilities

| Concern | Portable | Installer / native package |
|---|---|---|
| MyDNS binary | Included | Included |
| React `web/` assets | Included | Included |
| Node.js runtime | Not required | Not required |
| Configuration | Included/provided in package | Created/managed in platform config location |
| SQLite/data | Persistent beside package | Persistent platform data location |
| Logs | Persistent beside package | Persistent platform log location |
| Service registration | Manual | Managed by installer/package |
| Firewall integration | Documented/manual | Installer may configure/guide it |
| Start menu/shortcuts | No | Windows installer may provide |
| Uninstall | Delete package / manual cleanup | Native uninstall process |
| Upgrade | Replace application files manually | Managed upgrade preserving data/config |
| Best for | Admins, testing, custom layouts | Normal end users and managed servers |

## Release artifact relationship

Portable and installer distributions are two delivery mechanisms for the **same MyDNS release**:

```text
                    MyDNS v1.0.0
                         │
             ┌───────────┴───────────┐
             ▼                       ▼
       Portable package        Installer/package
             │                       │
             ▼                       ▼
       mydns + web              mydns + web
             │                       │
             └───────────┬───────────┘
                         ▼
                  Same application
```

The installer must not fork application behaviour. Differences are limited to filesystem placement, service integration, platform registration, and user experience.

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
