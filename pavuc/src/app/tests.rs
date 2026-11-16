use libpavuc::{Card, Command, Device, DeviceState, Port, Profile, PulseState, Stream, volume};
use ratatui::crossterm::event::KeyCode;

use super::{App, PopupKind, Tab};

pub(crate) fn device(index: u32, name: &str, volume: Vec<u32>) -> Device {
    Device {
        index,
        name: name.to_string(),
        description: format!("{name} description"),
        channels: volume.len() as u8,
        volume,
        mute: false,
        base_volume: volume::NORMAL,
        card: None,
        ports: Vec::new(),
        active_port: None,
        monitor: false,
        state: DeviceState::Idle,
    }
}

pub(crate) fn stream(index: u32, name: &str, device: u32, volume: Vec<u32>) -> Stream {
    Stream {
        index,
        name: name.to_string(),
        app_name: name.to_string(),
        media_name: String::new(),
        device,
        client: None,
        channels: volume.len() as u8,
        volume,
        mute: false,
        has_volume: true,
        volume_writable: true,
        corked: false,
    }
}

pub(crate) fn port(name: &str, available: bool) -> Port {
    Port {
        name: name.to_string(),
        description: format!("{name} port"),
        available,
    }
}

pub(crate) fn sample_state() -> PulseState {
    let half = volume::NORMAL / 2;
    let mut speakers = device(1, "speakers", vec![half, half]);
    speakers.ports = vec![port("analog", true), port("hdmi", false)];
    speakers.active_port = Some("hdmi".to_string());
    let headset = device(2, "headset", vec![volume::NORMAL]);
    let mic = device(10, "mic", vec![half, half]);
    let mut monitor = device(11, "speakers.monitor", vec![volume::NORMAL]);
    monitor.monitor = true;
    PulseState {
        sinks: vec![speakers, headset],
        sources: vec![mic, monitor],
        sink_inputs: vec![
            stream(100, "music", 1, vec![half, half]),
            stream(101, "video", 2, vec![volume::NORMAL]),
        ],
        source_outputs: vec![stream(200, "recorder", 11, vec![volume::NORMAL])],
        cards: vec![Card {
            index: 5,
            name: "card".to_string(),
            description: "Sound card".to_string(),
            profiles: vec![
                Profile {
                    name: "stereo".to_string(),
                    description: "Analog Stereo".to_string(),
                    available: true,
                },
                Profile {
                    name: "off".to_string(),
                    description: "Off".to_string(),
                    available: true,
                },
            ],
            active_profile: Some("off".to_string()),
        }],
        default_sink: Some("speakers".to_string()),
        ..PulseState::default()
    }
}

pub(crate) fn sample_app() -> App {
    let mut app = App::default();
    app.update_state(sample_state());
    app
}

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

#[test]
fn empty_tabs_send_no_commands() {
    let mut app = App::default();
    for tab in ['1', '2', '3', '4', '5'] {
        app.handle_key(KeyCode::Char(tab));
        for key in [
            KeyCode::Right,
            KeyCode::Left,
            KeyCode::Char('m'),
            KeyCode::Char('d'),
            KeyCode::Char('x'),
            KeyCode::Enter,
            KeyCode::Down,
        ] {
            assert_eq!(app.handle_key(key), None);
        }
        assert!(app.popup.is_none());
    }
}

#[test]
fn number_keys_and_tab_switch_tabs() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('3'));
    assert_eq!(app.current_tab(), Tab::Output);
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.current_tab(), Tab::Input);
    app.handle_key(KeyCode::Char('5'));
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.current_tab(), Tab::Playback);
    app.handle_key(KeyCode::BackTab);
    assert_eq!(app.current_tab(), Tab::Configuration);
}

#[test]
fn selection_wraps_around() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Up);
    assert_eq!(app.selection(), 1);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.selection(), 0);
}

#[test]
fn selection_is_kept_per_tab() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Down);
    app.handle_key(KeyCode::Char('3'));
    assert_eq!(app.selection(), 0);
    app.handle_key(KeyCode::Char('1'));
    assert_eq!(app.selection(), 1);
}

