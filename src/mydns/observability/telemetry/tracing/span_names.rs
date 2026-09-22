//! Canonical span-name constants for MyDNS tracing.
//!
//! These names define the documented span hierarchy described in
//! `docs/observability/tracing.md`. They are the authoritative source for
//! span names within the MyDNS codebase; application code should reference
//! these constants rather than writing the strings inline.
//!
//! ## Hierarchy
//!
//! ```text
//! Process
//!   dns.server
//!   http.server
//!   background
//!
//! DNS request
//!   dns.request
//!     dns.parse
//!     dns.zone_lookup
//!     dns.blocklist_lookup
//!     dns.cache_lookup
//!     dns.local_record_lookup
//!     dns.upstream.resolve
//!       dns.upstream.request
//!       dns.upstream.retry
//!     dns.response
//!
//! HTTP request
//!   http.request
//!     http.authentication
//!     http.handler
//!     http.response
//!
//! Database
//!   db.connect
//!   db.migrate
//!   db.records.list / .create / .update / .delete
//!   db.zones.load
//!   db.settings.get
//! ```

// ── Process-level ─────────────────────────────────────────────────────────────

/// Root span for the DNS server task.
pub const DNS_SERVER: &str = "dns.server";

/// Root span for the HTTP server task.
pub const HTTP_SERVER: &str = "http.server";

/// Root span for background tasks.
pub const BACKGROUND: &str = "background";

// ── DNS request pipeline ──────────────────────────────────────────────────────

/// Span wrapping a complete DNS request/response cycle.
pub const DNS_REQUEST: &str = "dns.request";

/// Span wrapping the DNS message parsing step.
pub const DNS_PARSE: &str = "dns.parse";

/// Span wrapping the local zone ownership lookup.
pub const DNS_ZONE_LOOKUP: &str = "dns.zone_lookup";

/// Span wrapping the blocklist decision.
pub const DNS_BLOCKLIST_LOOKUP: &str = "dns.blocklist_lookup";

/// Span wrapping the DNS cache lookup.
pub const DNS_CACHE_LOOKUP: &str = "dns.cache_lookup";

/// Span wrapping the local record index lookup.
pub const DNS_LOCAL_RECORD_LOOKUP: &str = "dns.local_record_lookup";

/// Span wrapping the full upstream resolution attempt (including retries).
pub const DNS_UPSTREAM_RESOLVE: &str = "dns.upstream.resolve";

/// Span wrapping a single upstream DNS request.
pub const DNS_UPSTREAM_REQUEST: &str = "dns.upstream.request";

/// Span wrapping a single upstream retry attempt.
pub const DNS_UPSTREAM_RETRY: &str = "dns.upstream.retry";

/// Span wrapping DNS response construction.
pub const DNS_RESPONSE: &str = "dns.response";

// ── HTTP request pipeline ─────────────────────────────────────────────────────

/// Span wrapping a complete HTTP request/response cycle.
pub const HTTP_REQUEST: &str = "http.request";

/// Span wrapping the authentication step within an HTTP request.
pub const HTTP_AUTHENTICATION: &str = "http.authentication";

/// Span wrapping the route handler logic.
pub const HTTP_HANDLER: &str = "http.handler";

/// Span wrapping HTTP response construction.
pub const HTTP_RESPONSE: &str = "http.response";

// ── Database operations ───────────────────────────────────────────────────────

/// Span wrapping database connection/open.
pub const DB_CONNECT: &str = "db.connect";

/// Span wrapping schema migration execution.
pub const DB_MIGRATE: &str = "db.migrate";

/// Span wrapping listing DNS records.
pub const DB_RECORDS_LIST: &str = "db.records.list";

/// Span wrapping creating a DNS record.
pub const DB_RECORDS_CREATE: &str = "db.records.create";

/// Span wrapping updating a DNS record.
pub const DB_RECORDS_UPDATE: &str = "db.records.update";

/// Span wrapping deleting a DNS record.
pub const DB_RECORDS_DELETE: &str = "db.records.delete";

/// Span wrapping loading zones from the database.
pub const DB_ZONES_LOAD: &str = "db.zones.load";

/// Span wrapping reading a setting from the database.
pub const DB_SETTINGS_GET: &str = "db.settings.get";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_name_constants_have_expected_prefixes() {
        assert!(DNS_SERVER.starts_with("dns."));
        assert!(HTTP_SERVER.starts_with("http."));
        assert!(DNS_REQUEST.starts_with("dns."));
        assert!(HTTP_REQUEST.starts_with("http."));
        assert!(DB_CONNECT.starts_with("db."));
    }

    #[test]
    fn no_span_name_is_empty() {
        let names = [
            DNS_SERVER,
            HTTP_SERVER,
            BACKGROUND,
            DNS_REQUEST,
            DNS_PARSE,
            DNS_ZONE_LOOKUP,
            DNS_BLOCKLIST_LOOKUP,
            DNS_CACHE_LOOKUP,
            DNS_LOCAL_RECORD_LOOKUP,
            DNS_UPSTREAM_RESOLVE,
            DNS_UPSTREAM_REQUEST,
            DNS_UPSTREAM_RETRY,
            DNS_RESPONSE,
            HTTP_REQUEST,
            HTTP_AUTHENTICATION,
            HTTP_HANDLER,
            HTTP_RESPONSE,
            DB_CONNECT,
            DB_MIGRATE,
            DB_RECORDS_LIST,
            DB_RECORDS_CREATE,
            DB_RECORDS_UPDATE,
            DB_RECORDS_DELETE,
            DB_ZONES_LOAD,
            DB_SETTINGS_GET,
        ];
        for name in names {
            assert!(!name.is_empty(), "span name must not be empty");
        }
    }
}
