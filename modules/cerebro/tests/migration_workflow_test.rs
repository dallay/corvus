use cerebro::migration::{import_legacy_export, validate_legacy_export, MigrationOptions};
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;
use serde_json::Value;

#[tokio::test]
async fn import_and_validate_reports_match() {
    let temp_dir = TempDir::new().expect("temp dir");
    let target = temp_dir.path().join("cerebro.db");
    let source = PathBuf::from("tests/fixtures/legacy/legacy_export.json");

    let options = MigrationOptions {
        namespace: None,
        database: None,
        dry_run: false,
    };

    let import_report = import_legacy_export(&source, &target, &options)
        .await
        .expect("import should succeed");
    assert_eq!(import_report.status.as_str(), "ok");
    assert_eq!(import_report.collections.get("memory").unwrap().count, 2);

    let validate_report = validate_legacy_export(&source, &target, &options)
        .await
        .expect("validate should succeed");
    assert_eq!(validate_report.status.as_str(), "ok");
    assert_eq!(validate_report.collections.get("session").unwrap().count, 1);
}

#[tokio::test]
async fn validation_reports_mismatch_on_modified_source() {
    let temp_dir = TempDir::new().expect("temp dir");
    let target = temp_dir.path().join("cerebro.db");
    let source = PathBuf::from("tests/fixtures/legacy/legacy_export.json");

    let options = MigrationOptions {
        namespace: None,
        database: None,
        dry_run: false,
    };

    let _ = import_legacy_export(&source, &target, &options)
        .await
        .expect("import should succeed");

    let raw = fs::read_to_string(&source).expect("read source");
    let mut json: Value = serde_json::from_str(&raw).expect("json parse");
    if let Some(array) = json.get_mut("memory").and_then(Value::as_array_mut) {
        array.pop();
    }
    let bad_source = temp_dir.path().join("legacy_export_modified.json");
    fs::write(&bad_source, serde_json::to_vec(&json).expect("json encode"))
        .expect("write modified source");

    let report = validate_legacy_export(&bad_source, &target, &options)
        .await
        .expect("validate should succeed");
    assert_eq!(report.status.as_str(), "mismatch");
}
