//! Telemetry subscriber pipeline — compose and install the global subscriber.
//!
//! [`init`] is the single entry point for setting up the MyDNS observability
//! infrastructure. It composes the logging layers, optional profiling layer,
//! and environment filter into one `tracing-subscriber` registry, then
//! installs it as the process-global subscriber.
//!
//! ## Pipeline layers (in order)
//!
//! 1. `EnvFilter` — honours `RUST_LOG`; falls back to the config-supplied level
//! 2. `fmt` layer writing to a non-blocking file (no ANSI) — writer from [`super::logging`]
//! 3. `fmt` layer writing to stdout (with ANSI)
//! 4. `SamplyLayer` — optional; skipped silently when Samply is not attached
//!
//! ## Ownership boundary
//!
//! This module composes signals; it does not own the policies of individual
//! signals. The non-blocking file writer and its guard belong to
//! [`super::logging`]. Span names and field vocabulary belong to
//! [`super::tracing`].
//!
//! ## Non-blocking writer lifetime
//!
//! `tracing-appender` returns a `WorkerGuard` alongside the non-blocking
//! writer. The guard **must** remain alive until the process exits, or the
//! writer worker will stop and buffered events will be lost. The guard is
//! stored inside [`super::logging::LoggingGuard`] and returned to `main`,
//! which binds it to a long-lived variable.

use tracing_samply::SamplyLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::logging::{layer::build_writer, LoggingConfig, LoggingGuard};

/// Build and install the global `tracing` subscriber for MyDNS.
///
/// Must be called **once** before any `tracing::info!` / `tracing::error!`
/// etc. calls. Calling it more than once will panic (the underlying
/// `tracing-subscriber` global prevents double-initialisation).
///
/// Returns a [`LoggingGuard`] that must be kept alive for the process
/// lifetime.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created. Samply
/// initialisation failure is non-fatal — the pipeline is installed without
/// the Samply layer and the process continues normally.
pub fn init(config: LoggingConfig) -> anyhow::Result<LoggingGuard> {
    // ── 1. Non-blocking file writer (from logging module) ─────────────────────
    //
    // The writer is set up in the logging module, which owns the file/directory
    // policy. The fmt layers are constructed here so that Rust can infer the
    // full composed subscriber type (Layer<S> is invariant over S).
    let writer = build_writer(&config)?;

    // ── 2. Format layers ──────────────────────────────────────────────────────
    //
    // File layer: no ANSI escape codes (log files are read by tools/dashboard).
    // Stdout layer: ANSI enabled for interactive terminal use.
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer.non_blocking_file)
        .with_ansi(false);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true);

    // ── 3. Environment filter ──────────────────────────────────────────────────
    //
    // Prefer RUST_LOG; fall back to the config-supplied level string.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.env_filter_fallback));

    // ── 4. Optional Samply profiling layer ─────────────────────────────────────
    //
    // Samply attaches when its profiler is running. Not finding a profiler is
    // the expected normal case — skipped silently. Only the attached case
    // emits a diagnostic, since that represents an explicit profiling session.
    //
    // eprintln! is used here because the subscriber is not yet installed so
    // tracing macros are not yet available.
    let samply_layer = match SamplyLayer::new() {
        Ok(layer) => {
            eprintln!("[telemetry] Samply profiler attached — SamplyLayer active");
            Some(layer)
        }
        Err(_) => {
            // Normal case: Samply is not running. No diagnostic needed.
            None
        }
    };

    // ── 5. Assemble and install ────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .with(samply_layer)
        .init();

    Ok(LoggingGuard::new(writer.worker_guard, config.log_filename))
}
