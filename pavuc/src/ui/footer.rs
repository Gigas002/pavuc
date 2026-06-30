use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Tab};

use super::ACCENT;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let hint = if app.popup.is_some() {
        "↑↓ select   Enter confirm   Esc cancel"
    } else {
        match app.current_tab() {
            Tab::Playback | Tab::Recording => {
                "Tab switch   ↑↓ select   ←→ vol   m mute   Enter move   x kill   q quit"
            }
            Tab::Output | Tab::Input => {
                "Tab switch   ↑↓ select   ←→ vol   m mute   d default   Enter port   q quit"
            }
            Tab::Configuration => "Tab switch   ↑↓ select   Enter profile   q quit",
        }
    };

    let line = if app.status.is_empty() {
        Line::from(hint).style(Style::default().fg(Color::DarkGray))
    } else {
        Line::from(vec![
            Span::from(app.status.clone()).fg(ACCENT),
            Span::from("   "),
            Span::from(hint).fg(Color::DarkGray),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}
