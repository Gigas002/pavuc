use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::KeyCode;

use super::{ITEM_HEIGHT, render};
use crate::app::App;
use crate::app::tests::sample_app;

/// Renders `app` into a `width` x `height` test terminal and returns the
/// screen as text, one line per row.
fn draw(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal.draw(|frame| render(frame, app)).expect("draw");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn item_height_fits_stream_card_layout() {
    assert_eq!(ITEM_HEIGHT, 4);
}

/// Regression: a terminal shorter than the header + one card (e.g. 10x3)
/// panicked with "index outside of buffer".
#[test]
fn renders_every_tab_and_popup_at_tiny_sizes() {
    for tab in ['1', '2', '3', '4', '5'] {
        let mut app = sample_app();
        app.handle_key(KeyCode::Char(tab));
        for popup in [false, true] {
            if popup {
                app.handle_key(KeyCode::Enter);
            }
            for width in [1, 2, 3, 5, 8, 13, 21, 40] {
                for height in 1..=12 {
                    draw(&app, width, height);
                }
            }
        }
    }
}

#[test]
fn shows_tabs_streams_and_footer() {
    let app = sample_app();
    let screen = draw(&app, 100, 20);
    assert!(screen.contains("1:Playback"));
    assert!(screen.contains("5:Configuration"));
    assert!(screen.contains("music"));
    assert!(screen.contains("→ speakers description"));
    assert!(screen.contains("50%"));
    assert!(screen.contains("q quit"));
}

#[test]
fn device_tab_marks_default_and_port() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('3'));
    let screen = draw(&app, 100, 20);
    assert!(screen.contains("★ default"));
    assert!(screen.contains("Port: hdmi port"));
    assert!(screen.contains("[Idle]"));
}

#[test]
fn empty_tab_explains_itself() {
    let screen = draw(&App::default(), 80, 12);
    assert!(screen.contains("No applications are playing audio."));
}

#[test]
fn popup_lists_choices() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('5'));
    app.handle_key(KeyCode::Enter);
    let screen = draw(&app, 100, 20);
    assert!(screen.contains("Select profile"));
    assert!(screen.contains("Analog Stereo"));
    assert!(screen.contains("➤ Off"));
}

#[test]
fn status_message_is_shown_in_footer() {
    let mut app = sample_app();
    app.status = "hello status".to_string();
    assert!(draw(&app, 100, 20).contains("hello status"));
}

#[test]
fn selected_item_stays_visible_when_scrolling() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Down);
    // Room for the header, one card and the footer only.
    let screen = draw(&app, 80, 8);
    assert!(screen.contains("video"));
    assert!(!screen.contains("music"));
}

/// Stream and device names come from other programs (a browser sets the
/// media name from the page title). Terminal control characters in them
/// must never reach the terminal.
#[test]
fn control_characters_in_names_are_not_rendered() {
    let mut app = sample_app();
    let mut state = app.state.clone();
    let evil = "evil\u{1b}]0;PWNED\u{7}\u{1b}[2J\r\nname\u{9b}31m";
    state.sink_inputs[0].name = evil.to_string();
    state.sinks[0].description = evil.to_string();
    app.update_state(state);
    for tab in ['1', '3'] {
        app.handle_key(KeyCode::Char(tab));
        app.handle_key(KeyCode::Enter);
        let screen = draw(&app, 120, 20);
        assert!(screen.contains("evil"));
        assert!(
            !screen.chars().any(|c| c.is_control() && c != '\n'),
            "control character rendered: {screen:?}"
        );
        app.handle_key(KeyCode::Esc);
    }
}

#[test]
fn disconnected_screen_says_it_is_waiting() {
    let mut app = sample_app();
    app.set_disconnected();
    let screen = draw(&app, 100, 12);
    assert!(screen.contains("Waiting for the audio server"));
    assert!(screen.contains("reconnecting"));
}
