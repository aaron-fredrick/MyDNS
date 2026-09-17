# Test Support

Shared test infrastructure, not test cases.

`mod.rs` currently provides isolated temporary SQLite databases, in-process HTTP/DNS servers, authentication helpers, ephemeral port allocation, and restart lifecycle support.

Keep MyDNS product logic out of this directory; support code should simplify tests without reimplementing the resolver or API.
