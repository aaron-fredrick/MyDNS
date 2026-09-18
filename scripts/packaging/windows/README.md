# Windows packaging

Windows releases use **Inno Setup**. The `.iss` files are installer definitions; the generated `.exe` files are release artifacts.

## Installer definitions

```text
windows/
├── mydns-windows-x64.iss
├── mydns-windows-arm64.iss
└── README.md
```

Both installers use the same stable MyDNS application identity (`AppId`) across releases and architectures. Architecture selection is handled separately by `ArchitecturesAllowed`.

## Installation layout

The installer deliberately separates **application files** from **persistent machine state**.

```text
C:\Program Files\MyDNS\
├── bin\
│   └── mydns.exe
└── web\
    └── ...

C:\ProgramData\MyDNS\
├── config\
│   └── mydns.toml
├── data\
│   └── mydns.db
└── logs\
    └── ...
```

`C:\Program Files\MyDNS` contains release-owned binaries and web assets. It may be replaced during an upgrade and is removed when the application is uninstalled.

`C:\ProgramData\MyDNS` contains machine-wide persistent state. The installer creates `config`, `data`, and `logs`. The example configuration is copied to `mydns.toml` only when that file does not already exist, so an existing configuration is not overwritten during an upgrade.

The database and logs are intentionally outside Program Files so application upgrades do not destroy runtime state. The current uninstall definition removes the application directory while retaining the ProgramData state.

## Packaging flow

1. Build the Rust Windows target.
2. Build the React frontend into the release web directory.
3. Stage the executable and web assets under `out/win-x64` or `out/win-arm64`.
4. Invoke the matching `.iss` definition with the release version.
5. Produce a versioned Windows setup `.exe` under `out/installers`.

The PowerShell packaging helper invokes `ISCC.exe` when `-Installer` is requested.
