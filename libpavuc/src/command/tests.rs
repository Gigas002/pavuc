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
