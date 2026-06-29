//! Data model mirroring the objects pavucontrol displays.
//!
//! Each type is a plain, owned snapshot translated from the corresponding
//! `libpulse` introspection struct, so the UI never has to touch raw FFI data.

use std::collections::HashMap;

use libpulse_binding::context::introspect::{
    CardInfo, SinkInfo, SinkInputInfo, SourceInfo, SourceOutputInfo,
};
use libpulse_binding::def::{SinkState, SourceState};
use libpulse_binding::proplist::{Proplist, properties};
use libpulse_binding::volume::ChannelVolumes;

use crate::volume;

/// Runtime state of an output (sink) or input (source) device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Actively in use by a non-corked stream.
    Running,
    /// Available but currently idle.
    Idle,
    /// Suspended to save power.
    Suspended,
    /// State could not be determined.
    Invalid,
}

impl From<SinkState> for DeviceState {
    fn from(value: SinkState) -> Self {
        match value {
            SinkState::Running => Self::Running,
            SinkState::Idle => Self::Idle,
            SinkState::Suspended => Self::Suspended,
            SinkState::Invalid => Self::Invalid,
        }
    }
}

impl From<SourceState> for DeviceState {
    fn from(value: SourceState) -> Self {
        match value {
            SourceState::Running => Self::Running,
            SourceState::Idle => Self::Idle,
            SourceState::Suspended => Self::Suspended,
            SourceState::Invalid => Self::Invalid,
        }
    }
}

impl DeviceState {
    /// Short human-readable label, matching the words pavucontrol uses.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Idle => "Idle",
            Self::Suspended => "Suspended",
            Self::Invalid => "Invalid",
        }
    }
}

/// A selectable port on a device (e.g. "Speakers", "Headphones").
#[derive(Debug, Clone)]
pub struct Port {
    /// Opaque identifier passed back to the server when selecting the port.
    pub name: String,
    /// Human-readable description shown in the UI.
    pub description: String,
    /// Whether the port is currently available (e.g. something is plugged in).
    pub available: bool,
}

/// A sink (output) or source (input) device.
#[derive(Debug, Clone)]
pub struct Device {
    /// Server-assigned index.
    pub index: u32,
    /// Stable device name (used for setting defaults).
    pub name: String,
    /// Friendly description shown to the user.
    pub description: String,
    /// Number of channels in [`Self::volume`].
    pub channels: u8,
    /// Per-channel raw volumes.
    pub volume: Vec<u32>,
    /// Whether the device is muted.
    pub mute: bool,
    /// Reference ("base") volume of the device, as a raw value.
    pub base_volume: u32,
    /// Owning card index, if any.
    pub card: Option<u32>,
    /// Ports exposed by the device.
    pub ports: Vec<Port>,
    /// Name of the currently active port, if any.
    pub active_port: Option<String>,
    /// Whether this is a monitor source (recording loopback of a sink).
    pub monitor: bool,
    /// Current device state.
    pub state: DeviceState,
}

impl Device {
    /// Average volume across channels as a rounded percentage.
    #[must_use]
    pub fn volume_percent(&self) -> u32 {
        volume::raw_to_percent(average(&self.volume))
    }

    pub(crate) fn from_sink(info: &SinkInfo) -> Self {
        Self {
            index: info.index,
            name: cow_string(&info.name),
            description: cow_string(&info.description),
            channels: info.volume.len(),
            volume: channels_to_vec(&info.volume),
            mute: info.mute,
            base_volume: info.base_volume.0,
            card: info.card,
            ports: info.ports.iter().map(port_from_sink).collect(),
            active_port: info.active_port.as_ref().and_then(|p| opt_cow(&p.name)),
            monitor: false,
            state: info.state.into(),
        }
    }

