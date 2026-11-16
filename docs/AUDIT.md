# Audit

A whole-application audit of pavuc and libpavuc (2026-11-16) in six parts:
security (§2), bugs (§3), performance (§4), UI usability and design (§5),
code quality, tests and project hygiene (§6), and features compared with
pavucontrol (§7). §8 lists what was left open, and §9 is the checklist to
re-run the audit.

Each finding says what was found, how serious it is, and what was done:
**fixed** (with the commit), **accepted** (with the reason), or **OK** (checked,
nothing to do). Every fix is its own commit, with a regression test where one
can be written.

Severity: **high** (wrong object changed, data or privacy loss, crash in
normal use), **medium** (a crash or hang in an unusual but reachable state,
behaviour that contradicts what the user asked for), **low** (hardening,
polish, or needs an unlikely setup).

## 1. Method

Every source file was read in full. Each finding below was then confirmed by
running the release build against the live audio server (PipeWire 1.6.11 with
`pipewire-pulse`, protocol 35) inside a `tmux` pane, driven with
`tmux send-keys` and read back with `tmux capture-pane`. To avoid touching the
real speakers, the tests used:

- a null sink (`module-null-sink`, "PavucAudit") with silent `pacat` streams
  from `/dev/zero`, and a `parec` recorder on its monitor;
- a stream whose `media.name` holds terminal escape sequences
  (`ESC]0;PWNED BEL ESC[31m ESC[2J`, a newline);
- a second, private `pipewire-pulse` on its own socket (`PULSE_SERVER=...`)
  that could be killed and restarted to test losing the server;
- a shell loop flipping the null sink's volume as fast as `pactl` can
  (~140 events/s) as an event storm.

All fixtures were removed afterwards. Static checks: `cargo clippy` (the
gates, plus `pedantic` and `nursery` for review), `cargo deny check`, `typos`,
`cargo llvm-cov`.

## 2. Security

pavuc is a local client of a per-user server. It reads the user's own config
file and talks to the audio server over the user's socket. Untrusted input is
what other programs put in the server: stream, device and card names. A
browser, for example, sets a stream's media name from the page title, which any
website controls.

| Area                       | Finding                                                                                                                                                                                                                                                                                                                           | Status                        |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| Escape injection (TUI)     | A stream named with `ESC]0;…BEL` (set title), `ESC[2J` (clear), CSI and newline rendered as inert text: ratatui's buffer drops control characters in every path used (titles, paragraphs, gauge labels, list items). No escape byte reached the terminal (checked in the raw `tmux` pane output). Pinned by a UI regression test. | OK                            |
| Escape injection (example) | The new `list` example prints names to stdout, where nothing filters them. A crafted media name sent raw `ESC]0;` / `ESC[2J` to the terminal (seen with `cat -v`). It now replaces control characters with U+FFFD.                                                                                                                | Fixed (972b431)               |
| Log file                   | The new `log_file` holds stream and media names, i.e. browsing history. It is created `0600` and appended to, never truncated.                                                                                                                                                                                                    | Fixed in design (bd9d676)     |
| `unsafe`                   | None in either crate. `libpavuc` had `#![forbid(unsafe_code)]`; the binary now has it too. All FFI goes through `libpulse-binding`.                                                                                                                                                                                               | Fixed (5add0fd)               |
| FFI drop order             | `PulseClient` declared the main loop first, so it was freed before the `Context` and pending operations made on it. `Drop` disconnects first, which frees the I/O events, so no crash was seen, but libpulse expects the context to go first. Fields are reordered so the main loop drops last.                                   | Fixed, low (fa06c89)          |
| Callback reference cycle   | The new completion callbacks are owned by operations the context keeps; they hold the context and failure list weakly, so no `Rc` cycle leaks the connection.                                                                                                                                                                     | OK                            |
| Config file                | The user's own file; read in full (no size cap) and parsed by `toml`; errors are reported, never panics. Unknown keys are now rejected (§3).                                                                                                                                                                                      | Accepted (low)                |
| Server-controlled sizes    | Volume commands clamp the channel count to `PA_CHANNELS_MAX` (32) before building `ChannelVolumes`, whose setters panic above it. A popup with ≥ 65,534 entries overflowed its `u16` height (debug panic); it now saturates.                                                                                                      | Fixed, low (208bef9, 10c0d79) |
| Dependencies               | `cargo deny check`: advisories, bans, licenses and sources OK. One duplicate (`syn` 2 and 3, via upstream crates), warn-only. CI ran only the licence check; it now runs all four.                                                                                                                                                | Fixed (f659ba4)               |
| Autospawn                  | `PulseClient::connect` uses `NOFLAGS`, so on a PulseAudio system without a running daemon libpulse may autospawn one. Same as pavucontrol and `pactl`, and controlled by the user's `client.conf`.                                                                                                                                | Accepted                      |

