//! Rendering for the pavuc TUI using ratatui.

use libpavuc::{Device, Stream, volume};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Gauge, List, ListItem, ListState, Paragraph};

use crate::app::{App, PopupKind, Tab};

const ACCENT: Color = Color::Cyan;
const ITEM_HEIGHT: u16 = 4;

/// Draws the entire UI for the current frame.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    if app.popup.is_some() {
        render_popup(frame, app);
    }
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| Line::from(format!(" {}:{} ", i + 1, tab.title())));
    let tabs = ratatui::widgets::Tabs::new(titles)
        .block(
            Block::bordered()
                .title(" pavuc — PulseAudio/PipeWire volume control ")
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .select(app.tab)
        .highlight_style(Style::default().fg(Color::Black).bg(ACCENT).bold())
        .divider("");
    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.item_count(app.current_tab());
    if count == 0 {
        let msg = empty_message(app.current_tab());
        let para = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(para, centered_line(area));
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
                render_stream_card(frame, area, app, s, target, "→", selected);
            }
        }
        Tab::Recording => {
            if let Some(s) = app.state.source_outputs.get(index) {
                let target = app
                    .state
                    .source(s.device)
                    .map_or("?", |d| d.description.as_str());
                render_stream_card(frame, area, app, s, target, "←", selected);
            }
        }
        Tab::Output => {
            if let Some(d) = app.state.sinks.get(index) {
                let default = app.state.is_default_sink(d);
                render_device_card(frame, area, d, default, selected);
            }
        }
        Tab::Input => {
            if let Some(d) = app.state.sources.get(index) {
                let default = app.state.is_default_source(d);
                render_device_card(frame, area, d, default, selected);
            }
        }
        Tab::Configuration => {
            if let Some(c) = app.state.cards.get(index) {
                render_card_card(frame, area, c, selected);
            }
        }
    }
}

fn render_stream_card(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    stream: &Stream,
    target: &str,
    arrow: &str,
    selected: bool,
) {
    let client = app
        .state
        .client_name(stream.client)
        .filter(|c| !c.is_empty() && *c != stream.app_name);
    let subtitle = match client {
        Some(c) => format!("{c}  {arrow} {target}"),
        None => format!("{arrow} {target}"),
    };
    let gauge = if stream.has_volume {
        Some((stream.volume_percent(), stream.mute, stream.volume_writable))
    } else {
        None
    };
    render_card(
        frame,
        area,
        &stream.name,
        right_status(stream.mute, false),
        &subtitle,
        gauge,
        selected,
    );
}

fn render_device_card(
    frame: &mut Frame,
    area: Rect,
    device: &Device,
    default: bool,
    selected: bool,
) {
    let port = device
        .active_port
        .as_ref()
        .and_then(|name| device.ports.iter().find(|p| &p.name == name))
        .map(|p| p.description.as_str());
    let subtitle = match port {
        Some(p) => format!("Port: {p}   [{}]", device.state.label()),
        None => format!("[{}]", device.state.label()),
    };
    render_card(
        frame,
        area,
        &device.description,
        right_status(device.mute, default),
        &subtitle,
        Some((device.volume_percent(), device.mute, true)),
        selected,
    );
}

fn render_card_card(frame: &mut Frame, area: Rect, card: &libpavuc::Card, selected: bool) {
    let profile = card
        .active_profile
        .as_ref()
        .and_then(|name| card.profiles.iter().find(|p| &p.name == name))
        .map_or("(none)", |p| p.description.as_str());
    render_card(
        frame,
        area,
        &card.description,
        Vec::new(),
        &format!("Profile: {profile}"),
        None,
        selected,
    );
}

/// Shared card renderer: bordered block with a subtitle line and an optional volume gauge.
fn render_card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    right: Vec<Span<'static>>,
    subtitle: &str,
    gauge: Option<(u32, bool, bool)>,
    selected: bool,
) {
    let border_style = if selected {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut block = Block::bordered()
        .border_style(border_style)
        .title(Span::from(format!(" {title} ")).bold());
    if !right.is_empty() {
        block = block.title(Line::from(right).right_aligned());
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let subtitle_widget =
        Paragraph::new(subtitle.to_string()).style(Style::default().fg(Color::Gray));
    frame.render_widget(subtitle_widget, rows[0]);

    if let Some((percent, muted, writable)) = gauge {
        let ratio = (f64::from(percent) / f64::from(volume::UI_MAX_PERCENT)).clamp(0.0, 1.0);
        let (color, label) = if muted {
            (Color::DarkGray, format!("{percent}%  muted"))
        } else if !writable {
            (Color::DarkGray, format!("{percent}%  (locked)"))
        } else if percent > 100 {
            (Color::LightRed, format!("{percent}%"))
        } else {
            (Color::Green, format!("{percent}%"))
        };
        let g = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .ratio(ratio)
            .label(Span::from(label).fg(Color::White).bold());
        frame.render_widget(g, rows[1]);
    } else if let Some(area) = rows.get(1) {
        let hint = Paragraph::new("Press Enter to change").style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        );
        frame.render_widget(hint, *area);
    }
}

fn right_status(muted: bool, default: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if default {
        spans.push(Span::from("★ default ").fg(Color::Yellow));
    }
    if muted {
        spans.push(Span::from("🔇 muted ").fg(Color::Red));
    }
    spans
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_popup(frame: &mut Frame, app: &App) {
    let Some(popup) = &app.popup else { return };

    let height = (popup.items.len() as u16 + 2).min(frame.area().height.saturating_sub(4));
    let area = centered_rect(frame.area(), 60, height.max(3));
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

fn empty_message(tab: Tab) -> &'static str {
    match tab {
        Tab::Playback => "No applications are playing audio.",
        Tab::Recording => "No applications are recording audio.",
        Tab::Output => "No output devices found.",
        Tab::Input => "No input devices found.",
        Tab::Configuration => "No sound cards found.",
    }
}

fn centered_line(area: Rect) -> Rect {
    let [line] = Layout::vertical([Constraint::Length(1)])
        .flex(Flex::Center)
        .areas(area);
    line
}

fn centered_rect(area: Rect, percent_x: u16, height: u16) -> Rect {
    let [horizontal] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [vertical] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(horizontal);
    vertical
}
