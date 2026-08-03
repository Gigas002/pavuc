use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState};

use crate::app::{App, PopupKind};
use crate::utils;

use super::ACCENT;

pub fn render(frame: &mut Frame, app: &App) {
    let Some(popup) = &app.popup else {
        return;
    };

    let height = (popup.items.len() as u16 + 2).min(frame.area().height.saturating_sub(4));
    let area = utils::centered_rect(frame.area(), 60, height.max(3));
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = popup
        .items
        .iter()
        .map(|item| {
            let mut style = Style::default();
            if !item.available {
                style = style.fg(Color::DarkGray).add_modifier(Modifier::DIM);
            }
            ListItem::new(item.label.clone()).style(style)
        })
        .collect();

    let title = match popup.kind {
        PopupKind::MoveSinkInput(_) | PopupKind::MoveSourceOutput(_) => "Route stream",
        PopupKind::SinkPort(_) | PopupKind::SourcePort(_) => "Select port",
        PopupKind::CardProfile(_) => "Select profile",
    };

    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(ACCENT))
                .title(Span::from(format!(" {title} ")).bold())
                .title_bottom(Line::from(format!(" {} ", popup.title)).fg(Color::Gray)),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(ACCENT).bold())
        .highlight_symbol("➤ ");

    let mut list_state = ListState::default();
    list_state.select(Some(popup.selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}
