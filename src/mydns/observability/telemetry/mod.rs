//! Telemetry subsystems for MyDNS observability.
//!
//! Long-term abstraction boundary for tracing, metrics, and logging.
//! Existing implementations remain in place until they are migrated.

pub mod tracing;
pub mod metrics;
pub mod logging;
