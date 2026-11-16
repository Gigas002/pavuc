use std::path::PathBuf;

use tracing::Level;

use crate::cli::CliOptions;
use crate::config::FileConfig;

use super::resolve;

#[test]
fn cli_overrides_file_and_defaults() {
    let cli = CliOptions {
        config: None,
        log_level: Some("debug".to_string()),
        log_file: Some(PathBuf::from("/cli.log")),
        client_name: Some("cli-name".to_string()),
        tick_ms: Some(50),
    };
    let file = FileConfig {
        client_name: Some("file-name".to_string()),
        log_level: Some("info".to_string()),
        log_file: Some(PathBuf::from("/file.log")),
        tick_ms: Some(200),
    };

    let settings = resolve(&cli, Some(file)).expect("resolve");
    assert_eq!(settings.client_name, "cli-name");
    assert_eq!(settings.log_level, Level::DEBUG);
    assert_eq!(settings.log_file, Some(PathBuf::from("/cli.log")));
    assert_eq!(settings.tick.as_millis(), 50);
}

#[test]
fn file_overrides_defaults() {
    let cli = CliOptions {
        config: None,
        log_level: None,
        log_file: None,
        client_name: None,
        tick_ms: None,
    };
    let file = FileConfig {
        client_name: Some("file-name".to_string()),
        log_level: Some("info".to_string()),
        log_file: Some(PathBuf::from("/file.log")),
        tick_ms: Some(250),
    };

    let settings = resolve(&cli, Some(file)).expect("resolve");
    assert_eq!(settings.client_name, "file-name");
    assert_eq!(settings.log_level, Level::INFO);
    assert_eq!(settings.log_file, Some(PathBuf::from("/file.log")));
    assert_eq!(settings.tick.as_millis(), 250);
}

#[test]
fn rejects_invalid_log_level() {
    let cli = CliOptions {
        config: None,
        log_level: Some("verbose".to_string()),
        log_file: None,
        client_name: None,
        tick_ms: None,
    };
    assert!(resolve(&cli, None).is_err());
}

/// Regression: `tick_ms = 0` made the event loop spin a core at 100%.
#[test]
fn rejects_tick_outside_range() {
    for ms in [0, 9, 1001, u64::MAX] {
        let cli = CliOptions {
            config: None,
            log_level: None,
            log_file: None,
            client_name: None,
            tick_ms: Some(ms),
        };
        assert!(resolve(&cli, None).is_err(), "{ms} ms accepted");
        let file = FileConfig {
            tick_ms: Some(ms),
            ..FileConfig::default()
        };
        assert!(
            resolve(&cli_none(), Some(file)).is_err(),
            "{ms} ms accepted"
        );
    }
}

#[test]
fn accepts_tick_range_bounds() {
    for ms in [10, 1000] {
        let file = FileConfig {
            tick_ms: Some(ms),
            ..FileConfig::default()
        };
        let settings = resolve(&cli_none(), Some(file)).expect("resolve");
        assert_eq!(settings.tick.as_millis(), u128::from(ms));
    }
}

fn cli_none() -> CliOptions {
    CliOptions {
        config: None,
        log_level: None,
        log_file: None,
        client_name: None,
        tick_ms: None,
    }
}

#[test]
fn defaults_log_to_stderr() {
    assert_eq!(resolve(&cli_none(), None).expect("resolve").log_file, None);
}
