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

#[test]
fn scale_keeps_channel_balance() {
    // 100% / 50% raised so the loud channel is at 105%.
    let scaled = scale(&[NORMAL, NORMAL / 2], 105);
    assert_eq!(scaled, vec![percent_to_raw(105), percent_to_raw(105) / 2]);
    assert_eq!(raw_to_percent(scaled[1]), 52);
}

#[test]
fn scale_of_balanced_channels_is_uniform() {
    assert_eq!(scale(&[NORMAL, NORMAL], 40), vec![percent_to_raw(40); 2]);
}

#[test]
fn scale_from_silence_sets_every_channel() {
    assert_eq!(scale(&[0, 0], 10), vec![percent_to_raw(10); 2]);
    assert_eq!(scale(&[], 10), vec![percent_to_raw(10)]);
}

#[test]
fn scale_to_zero_mutes_every_channel() {
    assert_eq!(scale(&[NORMAL, NORMAL / 3], 0), vec![0, 0]);
}

#[test]
fn max_is_the_loudest_channel() {
    assert_eq!(max(&[1, 7, 3]), 7);
    assert_eq!(max(&[]), MUTED);
}

#[test]
fn step_clamps_to_ui_range() {
    assert_eq!(step_percent(3, -5), 0);
    assert_eq!(step_percent(150, 5), UI_MAX_PERCENT);
    assert_eq!(step_percent(50, 5), 55);
}

/// Regression: "+5%" on a stream at 200% used to clamp it down to 153%.
#[test]
fn step_never_moves_against_the_key() {
    assert_eq!(step_percent(200, 5), 200);
    assert_eq!(step_percent(200, -5), 195);
}
