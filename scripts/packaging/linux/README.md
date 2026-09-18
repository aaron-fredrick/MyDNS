# Linux packaging

Linux releases use a native filesystem layout and systemd integration rather than an Inno Setup-style installer.

## Release artifacts

V1 should publish portable archives first:

```text
MyDNS-v<version>-linux-x64.tar.gz
MyDNS-v<version>-linux-arm64.tar.gz
```

The archive contains the application binary, built web assets, and example configuration. It is intended for systems where the operator wants to choose the final installation prefix.

## Native installation layout

When installed as a system service, the intended layout is:

```text
/usr/bin/mydns
/usr/share/mydns/web/
/etc/mydns/mydns.toml
/var/lib/mydns/
/var/log/mydns/
```

| Location | Purpose | Upgrade/uninstall behavior |
|---|---|---|
| `/usr/bin/mydns` | MyDNS executable | Replaced on upgrade; removed on uninstall |
| `/usr/share/mydns/web/` | React production assets | Replaced on upgrade; removed on uninstall |
| `/etc/mydns/mydns.toml` | System configuration | Preserved across upgrades; normally preserved on uninstall |
| `/var/lib/mydns/` | Persistent application data, including SQLite database | Preserved across upgrades and intentionally retained by default on uninstall |
| `/var/log/mydns/` | Persistent service logs | Preserved across upgrades and intentionally retained by default on uninstall |

The key rule is that **application files are disposable, while configuration and runtime state are persistent**.

## systemd

The Linux package should install a systemd unit that starts `/usr/bin/mydns` using `/etc/mydns/mydns.toml` and the service account/permissions defined by the production deployment design.

The service lifecycle must support clean install, start, stop, restart, upgrade, rollback, and uninstall behavior without deleting persistent configuration or database state accidentally.

## Future package formats

`.deb` and `.rpm` packages can be added later without changing the runtime filesystem layout above. The package-manager implementations should install the same files and preserve the same upgrade/uninstall semantics.
