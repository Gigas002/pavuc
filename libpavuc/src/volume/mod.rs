//! Volume math shared by the library and UI.
//!
//! PulseAudio represents volume as an opaque integer where
//! [`NORMAL`] (`0x10000`) means 100% (0 dB / unattenuated software volume).
//! pavucontrol displays and edits volume as a linear percentage of that
//! reference, allowing values above 100% up to [`UI_MAX_PERCENT`]. We mirror
//! that behaviour exactly so the TUI matches pavucontrol.

/// Raw value corresponding to 100% volume (`PA_VOLUME_NORM`).
pub const NORMAL: u32 = 0x1_0000;

/// Raw value corresponding to a muted channel (`PA_VOLUME_MUTED`).
pub const MUTED: u32 = 0;

/// Largest valid raw volume (`PA_VOLUME_MAX`); the server rejects anything above.
pub const MAX: u32 = u32::MAX / 2;

/// Maximum percentage exposed in the UI, matching pavucontrol's `PA_VOLUME_UI_MAX`
/// (roughly +11 dB).
pub const UI_MAX_PERCENT: u32 = 153;

/// Converts a raw PulseAudio volume into a rounded percentage of [`NORMAL`].
#[must_use]
pub fn raw_to_percent(raw: u32) -> u32 {
    ((u64::from(raw) * 100 + u64::from(NORMAL) / 2) / u64::from(NORMAL)) as u32
}

/// Converts a percentage of [`NORMAL`] into a raw PulseAudio volume,
/// saturating at [`MAX`].
#[must_use]
pub fn percent_to_raw(percent: u32) -> u32 {
    let raw = (u64::from(percent) * u64::from(NORMAL)) / 100;
    u32::try_from(raw).map_or(MAX, |raw| raw.min(MAX))
}

/// Clamps a percentage to the inclusive range `0..=UI_MAX_PERCENT`.
#[must_use]
pub fn clamp_percent(percent: i64) -> u32 {
    percent.clamp(0, i64::from(UI_MAX_PERCENT)) as u32
}

/// Moves `current` by `delta` percentage points, clamped to
/// `0..=UI_MAX_PERCENT`.
///
/// A volume already above [`UI_MAX_PERCENT`] (set by another program) is
/// never pulled down by a raise, and a cut from it goes down one step instead
/// of snapping to the UI maximum.
#[must_use]
pub fn step_percent(current: u32, delta: i64) -> u32 {
    let ceiling = i64::from(current.max(UI_MAX_PERCENT));
    let stepped = (i64::from(current) + delta).clamp(0, ceiling);
    u32::try_from(stepped).unwrap_or(u32::MAX)
}

/// Scales per-channel `volumes` so the loudest channel lands on `percent`,
/// keeping the balance between channels (like `pa_cvolume_scale`).
///
/// When every channel is silent there is no balance to keep, so all channels
/// are set to `percent`. An empty slice gives one channel.
#[must_use]
pub fn scale(volumes: &[u32], percent: u32) -> Vec<u32> {
    let target = u64::from(percent_to_raw(percent));
    let max = u64::from(max(volumes));
    if max == 0 {
        return vec![percent_to_raw(percent); volumes.len().max(1)];
    }
    volumes
        .iter()
        .map(|&v| {
            let scaled = (u64::from(v) * target + max / 2) / max;
            u32::try_from(scaled).map_or(MAX, |raw| raw.min(MAX))
        })
        .collect()
}

/// The loudest channel's raw volume (`pa_cvolume_max`), or 0 for no channels.
#[must_use]
pub fn max(volumes: &[u32]) -> u32 {
    volumes.iter().copied().max().unwrap_or(MUTED)
}

#[cfg(test)]
mod tests;
