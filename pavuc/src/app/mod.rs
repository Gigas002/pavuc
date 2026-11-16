//! Application state machine and main event loop.

use std::time::{Duration, Instant};

use libpavuc::PulseClient;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::settings::Settings;
use crate::ui;

mod state;

pub use state::{App, PopupKind, Tab};

/// How long to wait between reconnection attempts after the server went away.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

/// Connects to the audio server and runs the TUI until the user quits.
///
/// If the server goes away while running (e.g. `pipewire-pulse` restarts),
/// the TUI stays up and reconnects once the server is back.
///
/// # Errors
///
/// Returns an error if the first connection fails or the terminal loop
/// encounters an unrecoverable error.
pub fn run(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect(&settings)?;

    crate::logger::pause_stderr(true);
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, client, &settings);
    ratatui::restore();
    crate::logger::pause_stderr(false);
    result
}

fn connect(settings: &Settings) -> Result<PulseClient, String> {
    PulseClient::connect(&settings.client_name)
        .map_err(|error| format!("{error} (is PulseAudio or pipewire-pulse running?)"))
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    client: PulseClient,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();
    let mut client = Some(client);
    let mut next_attempt = Instant::now();

    while !app.should_quit {
        match &mut client {
            Some(connected) => match connected.iterate() {
                Ok(()) => {
                    app.update_state(connected.snapshot());
                    app.report_failures(connected.take_failures());
                }
                Err(error) => {
                    tracing::warn!("{error}; reconnecting");
                    client = None;
                    app.set_disconnected();
                    next_attempt = Instant::now() + RECONNECT_INTERVAL;
                }
            },
            None if Instant::now() >= next_attempt => match connect(settings) {
                Ok(connected) => {
                    tracing::info!("reconnected to the audio server");
                    client = Some(connected);
                    app.set_connected();
                }
                Err(error) => {
                    tracing::debug!("reconnect failed: {error}");
                    next_attempt = Instant::now() + RECONNECT_INTERVAL;
                }
            },
            None => {}
        }

        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(settings.tick)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.status.clear();
            if let Some(command) = app.handle_key_event(key)
                && let Some(connected) = &mut client
            {
                connected.execute(command);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
