//! Logging layer construction — file and stdout `fmt` layers.
//!
//! This module provides the logging-specific resources for the telemetry
//! pipeline: the non-blocking file writer and the `WorkerGuard` that keeps it
//! alive. The pipeline ([`crate::observability::telemetry::pipeline`]) is
//! responsible for composing these into `fmt` layers and installing them into
//! the global subscriber registry.
//!
//! ## Why this module does not construct `fmt::Layer` directly
//!
//! `tracing-subscriber`'s `Layer<S>` generic is invariant over `S`. A layer
//! returned as `Box<dyn Layer<Registry>>` cannot be passed to `.with()` on a
//! `Layered<EnvFilter, Registry>` — the composed subscriber type does not
//! match. Constructing the layers inline in the pipeline (where the full type
//! is inferred) avoids this constraint while keeping the non-blocking writer
//! lifecycle firmly inside this module.

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

use super::config::LoggingConfig;

/// The logging-specific resources produced by [`build_writer`].
///
/// The `worker_guard` must be kept alive for the process lifetime — it drives
/// the non-blocking file writer thread.
pub(crate) struct LoggingWriter {
    /// Non-blocking writer connected to the log file.
    pub(crate) non_blocking_file: NonBlocking,
    /// Must be held alive — dropping it shuts down the file writer thread.
    pub(crate) worker_guard: WorkerGuard,
}

/// Set up the non-blocking log file writer from the supplied config.
///
/// Creates the log directory if it does not exist, opens the file appender,
/// and returns the non-blocking writer together with the `WorkerGuard`.
///
/// The caller ([`super::super::pipeline`]) constructs the `fmt` layers using
/// the returned writer, so that Rust's type inference can resolve the full
/// composed subscriber type.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created.
pub(crate) fn build_writer(config: &LoggingConfig) -> anyhow::Result<LoggingWriter> {
    // Create the log directory if it does not already exist.
    std::fs::create_dir_all(&config.log_directory)?;

    // Non-blocking file appender — the returned WorkerGuard must outlive the
    // subscriber or buffered events will be silently lost.
    let file_appender =
        tracing_appender::rolling::never(&config.log_directory, &config.log_filename);
    let (non_blocking_file, worker_guard) = tracing_appender::non_blocking(file_appender);

    Ok(LoggingWriter {
        non_blocking_file,
        worker_guard,
    })
}
