use cerebro::migration::report::{CollectionReport, MigrationReport, MigrationStatus};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[test]
fn report_serializes_to_expected_json() {
    let mut collections = BTreeMap::new();
    collections.insert(
        "memory".to_string(),
        CollectionReport {
            count: 2,
            checksum: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
        },
    );
    collections.insert(
        "session".to_string(),
        CollectionReport {
            count: 1,
            checksum: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
        },
    );
    collections.insert(
        "prompt".to_string(),
        CollectionReport {
            count: 1,
            checksum: "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                .to_string(),
        },
    );

    let report = MigrationReport {
        source: "legacy_export.json".to_string(),
        target: "./cerebro.db".to_string(),
        collections,
        status: MigrationStatus::Ok,
    };

    let expected = read_fixture("tests/fixtures/reports/import_report_ok.json");
    assert_eq!(report.to_json_value(), expected);
}

#[test]
fn report_status_mismatch_matches_fixture() {
    let report = MigrationReport {
        source: "legacy_export.json".to_string(),
        target: "./cerebro.db".to_string(),
        collections: BTreeMap::new(),
        status: MigrationStatus::Mismatch,
    };

    let expected = read_fixture("tests/fixtures/reports/validate_report_mismatch.json");
    assert_eq!(report.status, MigrationStatus::Mismatch);
    assert_eq!(
        report.source,
        expected
            .get("source")
            .and_then(|value| value.as_str())
            .unwrap()
    );
}

fn read_fixture(path: &str) -> serde_json::Value {
    let data = fs::read_to_string(PathBuf::from(path)).expect("fixture should load");
    serde_json::from_str(&data).expect("fixture should parse")
}
