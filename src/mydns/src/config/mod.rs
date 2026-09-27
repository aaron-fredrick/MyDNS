pub mod ini;
#[cfg(test)]
mod tests;
pub mod toml;
pub mod types;

pub use types::{default_root_hints, AppConfig, ResolverMode, ResolverPriority, IANA_ROOT_HINTS};

use std::path::Path;

impl AppConfig {
    /// Loads configuration from `config.toml`, falling back to `config.ini` if absent.
    pub fn from_config_file() -> anyhow::Result<Self> {
        if Path::new("config.toml").exists() {
            Self::from_toml_file(Path::new("config.toml"))
        } else if Path::new("config.ini").exists() {
            Self::from_ini_file(Path::new("config.ini"))
        } else {
            Err(anyhow::anyhow!(
                "Configuration file not found. Please create config.toml (see config.toml.example)"
            ))
        }
    }
}

pub fn generate_secret(len: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
