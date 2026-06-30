use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Block;

use crate::app::{App, Tab};

use super::ACCENT;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
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
