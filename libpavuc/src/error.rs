//! Error type for the library.

use std::fmt;

/// Errors that can occur while talking to the PulseAudio/PipeWire server.
#[derive(Debug)]
pub enum Error {
    /// The PulseAudio main loop could not be created.
    Mainloop,
    /// The PulseAudio context could not be created.
    Context,
    /// Connecting to the server failed (the underlying message, if any).
    Connect(String),
    /// The connection was terminated or failed while running.
    Disconnected,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mainloop => write!(f, "failed to create the PulseAudio main loop"),
            Self::Context => write!(f, "failed to create the PulseAudio context"),
            Self::Connect(msg) => write!(f, "failed to connect to the audio server: {msg}"),
            Self::Disconnected => write!(f, "connection to the audio server was lost"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
