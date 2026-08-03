use ratatui::layout::Rect;

use super::{centered_line, centered_rect};

#[test]
fn centered_line_keeps_width() {
    let area = Rect::new(0, 0, 40, 10);
    let line = centered_line(area);
    assert_eq!(line.width, 40);
    assert_eq!(line.height, 1);
}

#[test]
fn centered_rect_fits_inside_parent() {
    let area = Rect::new(0, 0, 100, 20);
    let rect = centered_rect(area, 60, 5);
    assert!(rect.width <= area.width);
    assert!(rect.height <= area.height);
}
