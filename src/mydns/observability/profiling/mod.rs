//! Runtime profiling integration for MyDNS.
//!
//! Profiling is a performance-analysis capability, not a fourth telemetry
//! signal. This module owns the optional profiler integration used by the
//! telemetry pipeline.
//!
//! The current implementation integrates Samply when a profiler is attached.
//! The normal non-profiled process path remains silent and unchanged.

use tracing_samply::SamplyLayer;

/// Build the optional Samply profiling layer.
///
/// Samply is expected to be absent during normal operation. In that case this
/// returns None without producing diagnostic output. When a profiler is
/// attached, a short diagnostic is emitted before the global subscriber is
/// installed.
pub fn layer() -> Option<SamplyLayer> {
    match SamplyLayer::new() {
        Ok(layer) => {
            eprintln!("[telemetry] Samply profiler attached — SamplyLayer active");
            Some(layer)
        }
        Err(_) => None,
    }
}
