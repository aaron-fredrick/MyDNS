# Release and Operational Scripts

## Build

- `build-release.ps1` — builds the supported Rust targets, builds/typechecks the React frontend, and assembles complete per-target layouts under `out/`.
- `package-release.ps1` — creates a portable ZIP from an assembled target and can invoke the matching Inno Setup definition for Windows.

## Smoke

- `dns-smoke-test.ps1` — current Windows DNS smoke test against a running MyDNS instance.

## Installer

The Inno Setup definitions live under `scripts/installer/` and consume the generated `out/` layout. They package the application under Program Files while preserving persistent configuration, database, and logs under ProgramData during uninstall.

## Release rule

Generated output belongs under `out/` and must not be treated as source input. A release job should build the frontend before packaging and should pass the release version explicitly to the installer rather than relying on a hard-coded version.
