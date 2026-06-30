//! TOML configuration discovery, read, and deserialize.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Values deserialized from a configuration file.
///
/// Every field is optional; precedence is applied in [`crate::settings`].
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct FileConfig {
    /// Application name reported to the audio server.
    pub client_name: Option<String>,
    /// Log level string (`error`, `warn`, `info`, `debug`, `trace`).
    pub log_level: Option<String>,
    /// Event-loop tick interval in milliseconds.
    pub tick_ms: Option<u64>,
}

/// Loads and deserializes a TOML configuration file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load(path: &Path) -> Result<FileConfig, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("could not read config file {}: {error}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|error| format!("could not parse config file {}: {error}", path.display()))
}

/// Returns the conventional user config path when it exists.
#[must_use]
pub fn conventional_path() -> Option<PathBuf> {
    let path = dirs_config_path()?;
    path.exists().then_some(path)
}

fn dirs_config_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("pavuc").join("config.toml"))
}

#[cfg(test)]
mod tests;
