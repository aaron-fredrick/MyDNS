//! Operational telemetry and metrics for MyDNS.
//!
//! This module owns backend-calculated observability data. DNS-specific
//! instrumentation adapters remain in their owning subsystem (`dns/`).

mod metrics;

pub use metrics::Metrics;
