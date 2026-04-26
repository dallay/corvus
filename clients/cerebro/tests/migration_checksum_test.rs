use serde_json::json;

#[test]
fn canonical_checksum_is_order_insensitive_for_objects() {
    let value_a = json!({
        "b": 2,
        "a": 1,
        "nested": { "z": "last", "y": "first" }
    });
    let value_b = json!({
        "a": 1,
        "nested": { "y": "first", "z": "last" },
        "b": 2
    });

    let checksum_a = cerebro::migration::checksum::canonical_json_checksum(&value_a);
    let checksum_b = cerebro::migration::checksum::canonical_json_checksum(&value_b);

    assert_eq!(checksum_a, checksum_b);
    assert!(checksum_a.starts_with("sha256:"));
}

#[test]
fn canonical_checksum_is_order_sensitive_for_arrays() {
    let value_a = json!(["a", "b", "c"]);
    let value_b = json!(["c", "b", "a"]);

    let checksum_a = cerebro::migration::checksum::canonical_json_checksum(&value_a);
    let checksum_b = cerebro::migration::checksum::canonical_json_checksum(&value_b);

    assert_ne!(checksum_a, checksum_b);
}
