use super::*;

#[test]
fn normal_is_one_hundred_percent() {
    assert_eq!(raw_to_percent(NORMAL), 100);
    assert_eq!(percent_to_raw(100), NORMAL);
}

#[test]
fn muted_is_zero() {
    assert_eq!(raw_to_percent(MUTED), 0);
    assert_eq!(percent_to_raw(0), MUTED);
}

#[test]
fn clamp_respects_ui_max() {
    assert_eq!(clamp_percent(-10), 0);
    assert_eq!(clamp_percent(200), UI_MAX_PERCENT);
    assert_eq!(clamp_percent(75), 75);
}
