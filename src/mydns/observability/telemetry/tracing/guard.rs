//! Tracing guard — keeps non-blocking writer alive for the process lifetime.

/// Holds the resources that must remain alive while tracing is active.
///
/// Dropping this guard before the process exits causes the non-blocking file
/// writer to flush and shut down. In practice this should be bound to a
/// variable in `main` so it lives until `main` returns.
///
/// ```rust,ignore
/// let _tracing_guard = tracing_foundation::init(config)?;
/// // _tracing_guard lives until main returns
/// ```
pub struct TracingGuard {
    /// Keeps the non-blocking `tracing-appender` worker thread alive.
    ///
    /// When this guard is dropped the worker flushes any remaining events and
    /// exits. The field is intentionally unused beyond keeping it alive.
    _file_guard: tracing_appender::non_blocking::WorkerGuard,

    /// The name of the log file that was opened (directory-relative).
    ///
    /// Available for callers to log or expose after init. This is the same
    /// value that `main.rs` previously printed in the first `tracing::info!`
    /// call after subscriber init.
    pub log_filename: String,
}

impl TracingGuard {
    /// Construct a new guard from the worker guard returned by
    /// `tracing-appender` and the resolved log filename.
    pub(super) fn new(
        file_guard: tracing_appender::non_blocking::WorkerGuard,
        log_filename: String,
    ) -> Self {
        Self {
            _file_guard: file_guard,
            log_filename,
        }
    }
}
