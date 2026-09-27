//! Canonical span/log field-name constants for MyDNS tracing.
//!
//! These names define the structured field vocabulary described in
//! `docs/observability/tracing.md` and `docs/observability/logging.md`.
//! Using shared constants prevents accidental divergence between the field
//! names used in different modules.
//!
//! ## Security note
//!
//! The fields listed here are safe bounded attributes. Never record the
//! following as span/log fields:
//! - raw Authorization header values
//! - passwords or JWT secrets
//! - raw SQL parameters
//! - unbounded client identifiers in routine spans
//!
//! See `docs/observability/tracing.md` §Span attributes for the full policy.

// ── Common fields ─────────────────────────────────────────────────────────────

/// Identifies the subsystem that emitted the span or event.
///
/// Example values: `"dns"`, `"http"`, `"db"`, `"cache"`, `"upstream"`.
pub const COMPONENT: &str = "component";

/// Identifies the specific operation within the component.
///
/// Example values: `"resolve"`, `"lookup"`, `"migrate"`, `"login"`.
pub const OPERATION: &str = "operation";

/// Bounded outcome classification for the span.
///
/// Safe values: `"success"`, `"timeout"`, `"refused"`, `"not_found"`,
/// `"validation_error"`, `"internal_error"`.
pub const OUTCOME: &str = "outcome";

/// Duration of the operation in milliseconds.
pub const DURATION_MS: &str = "duration_ms";

// ── Correlation fields ────────────────────────────────────────────────────────

/// Trace identifier, included in structured logs to allow metric anomaly →
/// trace → log correlation as described in `docs/observability/tracing.md`.
pub const TRACE_ID: &str = "trace_id";

/// Per-request identifier propagated through the request lifecycle.
pub const REQUEST_ID: &str = "request_id";

// ── DNS-specific fields ───────────────────────────────────────────────────────

/// Transport protocol of a DNS request.
///
/// Safe values: `"udp"`, `"tcp"`.
pub const TRANSPORT: &str = "transport";

/// DNS record type.
///
/// Example values: `"A"`, `"AAAA"`, `"MX"`, `"CNAME"`.
/// This is a bounded enum of known types — do not use raw client input.
pub const RECORD_TYPE: &str = "record_type";

/// DNS response code.
///
/// Example values: `"NOERROR"`, `"NXDOMAIN"`, `"SERVFAIL"`, `"REFUSED"`.
pub const RESPONSE_CODE: &str = "response_code";

/// How the DNS request was resolved.
///
/// Example values: `"local"`, `"cache"`, `"upstream"`, `"blocked"`.
pub const RESOLUTION_PATH: &str = "resolution_path";

/// Identifier of the selected upstream resolver.
///
/// Must be a configured endpoint identifier, not arbitrary user input.
pub const UPSTREAM: &str = "upstream";

// ── HTTP-specific fields ──────────────────────────────────────────────────────

/// HTTP method.
///
/// Example values: `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`.
pub const HTTP_METHOD: &str = "http.method";

/// Normalized route template (not the raw URL).
///
/// Example values: `"/api/v1/records"`, `"/api/v1/records/:id"`.
/// Never use the raw URL path as it may contain user data.
pub const HTTP_ROUTE: &str = "http.route";

/// HTTP response status code.
pub const HTTP_STATUS: &str = "http.status";

// ── Database-specific fields ──────────────────────────────────────────────────

/// Semantic database operation name.
///
/// Example values: `"records.list"`, `"records.create"`, `"zones.load"`,
/// `"settings.get"`. Do not use raw SQL text.
pub const DB_OPERATION: &str = "db.operation";

/// Error classification for a database failure.
///
/// Example values: `"busy"`, `"locked"`, `"constraint"`, `"io"`.
pub const DB_ERROR_CLASS: &str = "db.error_class";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_field_name_is_empty() {
        let fields = [
            COMPONENT,
            OPERATION,
            OUTCOME,
            DURATION_MS,
            TRACE_ID,
            REQUEST_ID,
            TRANSPORT,
            RECORD_TYPE,
            RESPONSE_CODE,
            RESOLUTION_PATH,
            UPSTREAM,
            HTTP_METHOD,
            HTTP_ROUTE,
            HTTP_STATUS,
            DB_OPERATION,
            DB_ERROR_CLASS,
        ];
        for field in fields {
            assert!(!field.is_empty(), "field name must not be empty");
        }
    }

    #[test]
    fn http_fields_use_http_prefix() {
        assert!(HTTP_METHOD.starts_with("http."));
        assert!(HTTP_ROUTE.starts_with("http."));
        assert!(HTTP_STATUS.starts_with("http."));
    }

    #[test]
    fn db_fields_use_db_prefix() {
        assert!(DB_OPERATION.starts_with("db."));
        assert!(DB_ERROR_CLASS.starts_with("db."));
    }
}
