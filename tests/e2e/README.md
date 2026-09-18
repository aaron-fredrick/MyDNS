# End-to-End Tests

Full-system workflows against a real MyDNS process.

Planned coverage:

- startup/readiness and shutdown
- DNS + management API interaction
- Local DNS record lifecycle
- blocklist lifecycle and resolver effect
- cache and upstream behavior
- authentication/authorization workflows
- frontend-to-API workflows
- clean install, upgrade, restart, and uninstall

Use isolated ports, temporary configuration, database, and runtime state. E2E tests must not depend on a developer's machine state.
