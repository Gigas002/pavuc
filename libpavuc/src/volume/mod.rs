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

/// Maximum percentage exposed in the UI, matching pavucontrol's `PA_VOLUME_UI_MAX`
/// (roughly +11 dB).
pub const UI_MAX_PERCENT: u32 = 153;

/// Converts a raw PulseAudio volume into a rounded percentage of [`NORMAL`].
#[must_use]
pub fn raw_to_percent(raw: u32) -> u32 {
    ((u64::from(raw) * 100 + u64::from(NORMAL) / 2) / u64::from(NORMAL)) as u32
}

/// Converts a percentage of [`NORMAL`] into a raw PulseAudio volume.
#[must_use]
pub fn percent_to_raw(percent: u32) -> u32 {
    ((u64::from(percent) * u64::from(NORMAL)) / 100) as u32
}

/// Clamps a percentage to the inclusive range `0..=UI_MAX_PERCENT`.
#[must_use]
pub fn clamp_percent(percent: i64) -> u32 {
    percent.clamp(0, i64::from(UI_MAX_PERCENT)) as u32
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
            u32::try_from(scaled).unwrap_or(u32::MAX)
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
