use super::ITEM_HEIGHT;
use crate::app::{App, Tab};

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use crate::utils;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.item_count(app.current_tab());
    if count == 0 {
        let msg = empty_message(app.current_tab());
        let para = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(para, utils::centered_line(area));
        return;
    }

    let selected = app.selection();
    let max_visible = (area.height / ITEM_HEIGHT).max(1) as usize;
    let offset = selected.saturating_sub(max_visible.saturating_sub(1));

    for (row, index) in (offset..count).take(max_visible).enumerate() {
        let rect = Rect {
            x: area.x,
            y: area.y + row as u16 * ITEM_HEIGHT,
            width: area.width,
            height: ITEM_HEIGHT,
        };
        render_item(frame, app, rect, index, index == selected);
    }
}

fn render_item(frame: &mut Frame, app: &App, area: Rect, index: usize, selected: bool) {
    match app.current_tab() {
        Tab::Playback => {
            if let Some(s) = app.state.sink_inputs.get(index) {
                let target = app
                    .state
                    .sink(s.device)
                    .map_or("?", |d| d.description.as_str());
                super::card::render_stream(frame, area, app, s, target, "→", selected);
            }
        }
        Tab::Recording => {
            if let Some(s) = app.state.source_outputs.get(index) {
                let target = app
                    .state
                    .source(s.device)
                    .map_or("?", |d| d.description.as_str());
                super::card::render_stream(frame, area, app, s, target, "←", selected);
            }
        }
        Tab::Output => {
            if let Some(d) = app.state.sinks.get(index) {
                let default = app.state.is_default_sink(d);
                super::card::render_device(frame, area, d, default, selected);
            }
        }
        Tab::Input => {
            if let Some(d) = app.state.sources.get(index) {
                let default = app.state.is_default_source(d);
                super::card::render_device(frame, area, d, default, selected);
            }
        }
        Tab::Configuration => {
            if let Some(c) = app.state.cards.get(index) {
                super::card::render_configuration(frame, area, c, selected);
            }
        }
    }
}

fn empty_message(tab: Tab) -> &'static str {
    match tab {
        Tab::Playback => "No applications are playing audio.",
        Tab::Recording => "No applications are recording audio.",
        Tab::Output => "No output devices found.",
        Tab::Input => "No input devices found.",
        Tab::Configuration => "No sound cards found.",
    }
}
