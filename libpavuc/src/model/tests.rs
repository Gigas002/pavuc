use super::*;

#[test]
fn device_state_labels_match_pavucontrol() {
    assert_eq!(DeviceState::Running.label(), "Running");
    assert_eq!(DeviceState::Idle.label(), "Idle");
    assert_eq!(DeviceState::Suspended.label(), "Suspended");
    assert_eq!(DeviceState::Invalid.label(), "Invalid");
}

#[test]
fn pulse_state_default_is_empty() {
    let state = PulseState::default();
    assert!(state.sinks.is_empty());
    assert!(state.sources.is_empty());
    assert!(state.default_sink.is_none());
}
