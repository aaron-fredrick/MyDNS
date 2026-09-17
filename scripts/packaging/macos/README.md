# macOS packaging

macOS releases use native disk-image/package tooling rather than Inno Setup.

## Release artifacts

The intended V1 artifacts are:

```text
MyDNS-v<version>-macos-x64.dmg
MyDNS-v<version>-macos-arm64.dmg
```

A `.pkg` installer can be added when service installation and upgrade/uninstall behavior are defined for macOS.

## Application layout

The preferred macOS distribution model is a normal application bundle:

```text
/Applications/MyDNS.app/
└── Contents/
    ├── MacOS/
    │   └── mydns
    └── Resources/
        └── web/
```

The application bundle contains release-owned executable and frontend assets. It is disposable and can be replaced when upgrading.

## Persistent state

MyDNS configuration, database, and logs should not live inside the application bundle. The macOS implementation should use a stable per-machine or per-user application-data location, depending on the final service/security model. The final paths must be documented before `.pkg` installation is implemented.

The packaging design must preserve this separation:

```text
Application bundle  -> replaceable release files
Application data    -> persistent configuration/database/logs
```

Upgrades must replace the application without overwriting the existing configuration or database. Uninstall should remove release-owned application files while making the retention/removal policy for persistent state explicit to the user.

## Build and distribution

The packaging pipeline should build the Rust targets `x86_64-apple-darwin` and `aarch64-apple-darwin`, stage the MyDNS binary and built web assets, create a DMG from the staged application bundle, and validate the resulting artifact.

Production distribution should include Apple code signing and notarization before public release.
