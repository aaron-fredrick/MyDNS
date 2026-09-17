# Windows Installers

Inno Setup definitions for the supported Windows architectures:

- `mydns-windows-x64.iss`
- `mydns-windows-arm64.iss`

The definitions expect the release build to provide:

- `out/win-x64/bin/mydns.exe` or `out/win-arm64/bin/mydns.exe`
- `out/web/`
- `config.toml.example`

Installer output is written to `out/installers/`.

The release version should be supplied with Inno Setup's `/DMyDNSVersion=<version>` override. The default in the files is only a local-development fallback.

Persistent state is intentionally outside the application directory under `%ProgramData%\MyDNS\{config,data,logs}` so uninstalling the application does not silently delete the database or user configuration.
