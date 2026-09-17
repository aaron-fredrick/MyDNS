# Test Support

Shared test infrastructure, not test cases.

Planned helpers include temporary process lifecycle, isolated config/database/ports, DNS and HTTP clients, auth helpers, fixture loading, readiness polling, common assertions, and result collection.

Keep MyDNS product logic out of this directory; support code should simplify tests without reimplementing the resolver or API.