#[test]
fn update_state_clamps_selection_to_shorter_lists() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Down);
    let mut state = sample_state();
    state.sink_inputs.truncate(1);
    app.update_state(state);
    assert_eq!(app.selection(), 0);
    app.update_state(PulseState::default());
    assert_eq!(app.selection(), 0);
}

#[test]
fn volume_keys_step_coarse_and_fine() {
    let mut app = sample_app();
    let Some(Command::SetSinkInputVolume { index, volume }) = app.handle_key(KeyCode::Right) else {
        panic!("expected a volume command");
    };
    assert_eq!(index, 100);
    assert_eq!(volume, vec![volume::percent_to_raw(55); 2]);

    let Some(Command::SetSinkInputVolume { volume, .. }) = app.handle_key(KeyCode::Char(','))
    else {
        panic!("expected a volume command");
    };
    assert_eq!(volume, vec![volume::percent_to_raw(49); 2]);
}

#[test]
fn volume_keys_target_the_current_tab() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('2'));
    assert!(matches!(
        app.handle_key(KeyCode::Left),
        Some(Command::SetSourceOutputVolume { index: 200, .. })
    ));
    app.handle_key(KeyCode::Char('3'));
    assert!(matches!(
        app.handle_key(KeyCode::Char('l')),
        Some(Command::SetSinkVolume { index: 1, .. })
    ));
    app.handle_key(KeyCode::Char('4'));
    assert!(matches!(
        app.handle_key(KeyCode::Char('h')),
        Some(Command::SetSourceVolume { index: 10, .. })
    ));
    app.handle_key(KeyCode::Char('5'));
    assert_eq!(app.handle_key(KeyCode::Right), None);
}

#[test]
fn locked_stream_volume_is_refused_with_a_status() {
    let mut state = sample_state();
    state.sink_inputs[0].volume_writable = false;
    let mut app = App::default();
    app.update_state(state);
    assert_eq!(app.handle_key(KeyCode::Right), None);
    assert!(!app.status.is_empty());
}

#[test]
fn mute_toggles_the_selected_object() {
    let mut app = sample_app();
    assert_eq!(
        app.handle_key(KeyCode::Char('m')),
        Some(Command::SetSinkInputMute {
            index: 100,
            mute: true
        })
    );
    app.handle_key(KeyCode::Char('4'));
    assert_eq!(
        app.handle_key(KeyCode::Char('m')),
        Some(Command::SetSourceMute {
            index: 10,
            mute: true
        })
    );
}

#[test]
fn set_default_only_on_device_tabs() {
    let mut app = sample_app();
    assert_eq!(app.handle_key(KeyCode::Char('d')), None);
    assert!(!app.status.is_empty());
    app.handle_key(KeyCode::Char('3'));
    app.handle_key(KeyCode::Down);
    assert_eq!(
        app.handle_key(KeyCode::Char('d')),
        Some(Command::SetDefaultSink("headset".to_string()))
    );
}

#[test]
fn kill_only_on_stream_tabs() {
    let mut app = sample_app();
    assert_eq!(
        app.handle_key(KeyCode::Char('x')),
        Some(Command::KillSinkInput(100))
    );
    app.handle_key(KeyCode::Char('2'));
    assert_eq!(
        app.handle_key(KeyCode::Char('x')),
        Some(Command::KillSourceOutput(200))
    );
    app.handle_key(KeyCode::Char('3'));
    assert_eq!(app.handle_key(KeyCode::Char('x')), None);
}

#[test]
fn route_popup_preselects_current_sink_and_moves_stream() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Down);
    app.handle_key(KeyCode::Enter);
    let popup = app.popup.as_ref().expect("popup");
    assert!(matches!(popup.kind, PopupKind::MoveSinkInput(101)));
    assert_eq!(popup.selected, 1);
    app.handle_key(KeyCode::Up);
    assert_eq!(
        app.handle_key(KeyCode::Enter),
        Some(Command::MoveSinkInput {
            index: 101,
            sink: 1
        })
    );
    assert!(app.popup.is_none());
}

