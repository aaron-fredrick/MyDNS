//! Database metric domain.

pub mod aggregator;
pub mod measurements;

pub use aggregator::{DatabaseAggregationSnapshot, DatabaseAggregator};
