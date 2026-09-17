# Linux packaging

Linux releases use platform-native filesystem layout and systemd integration rather than an Inno Setup-style installer.

V1 release artifacts should include portable archives such as:

```text
MyDNS-v<version>-linux-x64.tar.gz
MyDNS-v<version>-linux-arm64.tar.gz
```

The packaged installation layout is:

```text
/usr/bin/mydns
/usr/share/mydns/web/
/etc/mydns/mydns.toml
/var/lib/mydns/
/var/log/mydns/
```

A future package-manager layer can add `.deb` and `.rpm` packages without changing the application runtime layout. The service definition should be packaged as a systemd unit and the installer lifecycle must preserve `/var/lib/mydns` and configuration across upgrades.