#[test]
fn port_popup_preselects_active_port() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('3'));
    app.handle_key(KeyCode::Enter);
    let popup = app.popup.as_ref().expect("popup");
    assert_eq!(popup.selected, 1);
    assert!(!popup.items[1].available);
    app.handle_key(KeyCode::Down);
    assert_eq!(
        app.handle_key(KeyCode::Enter),
        Some(Command::SetSinkPort {
            index: 1,
            port: "analog".to_string()
        })
    );
}

#[test]
fn profile_popup_sets_card_profile() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('5'));
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.popup.as_ref().expect("popup").selected, 1);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(
        app.handle_key(KeyCode::Enter),
        Some(Command::SetCardProfile {
            card: 5,
            profile: "stereo".to_string()
        })
    );
}

#[test]
fn popup_without_choices_is_not_opened() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('4'));
    app.handle_key(KeyCode::Enter);
    assert!(app.popup.is_none());
    assert!(!app.status.is_empty());
}

#[test]
fn escape_closes_popup_before_quitting() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Enter);
    assert!(app.popup.is_some());
    assert_eq!(app.handle_key(KeyCode::Esc), None);
    assert!(app.popup.is_none());
    assert!(!app.should_quit);
    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn popup_swallows_other_keys() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.handle_key(KeyCode::Char('x')), None);
    assert_eq!(app.handle_key(KeyCode::Char('3')), None);
    assert_eq!(app.current_tab(), Tab::Playback);
    assert!(app.popup.is_some());
}

/// Regression: a step used to set every channel to the average, so an
/// unbalanced 100% / 50% stream became 80% / 80% on "+5%".
#[test]
fn volume_step_keeps_channel_balance() {
    let mut state = sample_state();
    state.sink_inputs[0].volume = vec![volume::NORMAL, volume::NORMAL / 2];
    let mut app = App::default();
    app.update_state(state);
    let Some(Command::SetSinkInputVolume { volume, .. }) = app.handle_key(KeyCode::Right) else {
        panic!("expected a volume command");
    };
    assert_eq!(
        volume,
        vec![volume::percent_to_raw(105), volume::percent_to_raw(105) / 2]
    );
}

/// Regression: with streams A, B, C and B selected, A going away moved the
/// highlight onto C, so the next `m` or `x` hit the wrong application.
#[test]
fn selection_follows_its_object_when_lists_shift() {
    let mut state = sample_state();
    state
        .sink_inputs
        .push(stream(102, "game", 1, vec![volume::NORMAL]));
    let mut app = App::default();
    app.update_state(state.clone());
    app.handle_key(KeyCode::Down);
    assert_eq!(
        app.handle_key(KeyCode::Char('x')),
        Some(Command::KillSinkInput(101))
    );

    state.sink_inputs.remove(0);
    app.update_state(state);
    assert_eq!(app.selection(), 0);
    assert_eq!(
        app.handle_key(KeyCode::Char('x')),
        Some(Command::KillSinkInput(101))
    );
}

#[test]
fn selection_moves_to_the_next_object_when_its_own_goes_away() {
    let mut state = sample_state();
    state
        .sink_inputs
        .push(stream(102, "game", 1, vec![volume::NORMAL]));
    let mut app = App::default();
    app.update_state(state.clone());
    app.handle_key(KeyCode::Down);
    state.sink_inputs.remove(1);
    app.update_state(state);
    assert_eq!(
        app.handle_key(KeyCode::Char('x')),
        Some(Command::KillSinkInput(102))
    );
}

/// Regression: the Recording route popup hid monitor sources and always
/// highlighted the first entry, so Enter on a stream recording a monitor
/// moved it to the microphone.
#[test]
fn recording_popup_offers_monitors_and_preselects_current_source() {
    let mut app = sample_app();
    app.handle_key(KeyCode::Char('2'));
    app.handle_key(KeyCode::Enter);
    let popup = app.popup.as_ref().expect("popup");
    assert_eq!(popup.items.len(), 2);
    assert_eq!(popup.items[popup.selected].value, "11");
    assert_eq!(
        app.handle_key(KeyCode::Enter),
        Some(Command::MoveSourceOutput {
            index: 200,
            source: 11
        })
    );
}