## 3. Bugs

| Area                      | Finding                                                                                                                                                                                                                                                                     | Severity | Status                   |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------------------------ |
| Selection after updates   | Selection was a bare list position. With streams A, B, C and B selected, A ending moved the highlight onto C (reproduced live), so the next `m`, volume key or `x` (kill) hit another application. Each tab now re-finds its object by server index after every update.     | high     | Fixed (cb551cf)          |
| Channel balance           | A volume step set every channel to the average: `+5%` on a stream at 100% / 50% gave 80% / 80% (reproduced with `pactl`). The shown level is now the loudest channel and steps scale all channels proportionally, like pavucontrol (`pa_cvolume_max` / `pa_cvolume_scale`). | high     | Fixed (39a2567)          |
| Recording route popup     | The popup hid monitor sources and always highlighted the first entry. A recorder on "Monitor of …" saw the microphone highlighted, so Enter moved it there, and it could never be routed back to a monitor. Monitors are now listed and the current source is preselected.  | high     | Fixed (8cb3e8d)          |
| Volume above 153%         | `+` on a stream another program had set to 200% clamped it down to 153% (reproduced). Steps now never move against the key: the ceiling is the larger of 153% and the current level.                                                                                        | medium   | Fixed (67be072)          |
| Small terminals           | Cards were laid out below the header without clipping to the content area, so a short terminal (10×3; a split pane being resized) panicked with "index outside of buffer". Cards are clipped; a test renders every tab and popup at sizes from 1×1 up.                      | medium   | Fixed (f99922a)          |
| Server restart            | Restarting `pipewire-pulse` ended pavuc with "connection to the audio server was lost". It now keeps the TUI up, shows that it is waiting, retries every second, and carries on once the server is back (verified with the private server killed and restarted).            | medium   | Fixed (591c1de)          |
| Logs over the TUI         | Log lines went to stderr, the terminal the TUI draws on, and scrolled it off screen (seen at `--log-level debug` during a reconnect). stderr logging now pauses while the TUI is up; `--log-file` keeps them.                                                               | medium   | Fixed (bd9d676)          |
| Stale popup               | A popup kept the index of its stream/device; if that went away while it was open, Enter sent a command for a dead object. The popup now closes, with a status saying why.                                                                                                   | medium   | Fixed (933f926)          |
| Wrong connection error    | `pa_context_connect` returns `-1` and puts the reason in the context errno; the `-1` was read as an error code, so a missing server said "Access denied". Now "Connection refused" etc. The error was also printed three times; now once, with the hint.                    | medium   | Fixed (81d15fa, 5137e2a) |
| `tick_ms = 0`             | Busy loop: one core at 100% (measured 9.97 s CPU in 10 s). `tick_ms` is now checked to be 10–1000 ms on the CLI and in the file.                                                                                                                                            | medium   | Fixed (1085534)          |
| Ctrl+C                    | In raw mode Ctrl+C arrives as a key and was ignored, so the usual way out did nothing. It now quits from anywhere; other Ctrl/Alt chords no longer fall through to the plain key (Ctrl+X killed a stream).                                                                  | medium   | Fixed (e6c8373)          |
| Unknown config keys       | A misspelt key (`tickms`) was silently ignored. Now an error naming the key and the valid ones.                                                                                                                                                                             | low      | Fixed (863d210)          |
| Streams without volume    | A stream with `has_volume = false` showed the Configuration tab's "Press Enter to change", and the volume keys still sent it a volume change. Now "No volume control", and the keys say so.                                                                                 | low      | Fixed (8be17f3)          |
| `percent_to_raw` overflow | A large percentage wrapped when cast to `u32`, giving an arbitrary volume. Now saturates at `PA_VOLUME_MAX`.                                                                                                                                                                | low      | Fixed (96b8a7a)          |
| Unexplained exit          | Once during testing the first TUI session ended between two commands, shortly after a popup was closed with `Esc`; stderr wasn't captured then. Replaying the sequence with stderr captured did not reproduce it, and a popup's `Esc` only closes the popup.                | —        | Not reproduced           |

## 4. Performance

Measured with the release build, CPU time from `/proc/<pid>/stat` over 10 s,
default 100 ms tick.

