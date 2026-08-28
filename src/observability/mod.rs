//! Backend-owned operational telemetry.
//!
//! The observability subsystem owns metric state, aggregation, and typed
//! snapshots. DNS-specific instrumentation adapters stay in `dns/`.

mod metrics;
mod types;

pub use metrics::Metrics;
pub use types::{LatencyStats, MetricsSnapshot, UpstreamStats};
