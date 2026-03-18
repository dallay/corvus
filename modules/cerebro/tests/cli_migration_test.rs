use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn cli_migrate_import_and_validate_workflow() {
    let temp_dir = TempDir::new().expect("temp dir");
    let target = temp_dir.path().join("cerebro.db");
    let source = PathBuf::from("tests/fixtures/legacy/legacy_export.json");

    let mut import_cmd = Command::cargo_bin("cerebro").expect("binary");
    let import_output = import_cmd
        .args([
            "migrate",
            "import",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
        ])
        .output()
        .expect("import output");
    assert!(import_output.status.success());
    let import_json: Value = serde_json::from_slice(&import_output.stdout).expect("json output");
    assert_eq!(import_json.get("status").and_then(|v| v.as_str()), Some("ok"));

    let mut validate_cmd = Command::cargo_bin("cerebro").expect("binary");
    let validate_output = validate_cmd
        .args([
            "migrate",
            "validate",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
        ])
        .output()
        .expect("validate output");
    assert!(validate_output.status.success());
    let validate_json: Value =
        serde_json::from_slice(&validate_output.stdout).expect("json output");
    assert_eq!(
        validate_json.get("status").and_then(|v| v.as_str()),
        Some("ok")
    );
}
