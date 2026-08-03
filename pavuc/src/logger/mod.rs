//! `tracing` subscriber initialization from resolved settings.

use tracing_subscriber::EnvFilter;

use crate::settings::Settings;

/// Initializes the global tracing subscriber from `settings`.
pub fn init(settings: &Settings) {
    let filter = EnvFilter::new(settings.log_level.as_str());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(atty::is_stderr_tty())
        .init();
}

/// Whether stderr is connected to a terminal (for ANSI colour).
mod atty {
    #[must_use]
    pub fn is_stderr_tty() -> bool {
        use std::io::IsTerminal;
        std::io::stderr().is_terminal()
    }
}

#[cfg(test)]
mod tests;
