# End-to-End Tests

Full-system workflows against a running MyDNS instance.

Planned coverage:

- startup and readiness
- DNS + management API interaction
- Local DNS record lifecycle
- blocklist lifecycle and resolver effect
- cache and upstream behavior
- authentication and authorization
- frontend-to-API workflows
- clean install, upgrade, restart, and uninstall flows

E2E tests should use isolated temporary configuration, database, ports, and process state so they never depend on a developer's machine state.
