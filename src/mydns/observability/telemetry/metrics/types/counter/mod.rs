//! Counter metric types.
pub mod bounded;
pub mod bounded_snapshot;
pub mod scalar;
pub mod scalar_snapshot;

pub use bounded::BoundedCounter;
pub use bounded_snapshot::BoundedCounterSnapshot;
pub use scalar::ScalarCounter;
pub use scalar_snapshot::ScalarCounterSnapshot;
