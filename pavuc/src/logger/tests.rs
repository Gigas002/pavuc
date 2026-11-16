use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use super::{atty, open_log_file, pause_stderr, stderr_paused};

#[test]
fn stderr_tty_check_is_boolean() {
    let _ = atty::is_stderr_tty();
}

#[test]
fn stderr_can_be_paused_and_resumed() {
    pause_stderr(true);
    assert!(stderr_paused());
    pause_stderr(false);
    assert!(!stderr_paused());
}

#[test]
fn log_file_is_private_and_appended() {
    let path = std::env::temp_dir().join(format!("pavuc-log-test-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&path);
    writeln!(open_log_file(&path).expect("open"), "one").expect("write");
    writeln!(open_log_file(&path).expect("reopen"), "two").expect("write");
    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode();
    let contents = std::fs::read_to_string(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(contents, "one\ntwo\n");
}

#[test]
fn log_file_in_missing_directory_is_an_error() {
    assert!(open_log_file(std::path::Path::new("/nonexistent/dir/pavuc.log")).is_err());
}
