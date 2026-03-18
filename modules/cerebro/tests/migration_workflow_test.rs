use cerebro::migration::{import_legacy_export, validate_legacy_export, MigrationOptions};
use std::path::PathBuf;
use tempfile::TempDir;

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
