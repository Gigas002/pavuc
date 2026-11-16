//! `tracing` subscriber initialization from resolved settings.
//!
//! Logs go to the configured log file, or else to stderr. While the TUI owns
//! the terminal (see [`pause_stderr`]) stderr lines would draw over it, so
//! they are dropped then; use a log file to keep them.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use crate::settings::Settings;

/// Set while the TUI is on screen; stderr logging is skipped then.
static STDERR_PAUSED: AtomicBool = AtomicBool::new(false);

/// Initializes the global tracing subscriber from `settings`.
///
/// # Errors
///
/// Returns an error if the log file cannot be opened.
pub fn init(settings: &Settings) -> Result<(), String> {
    let filter = EnvFilter::new(settings.log_level.as_str());
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match &settings.log_file {
        Some(path) => {
            let file = open_log_file(path)
                .map_err(|error| format!("could not open log file {}: {error}", path.display()))?;
            builder.with_writer(file).with_ansi(false).init();
        }
        None => builder
            .with_writer(io::stderr.with_filter(|_| !stderr_paused()))
            .with_ansi(atty::is_stderr_tty())
            .init(),
    }
    Ok(())
}

/// Stops (`true`) or resumes (`false`) logging to stderr, so log lines don't
/// draw over the TUI.
pub fn pause_stderr(paused: bool) {
    STDERR_PAUSED.store(paused, Ordering::Relaxed);
}

fn stderr_paused() -> bool {
    STDERR_PAUSED.load(Ordering::Relaxed)
}

/// Opens `path` for appending, creating it readable by the user only: logs
/// can hold stream and media names (e.g. browser tab titles).
fn open_log_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
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
