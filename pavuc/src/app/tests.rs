use super::{App, Tab};

#[test]
fn tab_titles_match_pavucontrol() {
    assert_eq!(Tab::Playback.title(), "Playback");
    assert_eq!(Tab::Output.title(), "Output Devices");
    assert_eq!(Tab::ALL.len(), 5);
}

#[test]
fn empty_state_has_zero_items() {
    let app = App::default();
    assert_eq!(app.item_count(Tab::Playback), 0);
    assert_eq!(app.current_tab(), Tab::Playback);
}
