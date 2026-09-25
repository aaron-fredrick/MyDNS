//! Counter metric types.
pub mod bounded;
pub mod scalar;

pub use bounded::BoundedCounter;
pub use scalar::ScalarCounter;
