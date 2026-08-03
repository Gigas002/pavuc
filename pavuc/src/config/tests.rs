use std::io::Write;

use super::*;

#[test]
fn deserializes_example_fields() {
    let path = std::env::temp_dir().join("pavuc-config-test.toml");
    let mut file = std::fs::File::create(&path).expect("create");
    writeln!(
        file,
        r#"
client_name = "custom"
log_level = "info"
tick_ms = 200
"#
    )
    .expect("write");

    let config = load(&path).expect("load");
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        config,
        FileConfig {
            client_name: Some("custom".to_string()),
            log_level: Some("info".to_string()),
            tick_ms: Some(200),
        }
    );
}

#[test]
fn rejects_invalid_toml() {
    let path = std::env::temp_dir().join("pavuc-config-bad.toml");
    std::fs::write(&path, "not = [valid").expect("write");
    let result = load(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}
