//! Summary metric types.
pub mod tdigest;
pub mod tdigest_snapshot;
pub use tdigest::TDigestSummary;
pub use tdigest_snapshot::TDigestSummarySnapshot;
