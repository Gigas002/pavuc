# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

Changes from the audit in [docs/AUDIT.md](docs/AUDIT.md).

### Breaking (libpavuc)

- `PulseClient`'s per-operation methods (`set_sink_volume`, `move_sink_input`,
  ...) are replaced by `PulseClient::execute(Command)`. Volume commands carry
  raw per-channel volumes.
- `PulseClient::iterate` returns whether the snapshot changed.
- `Device::volume_percent` / `Stream::volume_percent` report the loudest
  channel (as pavucontrol does) instead of the average.

### Added

- `libpavuc`: `Command`, `PulseClient::connect_to`, `PulseClient::take_failures`,
  `volume::scale`, `volume::step_percent`, `volume::max`, `volume::MAX`.
- `libpavuc/examples/list.rs`, listed under 0.1.0 but missing until now.
- `pavuc`: reconnects when the audio server restarts.
- `pavuc`: refused commands are shown in the status line.
- `pavuc`: `--log-file` / `log_file`; Ctrl+C quits.

### Fixed

- Volume steps kept channel balance only when channels were equal; an
  unbalanced stream was flattened to its average.
- `+` on a volume above 153% lowered it to 153%.
- The selection jumped to another stream when one above it went away.
- The Recording route popup hid monitor sources and preselected the wrong one.
- Crash on terminals shorter than one card.
- A popup outliving its stream or device sent commands for a dead object.
- Connection errors said "Access denied" instead of the real reason, and
  were printed three times.
- Log lines drew over the TUI.
- `tick_ms = 0` spun a core at 100%; unknown config keys were ignored.
- Crate metadata pointed at a repository that doesn't exist.

### Performance

- Only the kind of object a server event names is re-fetched, and the UI
  redraws only when something changed.

## 0.1.0 - 2026-09-04

### Added

- `libpavuc`: a library wrapping the PulseAudio/PipeWire client API (`libpulse`)
  behind an owned snapshot model (`PulseState`) and a command interface
  (`PulseClient`) covering volume, mute, default device, stream routing, port
  selection and card profiles.
- `pavuc`: a ratatui terminal UI that is a pavucontrol analogue, with the
  same five tabs (Playback, Recording, Output Devices, Input Devices,
  Configuration), live updates, volume bars and modal pickers.
- `libpavuc/examples/list.rs`: a one-shot snapshot example / smoke test.
