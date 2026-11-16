use super::Command;

#[test]
fn commands_compare_by_value() {
    let a = Command::SetSinkVolume {
        index: 1,
        volume: vec![10, 20],
    };
    assert_eq!(a.clone(), a);
    assert_ne!(
        a,
        Command::SetSinkVolume {
            index: 1,
            volume: vec![20, 10],
        }
    );
}

#[test]
fn every_command_describes_itself() {
    let commands = [
        Command::SetSinkMute {
            index: 0,
            mute: true,
        },
        Command::SetDefaultSource(String::new()),
        Command::MoveSinkInput { index: 0, sink: 1 },
        Command::SetCardProfile {
            card: 0,
            profile: String::new(),
        },
        Command::SetSourcePort {
            index: 0,
            port: String::new(),
        },
        Command::KillSourceOutput(0),
    ];
    for command in commands {
        assert!(!command.describe().is_empty());
        assert!(command.describe().is_ascii());
    }
}
