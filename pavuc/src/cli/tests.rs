use std::path::PathBuf;

use clap::Parser;

use super::CliOptions;

#[test]
fn parses_defaults() {
    let cli = CliOptions::parse_from(["pavuc"]);
    assert!(cli.config.is_none());
    assert!(cli.log_level.is_none());
    assert!(cli.client_name.is_none());
    assert!(cli.tick_ms.is_none());
}

#[test]
fn parses_overrides() {
    let cli = CliOptions::parse_from([
        "pavuc",
        "--config",
        "/tmp/pavuc.toml",
        "--log-level",
        "debug",
        "--client-name",
        "my-pavuc",
        "--tick-ms",
        "250",
    ]);
    assert_eq!(cli.config, Some(PathBuf::from("/tmp/pavuc.toml")));
    assert_eq!(cli.log_level.as_deref(), Some("debug"));
    assert_eq!(cli.client_name.as_deref(), Some("my-pavuc"));
    assert_eq!(cli.tick_ms, Some(250));
}
