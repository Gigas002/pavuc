//! Unified settings resolver: CLI > config file > defaults.

use std::time::Duration;

use tracing::Level;

use crate::cli::CliOptions;
use crate::config::FileConfig;

/// Allowed range for the event-loop tick, in milliseconds. Below the minimum
/// the loop busy-waits (0 ms spins a core at 100%); above the maximum changes
/// made by other programs take too long to show up.
pub const TICK_MS_RANGE: std::ops::RangeInclusive<u64> = 10..=1000;

/// Fully resolved runtime settings consumed below the resolver boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Application name reported to the audio server.
    pub client_name: String,
    /// Maximum log level emitted to stderr.
    pub log_level: Level,
    /// How long to wait for input before redrawing.
    pub tick: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_name: "pavuc".to_string(),
            log_level: Level::WARN,
            tick: Duration::from_millis(100),
        }
    }
}

/// Merges CLI options and optional file config over built-in defaults.
///
/// # Errors
///
/// Returns an error if `log_level` cannot be parsed or `tick_ms` is outside
/// [`TICK_MS_RANGE`].
pub fn resolve(cli: &CliOptions, file: Option<FileConfig>) -> Result<Settings, String> {
    let mut settings = Settings::default();

    if let Some(file) = file {
        apply_file(&mut settings, &file)?;
    }

    apply_cli(&mut settings, cli)?;
    Ok(settings)
}

fn apply_file(settings: &mut Settings, file: &FileConfig) -> Result<(), String> {
    if let Some(name) = &file.client_name {
        settings.client_name = name.clone();
    }
    if let Some(level) = &file.log_level {
        settings.log_level = parse_level(level)?;
    }
    if let Some(ms) = file.tick_ms {
        settings.tick = parse_tick(ms)?;
    }
    Ok(())
}

fn apply_cli(settings: &mut Settings, cli: &CliOptions) -> Result<(), String> {
    if let Some(name) = &cli.client_name {
        settings.client_name = name.clone();
    }
    if let Some(level) = &cli.log_level {
        settings.log_level = parse_level(level)?;
    }
    if let Some(ms) = cli.tick_ms {
        settings.tick = parse_tick(ms)?;
    }
    Ok(())
}

fn parse_tick(ms: u64) -> Result<Duration, String> {
    if TICK_MS_RANGE.contains(&ms) {
        Ok(Duration::from_millis(ms))
    } else {
        Err(format!(
            "invalid tick_ms: {ms} (must be between {} and {})",
            TICK_MS_RANGE.start(),
            TICK_MS_RANGE.end()
        ))
    }
}

fn parse_level(value: &str) -> Result<Level, String> {
    value
        .parse()
        .map_err(|_| format!("invalid log level: {value}"))
}

#[cfg(test)]
mod tests;
