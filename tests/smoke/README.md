# Smoke Tests

Fast release and deployment health checks. Smoke tests should be short, deterministic, and suitable for every packaged build.

Planned areas:

- binary starts and remains healthy
- HTTP health/readiness endpoint
- DNS A/AAAA resolution
- Local DNS lookup
- blocklist lookup
- management API authentication and basic read
- frontend asset serving
- packaged layout and required runtime directories

The existing PowerShell DNS smoke script remains the current operational smoke check until a cross-platform harness is introduced.
