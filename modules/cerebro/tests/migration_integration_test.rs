use cerebro::migration::{import_legacy_export, validate_legacy_export, MigrationOptions};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn end_to_end_import_validate_matches_fixtures() {
    let temp_dir = TempDir::new().expect("temp dir");
    let target = temp_dir.path().join("cerebro.db");
    let source = PathBuf::from("tests/fixtures/legacy/legacy_export.json");
    let options = MigrationOptions {
        namespace: None,
        database: None,
        dry_run: false,
    };

    let report = import_legacy_export(&source, &target, &options)
        .await
        .expect("import");
    assert_eq!(report.status.as_str(), "ok");
    assert_eq!(report.collections.get("prompt").unwrap().count, 1);

    let validate = validate_legacy_export(&source, &target, &options)
        .await
        .expect("validate");
    assert_eq!(validate.status.as_str(), "ok");
}
