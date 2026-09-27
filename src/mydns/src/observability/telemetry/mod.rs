//! Telemetry subsystems for MyDNS observability.
//!
//! [`pipeline`] is the single composition point that wires logging and tracing
//! into the global `tracing-subscriber` registry.
//!
//! Profiling is owned by the parent observability subsystem. The telemetry
//! pipeline may compose the profiling layer, but profiling is not a telemetry
//! subsystem.

pub mod logging;
pub mod metrics;
pub mod pipeline;
pub mod tracing;
