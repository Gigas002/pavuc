//! Shared layout helpers with no settings or UI dependencies.

use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Centers a single-line area vertically within `area`.
#[must_use]
pub fn centered_line(area: Rect) -> Rect {
    let [line] = Layout::vertical([Constraint::Length(1)])
        .flex(Flex::Center)
        .areas(area);
    line
}

/// Centers a rectangle of `height` rows and `percent_x` width within `area`.
#[must_use]
pub fn centered_rect(area: Rect, percent_x: u16, height: u16) -> Rect {
    let [horizontal] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [vertical] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(horizontal);
    vertical
}

#[cfg(test)]
mod tests;
