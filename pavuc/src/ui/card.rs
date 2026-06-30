use libpavuc::{Device, Stream, volume};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Gauge, Paragraph};

use crate::app::App;

use super::ACCENT;

pub fn render_stream(
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
    render(
        frame,
        area,
        &stream.name,
        right_status(stream.mute, false),
        &subtitle,
        gauge,
        selected,
    );
}

pub fn render_device(
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
    render(
        frame,
        area,
        &device.description,
        right_status(device.mute, default),
        &subtitle,
        Some((device.volume_percent(), device.mute, true)),
        selected,
    );
}

pub fn render_configuration(frame: &mut Frame, area: Rect, card: &libpavuc::Card, selected: bool) {
    let profile = card
        .active_profile
        .as_ref()
        .and_then(|name| card.profiles.iter().find(|p| &p.name == name))
        .map_or("(none)", |p| p.description.as_str());
    render(
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
fn render(
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
