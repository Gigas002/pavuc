use tracing::Level;

use crate::cli::CliOptions;
use crate::config::FileConfig;

use super::resolve;

#[test]
fn cli_overrides_file_and_defaults() {
    let cli = CliOptions {
        config: None,
        log_level: Some("debug".to_string()),
        client_name: Some("cli-name".to_string()),
        tick_ms: Some(50),
    };
    let file = FileConfig {
        client_name: Some("file-name".to_string()),
        log_level: Some("info".to_string()),
        tick_ms: Some(200),
    };

    let settings = resolve(&cli, Some(file)).expect("resolve");
    assert_eq!(settings.client_name, "cli-name");
    assert_eq!(settings.log_level, Level::DEBUG);
    assert_eq!(settings.tick.as_millis(), 50);
}

#[test]
fn file_overrides_defaults() {
    let cli = CliOptions {
        config: None,
        log_level: None,
        client_name: None,
        tick_ms: None,
    };
    let file = FileConfig {
        client_name: Some("file-name".to_string()),
        log_level: Some("info".to_string()),
        tick_ms: Some(250),
    };

    let settings = resolve(&cli, Some(file)).expect("resolve");
    assert_eq!(settings.client_name, "file-name");
    assert_eq!(settings.log_level, Level::INFO);
    assert_eq!(settings.tick.as_millis(), 250);
}

#[test]
fn rejects_invalid_log_level() {
    let cli = CliOptions {
        config: None,
        log_level: Some("verbose".to_string()),
        client_name: None,
        tick_ms: None,
    };
    assert!(resolve(&cli, None).is_err());
}
