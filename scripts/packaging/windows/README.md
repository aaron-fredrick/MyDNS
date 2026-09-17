# Windows packaging

Windows releases use **Inno Setup**. The `.iss` files are the installer definitions; the generated `.exe` files are release artifacts.

```text
windows/
├── mydns-windows-x64.iss
├── mydns-windows-arm64.iss
└── README.md
```

Both installers use the same stable MyDNS application identity (`AppId`) across releases and architectures. Architecture selection is handled separately by `ArchitecturesAllowed`.

Persistent state is kept under `%ProgramData%\MyDNS\` while application binaries and web assets are installed under Program Files.

The PowerShell packaging helper invokes `ISCC.exe` when `-Installer` is requested.
