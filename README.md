# pavuc

A **[pavucontrol](https://freedesktop.org/software/pulseaudio/pavucontrol/) analogue** TUI built with [ratatui](https://ratatui.rs).

pavucontrol is a PulseAudio client. On modern systems PipeWire ships
`pipewire-pulse`, a drop-in PulseAudio server, so the very same client API
(`libpulse`) drives **both PulseAudio and PipeWire** transparently. `pavuc`
speaks that API and mirrors pavucontrol's five tabs in your terminal.

```
┌ pavuc — PulseAudio/PipeWire volume control ────────────────────────────────┐
│  1:Playback   2:Recording   3:Output Devices   4:Input Devices   5:Config   │
└─────────────────────────────────────────────────────────────────────────────┘
┌ Razer Kraken V3 Pro Analog Stereo ───────────────────────────── ★ default ──┐
│ Port: Headphones   [Running]                                                │
│ ███████████████████████████░░░░░░░░░░░░░░░░░░░  75%                          │
└─────────────────────────────────────────────────────────────────────────────┘
┌ USB Audio Speakers ─────────────────────────────────────────────────────────┐
│ Port: Speakers   [Idle]                                                     │
│ ██████████████████████████████████░░░░░░░░░░░  100%                          │
└─────────────────────────────────────────────────────────────────────────────┘
 Tab switch   ↑↓ select   ←→ vol   m mute   d default   Enter port   q quit
```

## Workspace layout

| Crate          | Description                                                                 |
| -------------- | --------------------------------------------------------------------------- |
| **`libpavuc`** | Library wrapping `libpulse` behind a clean, UI-friendly snapshot + commands |
| **`pavuc`**    | The ratatui terminal UI binary                                              |

## Features

A faithful, keyboard-driven port of pavucontrol's functionality:

- **Playback** — per-application streams with volume bars, mute, routing to a
  different output (`Enter`), and the ability to kill a stream (`x`).
- **Recording** — per-application capture streams with volume, mute and routing
  to a different input.
- **Output Devices** — sinks with volume, mute, set-as-default (`d`) and port
  selection (`Enter`).
- **Input Devices** — sources with volume, mute, set-as-default and port
  selection.
- **Configuration** — sound cards with profile selection (`Enter`).
- Live updates: the view reflects changes made by other apps in real time via
  PulseAudio's subscription events.

## Requirements

- A running **PulseAudio** server **or PipeWire with `pipewire-pulse`**.
- `libpulse` at build time (Debian/Ubuntu: `libpulse-dev`, Arch: `libpulse`,
  Fedora: `pulseaudio-libs-devel`).

## Build & run

```sh
cargo run -p pavuc
```

Or install the binary:

```sh
cargo install --path pavuc
pavuc
```

## Keybindings

| Key                          | Action                                              |
| ---------------------------- | --------------------------------------------------- |
| `1`–`5`, `Tab` / `Shift+Tab` | Switch tabs                                         |
| `↑`/`↓` or `k`/`j`           | Move selection                                      |
| `←`/`→` or `h`/`l`           | Volume −/+ 5%                                       |
| `<`/`>` or `,`/`.`           | Volume −/+ 1% (fine)                                |
| `m`                          | Toggle mute                                         |
| `d`                          | Set device as default (Output/Input tabs)           |
| `Enter`                      | Route stream / select port / select profile (popup) |
| `x`                          | Kill the selected stream (Playback/Recording)       |
| `q` / `Esc`                  | Quit (or close an open popup)                       |

## Development

```sh
cargo build --workspace          # build everything
cargo test --workspace           # run tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
