# Test Support

Shared test infrastructure that is not itself a test case.

Planned contents:

- temporary MyDNS process lifecycle
- isolated config/database/port helpers
- DNS client helpers
- HTTP API clients
- authentication helpers
- fixture loading
- readiness polling
- common assertions and result collection

Keep product logic out of this directory. Support code should make tests clearer without becoming a second implementation of MyDNS.
