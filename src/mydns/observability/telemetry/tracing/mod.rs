//! MyDNS tracing infrastructure.
//!
//! This module owns the complete tracing/subscriber pipeline for the MyDNS
//! process.  It is the central configuration point for:
//!
//! - subscriber layers (file, stdout, Samply)
//! - environment-variable filter (`RUST_LOG`)
//! - formatting (text with/without ANSI)
//! - non-blocking writer lifetime management
//! - canonical span names ([`span_names`])
//! - canonical structured field names ([`fields`])
//!
//! # Usage
//!
//! ```rust,ignore
//! use mydns::observability::telemetry::tracing as tracing_foundation;
//!
//! let config = tracing_foundation::TracingConfig::default();
//! let _tracing_guard = tracing_foundation::init(config)?;
//! tracing::info!(log_file = %_tracing_guard.log_filename, "MyDNS starting");
//! ```
//!
//! The returned [`TracingGuard`] must be kept alive for the duration of the
//! process.  Dropping it earlier causes the non-blocking file writer to stop.

mod config;
mod guard;
mod pipeline;

pub mod fields;
pub mod span_names;

pub use config::TracingConfig;
pub use guard::TracingGuard;
pub use pipeline::init;
