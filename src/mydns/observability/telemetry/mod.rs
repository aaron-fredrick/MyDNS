//! Telemetry subsystems for MyDNS observability.
//!
//! [`pipeline`] is the single composition point that wires logging, tracing,
//! and future signals into the global `tracing-subscriber` registry.

pub mod logging;
pub mod metrics;
pub mod pipeline;
pub mod tracing;
