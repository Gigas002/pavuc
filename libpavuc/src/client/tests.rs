//! Unit tests for [`super::PulseClient`] and its private helpers.
//!
//! Most of the client talks to a live PulseAudio/PipeWire server over `libpulse`.
//! Those paths are covered by the ignored smoke test below (run manually when a
//! server is available). Pure helpers are tested here without I/O.

use super::{PulseClient, uniform_volume};
use crate::volume;

#[test]
fn connect_fn_matches_public_api() {
    let _connect: fn(&str) -> Result<PulseClient, crate::Error> = PulseClient::connect;
}

#[test]
fn uniform_volume_clamps_zero_channels_to_one() {
    let cv = uniform_volume(0, volume::NORMAL);
    assert_eq!(cv.len(), 1);
    assert_eq!(cv.get()[0].0, volume::NORMAL);
}

#[test]
fn uniform_volume_sets_all_channels() {
    let cv = uniform_volume(2, volume::percent_to_raw(50));
    assert_eq!(cv.len(), 2);
    for channel in cv.get() {
        assert_eq!(channel.0, volume::percent_to_raw(50));
    }
}

#[test]
fn uniform_volume_uses_raw_value_directly() {
    let cv = uniform_volume(1, 42_000);
    assert_eq!(cv.get()[0].0, 42_000);
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
