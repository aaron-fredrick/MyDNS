//! Counter metric types.
pub mod bounded;
pub mod scalar;

pub use bounded::BoundedCounts;
pub use scalar::ScalarCounter;
