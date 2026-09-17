# macOS packaging

macOS releases use native disk-image/package tooling rather than Inno Setup.

The intended V1 release artifacts are:

```text
MyDNS-v<version>-macos-x64.dmg
MyDNS-v<version>-macos-arm64.dmg
```

A `.pkg` installer can be added when service installation and upgrade/uninstall behavior are defined for macOS.

The packaging pipeline should build the Rust targets `x86_64-apple-darwin` and `aarch64-apple-darwin`, stage the MyDNS binary and built web assets, and create a DMG from each staged application bundle/package layout.

Production distribution should also include Apple code signing and notarization before public release.
