//! Unit tests for [`super::PulseClient`] and its private helpers.
//!
//! Most of the client talks to a live PulseAudio/PipeWire server over `libpulse`.
//! Those paths are covered by the ignored smoke test below (run manually when a
//! server is available). Pure helpers are tested here without I/O.

use libpulse_binding::volume::ChannelVolumes;

use super::{PulseClient, channel_volumes};
use crate::volume;

#[test]
fn connect_fn_matches_public_api() {
    let _connect: fn(&str) -> Result<PulseClient, crate::Error> = PulseClient::connect;
}

#[test]
fn channel_volumes_keeps_each_channel() {
    let cv = channel_volumes(&[volume::NORMAL, volume::NORMAL / 2]);
    assert_eq!(cv.len(), 2);
    assert_eq!(cv.get()[0].0, volume::NORMAL);
    assert_eq!(cv.get()[1].0, volume::NORMAL / 2);
}

#[test]
fn channel_volumes_turns_empty_into_one_muted_channel() {
    let cv = channel_volumes(&[]);
    assert_eq!(cv.len(), 1);
    assert_eq!(cv.get()[0].0, volume::MUTED);
}

#[test]
fn channel_volumes_caps_at_channels_max() {
    let raw = vec![42; 64];
    let cv = channel_volumes(&raw);
    assert_eq!(cv.len(), ChannelVolumes::CHANNELS_MAX);
    assert!(cv.get().iter().all(|v| v.0 == 42));
}

/// Manual smoke test — requires PulseAudio or `pipewire-pulse` running.
///
/// ```sh
/// cargo test -p libpavuc connect_snapshot_smoke -- --ignored --nocapture
/// ```
#[test]
#[ignore = "requires a running PulseAudio or pipewire-pulse server"]
fn connect_snapshot_smoke() {
    let mut client = PulseClient::connect("libpavuc-test").expect("connect");
    for _ in 0..20 {
        client.iterate().expect("iterate");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let state = client.snapshot();
    // We only assert the call graph works; device counts vary by machine.
    let _ = (
        state.sinks.len(),
        state.sources.len(),
        state.sink_inputs.len(),
        state.source_outputs.len(),
        state.cards.len(),
    );
}
