//! Tracing subscriber pipeline — build and install the global subscriber.
//!
//! [`init`] is the single entry point for setting up the MyDNS tracing
//! infrastructure. It replaces the ad-hoc subscriber construction that
//! previously lived inline in `main.rs`, keeping all subscriber policy in the
//! observability subsystem boundary as described in
//! `docs/observability/architecture.md`.
//!
//! ## Pipeline layers (in order)
//!
//! 1. `EnvFilter` — honours `RUST_LOG`; falls back to `info`
//! 2. `fmt` layer writing to a non-blocking file (no ANSI)
//! 3. `fmt` layer writing to stdout (with ANSI)
//! 4. `SamplyLayer` — optional; skipped silently when Samply is not attached
//!
//! ## Non-blocking writer lifetime
//!
//! `tracing-appender` returns a `WorkerGuard` alongside the non-blocking
//! writer. The guard **must** remain alive until the process exits, or the
//! writer worker will stop and buffered events will be lost. The guard is
//! stored inside [`super::guard::TracingGuard`] and returned to `main`, which
//! binds it to a long-lived variable.

use tracing_samply::SamplyLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::{config::TracingConfig, guard::TracingGuard};

/// Build and install the global `tracing` subscriber for MyDNS.
///
/// Must be called **once** before any `tracing::info!` / `tracing::error!`
/// etc. calls. Calling it more than once will panic (the underlying
/// `tracing-subscriber` global prevents double-initialisation).
///
/// Returns a [`TracingGuard`] that must be kept alive for the process
/// lifetime.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created. Samply
/// initialisation failure is non-fatal — the pipeline is installed without
/// the Samply layer and the process continues normally.
pub fn init(config: TracingConfig) -> anyhow::Result<TracingGuard> {
    // ── 1. Log directory and non-blocking file writer ─────────────────────────
    std::fs::create_dir_all(&config.log_directory)?;

    let file_appender =
        tracing_appender::rolling::never(&config.log_directory, &config.log_filename);
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);

    // ── 2. Filter ─────────────────────────────────────────────────────────────
    //
    // Prefer RUST_LOG; fall back to the config-supplied level string.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.env_filter_fallback));

    // ── 3. Format layers ──────────────────────────────────────────────────────
    //
    // File layer: no ANSI escape codes (log files are read by tools/dashboard).
    // Stdout layer: ANSI enabled for interactive terminal use.
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true);

    // ── 4. Optional Samply profiling layer ────────────────────────────────────
    //
    // Samply attaches when its profiler is running. Not finding a profiler is
    // the expected normal case — skipped silently. Only the attached case
    // emits a diagnostic, since that represents an explicit profiling session.
    //
    // eprintln! is used here because the subscriber is not yet installed so
    // tracing macros are not yet available.
    let samply_layer = match SamplyLayer::new() {
        Ok(layer) => {
            eprintln!("[tracing] Samply profiler attached — SamplyLayer active");
            Some(layer)
        }
        Err(_) => {
            // Normal case: Samply is not running. No diagnostic needed.
            None
        }
    };

    // ── 5. Assemble and install ───────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .with(samply_layer)
        .init();

    Ok(TracingGuard::new(file_guard, config.log_filename))
}
