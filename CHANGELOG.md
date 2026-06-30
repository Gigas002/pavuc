# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- `libpavuc`: a library wrapping the PulseAudio/PipeWire client API (`libpulse`)
  behind an owned snapshot model (`PulseState`) and a command interface
  (`PulseClient`) covering volume, mute, default device, stream routing, port
  selection and card profiles.
- `pavuc`: a ratatui terminal UI that is a pavucontrol analogue, with the
  same five tabs (Playback, Recording, Output Devices, Input Devices,
  Configuration), live updates, volume bars and modal pickers.
- `libpavuc/examples/list.rs`: a one-shot snapshot example / smoke test.