| Area                       | Finding                                                                                                                                                                                                                                                          | Status          |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Re-fetching on events      | Any subscription event re-fetched all seven object lists. Only the kind the event names (sinks, sources, streams, cards, clients, server) is re-fetched now, so a stream of sink-volume events costs the server one sink-list request per tick instead of seven. | Fixed (23b79cd) |
| Idle redraws               | The snapshot was cloned and the screen redrawn every tick even when nothing changed. `iterate` now reports changes and pavuc redraws only after a change, key, resize or status update. Idle CPU: 20 ms → below the 10 ms clock resolution per 10 s.             | Fixed (23b79cd) |
| Event storm                | ~140 volume events/s from another client: 80 ms CPU per 10 s (0.8%) before and after; that is redraw work that is actually needed.                                                                                                                               | OK              |
| Latency of outside changes | Server events are read once per tick, so a change by another program shows up within `tick_ms` (100 ms by default). Waiting on the server socket and the terminal together would remove this, but needs a different main-loop integration.                       | Accepted        |
| Startup                    | Connect, full snapshot and exit (`examples/list`): 118 ms, of which 100 ms is the example's own wait for updates to settle.                                                                                                                                      | OK              |
| Memory                     | 7.4 MB RSS (peak equal) with ~10 objects. The snapshot is cloned only on change.                                                                                                                                                                                 | OK              |
| Connecting blocks          | `connect` blocks until the server answers. A server that accepts the socket but never replies would hang startup (and a reconnect attempt). Same as pavucontrol.                                                                                                 | Accepted (low)  |

## 5. UI usability and design

| Area                  | Finding                                                                                                                                                                                                                                        | Status          |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Silent failures       | Every command was sent with a no-op completion callback, so a refused port, profile, move or kill did nothing visible. Refused commands now show in the status line with the server's reason ("Could not change the profile: No such entity"). | Fixed (3f6a721) |
| Keys that don't apply | `x` on a device/card tab and `m` on Configuration silently did nothing (`d` already explained itself). They now say why; a kill names the stream it terminated.                                                                                | Fixed (dee28ba) |
| Disconnected state    | While reconnecting, the content area reads "Waiting for the audio server…" and the status line says so, instead of an empty "No applications are playing audio."                                                                               | Fixed (591c1de) |
| Layout                | Header tabs, one 4-row card per object, footer with per-tab key hints; selected card in cyan; over-100% volume in light red; muted/locked gauges grey. Long footer hints are cut off below ~80 columns.                                        | OK / accepted   |
| Colours               | 16-colour palette only, so it follows the terminal theme. The gauge track is hard-coded black; not checked on a light theme.                                                                                                                   | Accepted        |
| Emoji                 | The muted marker 🔇 is double-width; terminals without emoji fonts show a box. Text "muted" sits next to it, so the meaning survives.                                                                                                          | Accepted        |
| Scrolling             | The list scrolls to keep the selection visible (tested).                                                                                                                                                                                       | OK              |
| Help                  | Keys are listed in the footer and README; there is no in-app help screen.                                                                                                                                                                      | Accepted        |

## 6. Code quality, tests and project hygiene

