use std::path::PathBuf;

use super::*;

/// A config file in the temp dir, unique per test and process, removed on drop.
struct TempConfig(PathBuf);

impl TempConfig {
    fn new(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("pavuc-config-{}-{name}.toml", std::process::id()));
        std::fs::write(&path, contents).expect("write");
        Self(path)
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn deserializes_example_fields() {
    let file = TempConfig::new(
        "fields",
        r#"
client_name = "custom"
log_level = "info"
tick_ms = 200
"#,
    );
    assert_eq!(
        load(&file.0).expect("load"),
        FileConfig {
            client_name: Some("custom".to_string()),
            log_level: Some("info".to_string()),
            log_file: None,
            tick_ms: Some(200),
        }
    );
}

#[test]
fn shipped_example_is_valid() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pavuc.toml");
    let config = load(&path).expect("examples/pavuc.toml must load");
    assert_eq!(config.client_name.as_deref(), Some("pavuc"));
}

#[test]
fn rejects_invalid_toml() {
    let file = TempConfig::new("bad", "not = [valid");
    assert!(load(&file.0).is_err());
}

/// Regression: a misspelt key (`tickms`) was silently ignored.
#[test]
fn rejects_unknown_keys() {
    let file = TempConfig::new("unknown", "tickms = 50\n");
    let error = load(&file.0).expect_err("unknown key accepted");
    assert!(error.contains("tickms"), "{error}");
}

#[test]
fn missing_file_is_an_error() {
    let error = load(Path::new("/nonexistent/pavuc.toml")).expect_err("loaded");
    assert!(error.contains("could not read"), "{error}");
}
