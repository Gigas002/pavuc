# Architecture

`pavuc` is split into a reusable library (`libpavuc`) and a thin UI binary
(`pavuc`).

## Why `libpulse`?

pavucontrol is, at its core, a [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/)
client. On a PipeWire system the `pipewire-pulse` daemon implements the
PulseAudio server protocol, so a single client API — `libpulse` — works against
**both** servers without changes. Targeting `libpulse` (via the
[`libpulse-binding`](https://crates.io/crates/libpulse-binding) crate) is what
makes `pavuc` a true 1:1 analogue: it sees the exact same objects pavucontrol
does (sinks, sources, sink inputs, source outputs, cards, profiles, ports).

## `libpavuc`

```
PulseClient ── owns ──> Mainloop + Context   (libpulse, single-threaded)
     │
     ├── iterate()   pumps the main loop, dispatches events
     ├── snapshot()  returns an owned PulseState
     └── set_*/move_*/kill_* … commands
```

- A **non-threaded** `libpulse` main loop is pumped cooperatively from the UI's
  event loop via `PulseClient::iterate()`. Because everything runs on one
  thread, shared state lives behind plain `Rc<RefCell<_>>` rather than locks.
- A **subscription callback** flags the cached state dirty whenever the server
  reports a change. The next `iterate()` re-introspects every category and
  rebuilds an owned `PulseState`.
- In-flight `Operation`s (and their boxed callbacks) are kept alive in a
  type-erased `pending` list until the server reports them finished, then
  dropped — this avoids use-after-free of callback closures.
- `model.rs` converts the raw `libpulse` introspection structs into owned,
  UI-friendly types (`Device`, `Stream`, `Card`, `Port`, `Profile`).
- `volume.rs` holds the percentage math, matching pavucontrol's linear scale
  where `0x10000` is 100% and the UI cap is ~153%.

## `pavuc`

```
main.rs   terminal init/restore + event loop (ratatui + crossterm)
  │
  ├── app.rs   App state: current tab, per-tab selection, modal popups,
  │            and key handling that calls PulseClient commands
  └── ui.rs    pure rendering of the App snapshot into a Frame
```

The event loop each tick: `client.iterate()` → `app.update_state(snapshot)` →
`terminal.draw(render)` → poll for a key for `TICK` (100 ms). Rendering is a
pure function of `App`; all mutation happens in `app.rs` key handlers.
