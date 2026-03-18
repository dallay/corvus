use assert_cmd::Command;
use serde_json::Value;
use std::fs;
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
    assert_eq!(
        import_json.get("status").and_then(|v| v.as_str()),
        Some("ok")
    );

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

#[test]
fn cli_validate_exits_nonzero_on_mismatch() {
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

    let raw = fs::read_to_string(&source).expect("read source");
    let mut json: Value = serde_json::from_str(&raw).expect("json parse");
    if let Some(array) = json.get_mut("memory").and_then(Value::as_array_mut) {
        array.pop();
    }
    let bad_source = temp_dir.path().join("legacy_export_modified.json");
    fs::write(&bad_source, serde_json::to_vec(&json).expect("json encode"))
        .expect("write modified source");

    let mut validate_cmd = Command::cargo_bin("cerebro").expect("binary");
    let validate_output = validate_cmd
        .args([
            "migrate",
            "validate",
            "--source",
            bad_source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
        ])
        .output()
        .expect("validate output");
    assert_eq!(validate_output.status.code(), Some(2));
}
