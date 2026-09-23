//! Logging configuration.

use chrono::Local;

/// Configuration for the MyDNS logging pipeline.
///
/// This is the single point that controls the subscriber setup: where log files
/// are written, what filter level is applied, and what the log file is named.
/// All fields have sensible defaults that replicate the previous behaviour
/// in `main.rs`.
pub struct LoggingConfig {
    /// Directory under which log files are written.
    ///
    /// Created automatically by [`super::super::pipeline::init`] if it does not
    /// exist.
    pub log_directory: String,

    /// Name of the log file within [`log_directory`](Self::log_directory).
    ///
    /// Defaults to a timestamped name: `mydns_YYYY-MM-DD_HH-MM-SS.log`.
    pub log_filename: String,

    /// `tracing-subscriber` filter directive.
    ///
    /// Reads from the `RUST_LOG` environment variable when present, falling
    /// back to this value when the variable is absent or invalid. The default
    /// is `"info"`, which matches the previous hard-coded fallback.
    pub env_filter_fallback: String,
}

impl LoggingConfig {
    /// Create a config with a freshly-generated timestamped log filename.
    ///
    /// This is equivalent to what `main.rs` previously computed inline:
    /// ```text
    /// let log_filename = format!("mydns_{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    /// ```
    pub fn new() -> Self {
        let log_filename = format!("mydns_{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S"));
        Self {
            log_directory: "logs".to_string(),
            log_filename,
            env_filter_fallback: "info".to_string(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_directory_is_logs() {
        let cfg = LoggingConfig::default();
        assert_eq!(cfg.log_directory, "logs");
    }

    #[test]
    fn default_log_filename_has_expected_prefix_and_suffix() {
        let cfg = LoggingConfig::default();
        assert!(
            cfg.log_filename.starts_with("mydns_"),
            "filename should start with 'mydns_': {}",
            cfg.log_filename
        );
        assert!(
            cfg.log_filename.ends_with(".log"),
            "filename should end with '.log': {}",
            cfg.log_filename
        );
    }

    #[test]
    fn default_env_filter_fallback_is_info() {
        let cfg = LoggingConfig::default();
        assert_eq!(cfg.env_filter_fallback, "info");
    }
}
