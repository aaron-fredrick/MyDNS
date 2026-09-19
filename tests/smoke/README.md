# Smoke Tests

Fast checks suitable for every packaged build and deployment.

Implemented checks include HTTP stats reachability, authenticated login, record API reachability, and historical metrics API reachability via tests/smoke/api-smoke.ps1. The existing scripts/dns-smoke-test.ps1 remains the DNS protocol smoke check. Packaged-process startup, frontend asset serving, and deployment-directory checks remain release-smoke work.

The existing PowerShell DNS smoke script remains the operational smoke check until a cross-platform harness is added.
