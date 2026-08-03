//! Rendering for the pavuc TUI using ratatui.

mod card;
mod content;
mod footer;
mod popup;
mod tabs;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Color;

use crate::app::App;

pub(crate) const ACCENT: Color = Color::Cyan;
pub(crate) const ITEM_HEIGHT: u16 = 4;

/// Draws the entire UI for the current frame.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    tabs::render(frame, app, chunks[0]);
    content::render(frame, app, chunks[1]);
    footer::render(frame, app, chunks[2]);

    if app.popup.is_some() {
        popup::render(frame, app);
    }
}

#[cfg(test)]
mod tests;
