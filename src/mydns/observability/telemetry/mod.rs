//! Telemetry subsystems for MyDNS observability.
//!
//! [`pipeline`] is the single composition point that wires logging, tracing,
//! logging and tracing into the global `tracing-subscriber` registry.\n//!\n//! Profiling is owned by the parent observability subsystem and is only\n//! composed here by the telemetry pipeline.

pub mod logging;
pub mod metrics;
pub mod pipeline;
pub mod profiling;
pub mod tracing;