    pub(crate) fn from_source(info: &SourceInfo) -> Self {
        Self {
            index: info.index,
            name: cow_string(&info.name),
            description: cow_string(&info.description),
            channels: info.volume.len(),
            volume: channels_to_vec(&info.volume),
            mute: info.mute,
            base_volume: info.base_volume.0,
            card: info.card,
            ports: info.ports.iter().map(port_from_source).collect(),
            active_port: info.active_port.as_ref().and_then(|p| opt_cow(&p.name)),
            monitor: info.monitor_of_sink.is_some(),
            state: info.state.into(),
        }
    }
}

/// A playback stream (sink input) or recording stream (source output).
#[derive(Debug, Clone)]
pub struct Stream {
    /// Server-assigned index.
    pub index: u32,
    /// Best display name, e.g. "Firefox: YouTube".
    pub name: String,
    /// Application name from the stream's property list.
    pub app_name: String,
    /// Media (track/title) name from the stream's property list.
    pub media_name: String,
    /// Index of the device this stream is routed to (sink or source).
    pub device: u32,
    /// Owning client index, if any.
    pub client: Option<u32>,
    /// Number of channels in [`Self::volume`].
    pub channels: u8,
    /// Per-channel raw volumes.
    pub volume: Vec<u32>,
    /// Whether the stream is muted.
    pub mute: bool,
    /// Whether the stream exposes a meaningful volume.
    pub has_volume: bool,
    /// Whether the stream's volume can be changed.
    pub volume_writable: bool,
    /// Whether the stream is corked (paused).
    pub corked: bool,
}

impl Stream {
    /// Average volume across channels as a rounded percentage.
    #[must_use]
    pub fn volume_percent(&self) -> u32 {
        volume::raw_to_percent(average(&self.volume))
    }

    pub(crate) fn from_sink_input(info: &SinkInputInfo) -> Self {
        let app_name = proplist_app_name(&info.proplist, &info.name);
        let media_name = proplist_media_name(&info.proplist, &info.name);
        Self {
            index: info.index,
            name: display_name(&app_name, &media_name),
            app_name,
            media_name,
            device: info.sink,
            client: info.client,
            channels: info.volume.len(),
            volume: channels_to_vec(&info.volume),
            mute: info.mute,
            has_volume: info.has_volume,
            volume_writable: info.volume_writable,
            corked: info.corked,
        }
    }

    pub(crate) fn from_source_output(info: &SourceOutputInfo) -> Self {
        let app_name = proplist_app_name(&info.proplist, &info.name);
        let media_name = proplist_media_name(&info.proplist, &info.name);
        Self {
            index: info.index,
            name: display_name(&app_name, &media_name),
            app_name,
            media_name,
            device: info.source,
            client: info.client,
            channels: info.volume.len(),
            volume: channels_to_vec(&info.volume),
            mute: info.mute,
            has_volume: info.has_volume,
            volume_writable: info.volume_writable,
            corked: info.corked,
        }
    }
}

/// A configurable profile on a card (e.g. "Analog Stereo Duplex").
#[derive(Debug, Clone)]
pub struct Profile {
    /// Opaque identifier passed back to the server when selecting the profile.
    pub name: String,
    /// Human-readable description shown in the UI.
    pub description: String,
    /// Whether the profile is currently available.
    pub available: bool,
}

/// A sound card grouping one or more sinks and sources.
#[derive(Debug, Clone)]
pub struct Card {
    /// Server-assigned index.
    pub index: u32,
    /// Stable card name.
    pub name: String,
    /// Friendly description shown to the user.
    pub description: String,
    /// Profiles offered by the card.
    pub profiles: Vec<Profile>,
    /// Name of the currently active profile, if any.
    pub active_profile: Option<String>,
}

impl Card {
    pub(crate) fn from_info(info: &CardInfo) -> Self {
        let description = info
            .proplist
            .get_str(properties::DEVICE_DESCRIPTION)
            .unwrap_or_else(|| cow_string(&info.name));
        Self {
            index: info.index,
            name: cow_string(&info.name),
            description,
            profiles: info
                .profiles
                .iter()
                .map(|p| Profile {
                    name: cow_string(&p.name),
                    description: cow_string(&p.description),
                    available: p.available,
                })
                .collect(),
            active_profile: info.active_profile.as_ref().and_then(|p| opt_cow(&p.name)),
        }
    }
}

