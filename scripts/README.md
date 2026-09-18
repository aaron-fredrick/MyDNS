# Release and Operational Scripts

## Build

- `build-release.ps1` — builds the supported Rust targets, builds/typechecks the React frontend, and assembles complete per-target layouts under `out/`.
- `package-release.ps1` — creates a portable package from an assembled target and can invoke the matching platform packaging definition.

## Smoke

- `dns-smoke-test.ps1` — current Windows DNS smoke test against a running MyDNS instance.

## Packaging

Platform-specific packaging definitions live under `scripts/packaging/`:

```text
scripts/packaging/
├── windows/   # Inno Setup .iss definitions and Windows installer documentation
├── linux/     # Linux archive/package and systemd packaging definitions
└── macos/     # macOS DMG/PKG packaging definitions
```

Windows currently uses Inno Setup for x64 and ARM64 installers. Linux uses portable archives first, with `.deb`/`.rpm` package-manager integration planned as a follow-up. macOS uses native DMG/PKG tooling, with code signing and notarization required for production distribution.

## Release rule

Generated output belongs under `out/` and must not be treated as source input. A release job should build the frontend before packaging and should pass the release version explicitly to platform-specific packaging tools.
