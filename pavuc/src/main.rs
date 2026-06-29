//! pavuc — a TUI 1:1 analogue of pavucontrol, built with ratatui.
//!
//! Connects to PulseAudio (or PipeWire via `pipewire-pulse`) and presents the
//! same five tabs pavucontrol does: Playback, Recording, Output Devices,
//! Input Devices and Configuration.

mod app;
mod ui;

use std::time::Duration;

use app::App;
use libpavuc::PulseClient;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

/// How long to wait for input before redrawing (drives the refresh cadence).
const TICK: Duration = Duration::from_millis(100);

fn main() {
    let mut client = match PulseClient::connect("pavuc") {
        Ok(client) => client,
        Err(error) => {
            eprintln!("pavuc: could not connect to the audio server: {error}");
            eprintln!("       is PulseAudio or pipewire-pulse running?");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut client);
    ratatui::restore();

    if let Err(error) = result {
        eprintln!("pavuc: {error}");
        std::process::exit(1);
    }
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    client: &mut PulseClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();

    while !app.should_quit {
        client.iterate()?;
        app.update_state(client.snapshot());

        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.status.clear();
            app.handle_key(key.code, client);
        }
    }

    Ok(())
}
