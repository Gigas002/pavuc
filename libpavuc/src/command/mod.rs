//! Commands a UI can send to the audio server.
//!
//! A [`Command`] is a plain value describing one change. Building commands is
//! pure, so an application can decide *what* to do (and be unit-tested)
//! without a server; [`crate::PulseClient::execute`] then does it.

/// One change to apply on the audio server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Set a sink's (output device's) volume, one raw value per channel.
    SetSinkVolume {
        /// Sink index.
        index: u32,
        /// Raw per-channel volumes.
        volume: Vec<u32>,
    },
    /// Mute or unmute a sink.
    SetSinkMute {
        /// Sink index.
        index: u32,
        /// New mute state.
        mute: bool,
    },
    /// Set a source's (input device's) volume, one raw value per channel.
    SetSourceVolume {
        /// Source index.
        index: u32,
        /// Raw per-channel volumes.
        volume: Vec<u32>,
    },
    /// Mute or unmute a source.
    SetSourceMute {
        /// Source index.
        index: u32,
        /// New mute state.
        mute: bool,
    },
    /// Set a playback stream's (sink input's) volume, one raw value per channel.
    SetSinkInputVolume {
        /// Sink input index.
        index: u32,
        /// Raw per-channel volumes.
        volume: Vec<u32>,
    },
    /// Mute or unmute a playback stream.
    SetSinkInputMute {
        /// Sink input index.
        index: u32,
        /// New mute state.
        mute: bool,
    },
    /// Set a recording stream's (source output's) volume, one raw value per channel.
    SetSourceOutputVolume {
        /// Source output index.
        index: u32,
        /// Raw per-channel volumes.
        volume: Vec<u32>,
    },
    /// Mute or unmute a recording stream.
    SetSourceOutputMute {
        /// Source output index.
        index: u32,
        /// New mute state.
        mute: bool,
    },
    /// Make the named sink the default output.
    SetDefaultSink(String),
    /// Make the named source the default input.
    SetDefaultSource(String),
    /// Move a playback stream to another sink.
    MoveSinkInput {
        /// Sink input index.
        index: u32,
        /// Destination sink index.
        sink: u32,
    },
    /// Move a recording stream to another source.
    MoveSourceOutput {
        /// Source output index.
        index: u32,
        /// Destination source index.
        source: u32,
    },
    /// Select a card's active profile.
    SetCardProfile {
        /// Card index.
        card: u32,
        /// Profile name.
        profile: String,
    },
    /// Select a sink's active port.
    SetSinkPort {
        /// Sink index.
        index: u32,
        /// Port name.
        port: String,
    },
    /// Select a source's active port.
    SetSourcePort {
        /// Source index.
        index: u32,
        /// Port name.
        port: String,
    },
    /// Terminate a playback stream.
    KillSinkInput(u32),
    /// Terminate a recording stream.
    KillSourceOutput(u32),
}

#[cfg(test)]
mod tests;
