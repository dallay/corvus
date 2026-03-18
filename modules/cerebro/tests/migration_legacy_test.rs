use cerebro::migration::legacy::{normalize_export, read_legacy_export};
use std::path::PathBuf;

#[test]
fn parses_legacy_export_fixture() {
    let path = PathBuf::from("tests/fixtures/legacy/legacy_export.json");
    let export = read_legacy_export(&path).expect("legacy export should parse");

    assert_eq!(export.memory.len(), 2);
    assert_eq!(export.session.len(), 1);
    assert_eq!(export.prompt.len(), 1);
}

#[test]
fn normalizes_memory_ids_and_sorting() {
    let path = PathBuf::from("tests/fixtures/legacy/legacy_export.json");
    let export = read_legacy_export(&path).expect("legacy export should parse");
    let normalized = normalize_export(export);

    assert_eq!(normalized.memory.len(), 2);
    assert_eq!(normalized.memory[0].memory_id, "01");
    assert_eq!(normalized.memory[1].memory_id, "02");
    assert_eq!(normalized.session[0].id, "session:01");
}
