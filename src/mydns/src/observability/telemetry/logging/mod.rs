//! Logging telemetry — output configuration, formatting, and writer lifetime.
//!
//! This module owns everything specific to log output:
//!
//! - log file and stdout destinations ([`layer`])
//! - formatting (text with/without ANSI)
//! - log directory and filename policy ([`config`])
//! - non-blocking writer lifetime management ([`guard`])
//!
//! Subscriber composition (wiring these layers into the global registry) lives
//! in [`super::pipeline`].

mod config;
mod guard;
pub(crate) mod layer;

pub use config::LoggingConfig;
pub use guard::LoggingGuard;
