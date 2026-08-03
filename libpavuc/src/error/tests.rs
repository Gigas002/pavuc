use super::*;

#[test]
fn display_messages_are_stable() {
    assert_eq!(
        Error::Mainloop.to_string(),
        "failed to create the PulseAudio main loop"
    );
    assert_eq!(
        Error::Disconnected.to_string(),
        "connection to the audio server was lost"
    );
    assert!(
        Error::Connect("pipewire down".into())
            .to_string()
            .contains("pipewire down")
    );
}
