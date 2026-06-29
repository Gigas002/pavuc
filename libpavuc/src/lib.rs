//! `libpavuc` — the PulseAudio/PipeWire control library powering `pavuc`,
//! a TUI analogue of pavucontrol.
//!
//! pavucontrol is a PulseAudio client. On modern systems PipeWire ships
//! `pipewire-pulse`, a drop-in PulseAudio server, so the very same client API
//! (`libpulse`) drives PipeWire transparently. This crate wraps that API behind
//! a small, UI-friendly surface:
//!
//! - [`PulseClient`] owns the connection and exposes commands (set volume,
//!   mute, move streams, change card profiles, select ports, set defaults).
//! - [`PulseState`] is an owned snapshot of everything pavucontrol shows
//!   (sinks, sources, playback/recording streams, and cards).
//!
//! # Example
//!
//! ```no_run
//! use libpavuc::PulseClient;
//!
//! let mut client = PulseClient::connect("my-app")?;
//! loop {
//!     client.iterate()?; // pump the main loop each tick
//!     let state = client.snapshot();
//!     for sink in &state.sinks {
//!         println!("{}: {}%", sink.description, sink.volume_percent());
//!     }
//!     # break;
//! }
//! # Ok::<(), libpavuc::Error>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod error;
mod model;
pub mod volume;

pub use client::PulseClient;
pub use error::{Error, Result};
pub use model::{Card, Device, DeviceState, Port, Profile, PulseState, Stream};
