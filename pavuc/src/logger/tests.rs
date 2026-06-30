use super::atty;

#[test]
fn stderr_tty_check_is_boolean() {
    let _ = atty::is_stderr_tty();
}