/// A complete, owned snapshot of the server's relevant state.
#[derive(Debug, Default, Clone)]
pub struct PulseState {
    /// Output devices.
    pub sinks: Vec<Device>,
    /// Input devices.
    pub sources: Vec<Device>,
    /// Playback streams.
    pub sink_inputs: Vec<Stream>,
    /// Recording streams.
    pub source_outputs: Vec<Stream>,
    /// Sound cards.
    pub cards: Vec<Card>,
    /// Client index to friendly client name.
    pub clients: HashMap<u32, String>,
    /// Name of the default sink.
    pub default_sink: Option<String>,
    /// Name of the default source.
    pub default_source: Option<String>,
}

impl PulseState {
    /// Looks up a sink by index.
    #[must_use]
    pub fn sink(&self, index: u32) -> Option<&Device> {
        self.sinks.iter().find(|d| d.index == index)
    }

    /// Looks up a source by index.
    #[must_use]
    pub fn source(&self, index: u32) -> Option<&Device> {
        self.sources.iter().find(|d| d.index == index)
    }

    /// Returns the friendly client name for the given client index, if known.
    #[must_use]
    pub fn client_name(&self, client: Option<u32>) -> Option<&str> {
        client
            .and_then(|c| self.clients.get(&c))
            .map(String::as_str)
    }

    /// Whether the given sink is the default sink.
    #[must_use]
    pub fn is_default_sink(&self, device: &Device) -> bool {
        self.default_sink.as_deref() == Some(device.name.as_str())
    }

    /// Whether the given source is the default source.
    #[must_use]
    pub fn is_default_source(&self, device: &Device) -> bool {
        self.default_source.as_deref() == Some(device.name.as_str())
    }
}

fn average(volume: &[u32]) -> u32 {
    if volume.is_empty() {
        return 0;
    }
    let sum: u64 = volume.iter().map(|&v| u64::from(v)).sum();
    (sum / volume.len() as u64) as u32
}

fn channels_to_vec(cv: &ChannelVolumes) -> Vec<u32> {
    cv.get().iter().map(|v| v.0).collect()
}

fn cow_string(value: &Option<std::borrow::Cow<'_, str>>) -> String {
    value.as_ref().map_or_else(String::new, |c| c.to_string())
}

fn opt_cow(value: &Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    value.as_ref().map(ToString::to_string)
}

fn port_from_sink(p: &libpulse_binding::context::introspect::SinkPortInfo) -> Port {
    Port {
        name: cow_string(&p.name),
        description: cow_string(&p.description),
        available: !matches!(p.available, libpulse_binding::def::PortAvailable::No),
    }
}

fn port_from_source(p: &libpulse_binding::context::introspect::SourcePortInfo) -> Port {
    Port {
        name: cow_string(&p.name),
        description: cow_string(&p.description),
        available: !matches!(p.available, libpulse_binding::def::PortAvailable::No),
    }
}

fn proplist_app_name(proplist: &Proplist, fallback: &Option<std::borrow::Cow<'_, str>>) -> String {
    proplist
        .get_str(properties::APPLICATION_NAME)
        .or_else(|| proplist.get_str(properties::APPLICATION_PROCESS_BINARY))
        .unwrap_or_else(|| cow_string(fallback))
}

fn proplist_media_name(
    proplist: &Proplist,
    fallback: &Option<std::borrow::Cow<'_, str>>,
) -> String {
    proplist
        .get_str(properties::MEDIA_NAME)
        .unwrap_or_else(|| cow_string(fallback))
}

fn display_name(app_name: &str, media_name: &str) -> String {
    match (app_name.is_empty(), media_name.is_empty()) {
        (false, false) if app_name != media_name => format!("{app_name}: {media_name}"),
        (false, _) => app_name.to_string(),
        (true, false) => media_name.to_string(),
        (true, true) => "Unknown stream".to_string(),
    }
}
