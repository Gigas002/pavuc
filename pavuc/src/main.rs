//! pavuc — a pavucontrol analogue TUI, built with ratatui.
//!
//! Connects to PulseAudio (or PipeWire via `pipewire-pulse`) and presents the
//! same five tabs pavucontrol does: Playback, Recording, Output Devices,
//! Input Devices and Configuration.

mod app;
mod cli;
mod config;
mod logger;
mod settings;
mod ui;
mod utils;

use clap::Parser;

use cli::CliOptions;

fn main() {
    let cli = CliOptions::parse();
    let file_config = load_file_config(&cli);

    let settings = match settings::resolve(&cli, file_config) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("pavuc: {error}");
            std::process::exit(1);
        }
    };

    logger::init(&settings);

    if let Err(error) = app::run(settings) {
        tracing::error!("{error}");
        std::process::exit(1);
    }
}

fn load_file_config(cli: &CliOptions) -> Option<config::FileConfig> {
    let path = cli.config.clone().or_else(config::conventional_path)?;

    match config::load(&path) {
        Ok(file_config) => Some(file_config),
        Err(error) => {
            eprintln!("pavuc: {error}");
            std::process::exit(1);
        }
    }
}
