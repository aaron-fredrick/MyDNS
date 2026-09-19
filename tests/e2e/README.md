# End-to-End Tests

Full-system workflows against a real MyDNS process.

Implemented API workflow coverage:

- authentication/login
- records CRUD
- zone CRUD
- blocklist CRUD
- settings access
- stats and historical metrics contract

Run `python tests/e2e/api_e2e.py --password <test-password>` against an isolated running instance. DNS, frontend-browser, and install/upgrade lifecycle workflows remain additional release-level E2E work.

Use isolated ports, temporary configuration, database, and runtime state. E2E tests must not depend on a developer's machine state.
