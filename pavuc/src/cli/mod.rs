//! Argument and subcommand definitions for the pavuc binary.

use std::path::PathBuf;

use clap::Parser;

/// A pavucontrol analogue TUI for PulseAudio/PipeWire.
#[derive(Debug, Parser)]
#[command(name = "pavuc", version, about)]
pub struct CliOptions {
    /// Path to a TOML configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log level (`error`, `warn`, `info`, `debug`, `trace`).
    #[arg(short, long, value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// Application name reported to the audio server.
    #[arg(long, value_name = "NAME")]
    pub client_name: Option<String>,

    /// Event-loop tick interval in milliseconds.
    #[arg(long, value_name = "MS")]
    pub tick_ms: Option<u64>,
}

#[cfg(test)]
mod tests;
