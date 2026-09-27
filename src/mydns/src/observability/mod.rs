//! Backend-owned operational telemetry.
//!
//! The observability subsystem owns metric state, aggregation, and typed
//! snapshots. DNS-specific instrumentation adapters stay in `dns/`.

mod metrics;
mod types;

pub mod alerts;
pub mod database;
pub mod health;
pub mod profiling;
pub mod resources;
pub mod telemetry;

pub use metrics::Metrics;
pub use types::{HistorySample, LatencyStats, MetricsHistory, MetricsSnapshot, UpstreamStats};
