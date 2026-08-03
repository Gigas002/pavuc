//! Application state machine and main event loop.

use libpavuc::PulseClient;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::settings::Settings;
use crate::ui;

mod state;

pub use state::{App, PopupKind, Tab};

/// Connects to the audio server and runs the TUI until the user quits.
///
/// # Errors
///
/// Returns an error if the connection fails or the terminal loop encounters
/// an unrecoverable error.
pub fn run(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = match PulseClient::connect(&settings.client_name) {
        Ok(client) => client,
        Err(error) => {
            tracing::error!("could not connect to the audio server: {error}");
            tracing::error!("is PulseAudio or pipewire-pulse running?");
            return Err(error.into());
        }
    };

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut client, &settings);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    client: &mut PulseClient,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();

    while !app.should_quit {
        client.iterate()?;
        app.update_state(client.snapshot());

        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(settings.tick)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.status.clear();
            app.handle_key(key.code, client);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