| Area             | Finding                                                                                                                                                                                                                                                                        | Status                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------- |
| Testability      | `App::handle_key` took a concrete `&mut PulseClient`, so no key handling could be tested without a server. It now returns a `libpavuc::Command`, which `PulseClient::execute` runs.                                                                                            | Fixed (208bef9)             |
| Coverage         | 8.9% of lines before (UI and app state untested). Now 85 tests: app key paths, UI rendering on a `TestBackend` (size sweep, screens, escape filtering), volume math, settings. Lines covered: 59.7% without a server (what CI's codecov measures), 84.3% with the live tests.  | Fixed                       |
| Live tests in CI | The `#[ignore]` client tests never ran in CI. A new job starts PulseAudio with a null sink and runs them. Not yet run on GitHub: this machine has PipeWire, not the `pulseaudio` daemon the job uses.                                                                          | Fixed, unverified (f659ba4) |
| Untested code    | `app::run`/`event_loop` and `main` need a terminal and a server (covered by the live checks in §1, not by unit tests); `config::conventional_path` depends on the environment.                                                                                                 | Accepted                    |
| Clippy           | Clean with `-D warnings` in all feature sets. `pedantic`/`nursery` leave only style notes and casts proven bounded (rows on screen, channel counts ≤ 32, percentages ≤ 6.5M).                                                                                                  | OK                          |
| Gates            | fmt, typos, deny, clippy and tests (no-default, default, all features) and `cargo doc -D warnings` all pass at the end, and every audit commit was checked to build and pass clippy and tests on its own (§9).                                                                 | OK                          |
| Crate metadata   | `repository`/`homepage` pointed at `github.com/pavuc/pavuc`, which doesn't exist (also on the published 0.1.0). Now `github.com/Gigas002/pavuc`. Cargo warnings for a redundant homepage and unused workspace description are gone; `readme` and categories added.             | Fixed (ae2285c)             |
| Changelog        | 0.1.0 lists `libpavuc/examples/list.rs`, which was never added. It exists now. An Unreleased section lists the audit's changes.                                                                                                                                                | Fixed (972b431, 1b93e1d)    |
| README           | Keybindings only: no install, flags or config docs. Added.                                                                                                                                                                                                                     | Fixed (1b93e1d)             |
| Release          | libpavuc's API changed incompatibly (`execute(Command)` replaces the per-operation methods; `iterate` returns `bool`; volume shows the loudest channel). Both crates were still `0.1.0`, already on crates.io, so the deploy workflow would have failed; they are now `0.2.0`. | Fixed (version bump)        |

## 7. Features compared with pavucontrol

| pavucontrol feature                                          | pavuc                                                    |
| ------------------------------------------------------------ | -------------------------------------------------------- |
| Five tabs: Playback, Recording, Output, Input, Configuration | Yes                                                      |
| Volume, mute per stream and device                           | Yes                                                      |
| Channel balance kept while changing volume                   | Yes (since 39a2567)                                      |
| Per-channel sliders / unlock channels                        | No: balance is kept but can't be edited                  |
| Route streams to another device, incl. monitors              | Yes (monitors since 8cb3e8d)                             |
| Ports, profiles, set default device                          | Yes                                                      |
| Terminate stream                                             | Yes (`x`)                                                |
| Peak (VU) meters                                             | No: each meter needs its own record stream on the server |
| Show/hide filters (e.g. "All except monitors")               | No: monitors are always listed under Input Devices       |
| Base-volume marker, latency offset                           | No                                                       |
| Reconnect after server restart                               | Yes (since 591c1de)                                      |

## 8. Left open

- **Release**: both crates are now 0.2.0; set the date in CHANGELOG.md when tagging.
- **CI**: watch the first run of the new live-server job (§6).
- **Features**: per-channel volume, peak meters, view filters, base volume
  (§7). Each is a feature, not a fix.
- **Latency**: waiting on the server socket instead of polling per tick (§4).

## 9. Re-running the audit

```sh
# Gates (docs/ARCHITECTURE.md §8)
cargo fmt --all -- --check
typos
cargo deny check
for f in --no-default-features "" --all-features; do
  cargo clippy --workspace --all-targets $f -- -D warnings
  cargo test --workspace $f
done
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Live server (PulseAudio or pipewire-pulse running)
cargo test --workspace -- --ignored
cargo run -p libpavuc --example list

# Coverage (system LLVM tools on Arch: LLVM_COV / LLVM_PROFDATA)
cargo llvm-cov --workspace --all-features --summary-only -- --include-ignored
```

Each audit commit on its own (from a scratch worktree):

```sh
for c in $(git rev-list --reverse a882745..HEAD); do
  git checkout -q "$c" && cargo fmt --all -- --check &&
    cargo clippy --workspace --all-targets --all-features -- -D warnings &&
    cargo test --workspace --all-features || echo "FAILED: $c"
done
```

Manual checks with a null sink (`pactl load-module module-null-sink
sink_name=audit`) and silent streams (`pacat --device=audit < /dev/zero`):

- [ ] An unbalanced stream (`pactl set-sink-input-volume N 100% 50%`) keeps
      its balance on `→`.
- [ ] A stream at 200% isn't lowered by `→`.
- [ ] With three streams and the middle one selected, ending the first keeps
      the highlight on the same stream.
- [ ] A recorder on a monitor sees that monitor preselected on Enter.
- [ ] Resizing to 10×3 and 1×1 doesn't crash.
- [ ] Ctrl+C quits.
- [ ] Killing a private `pipewire-pulse` (`PULSE_SERVER=unix:…`) shows
      "Waiting…", and restarting it reconnects.
- [ ] `--log-level debug` prints nothing over the TUI; `--log-file` gets the
      lines and is created `0600`.
- [ ] A stream whose `media.name` holds escape sequences renders as inert
      text.
- [ ] Idle CPU over 10 s is ~0 ms; `--tick-ms 0` is rejected.
