use cerebro::tui::redaction::RedactionPolicy;
use cerebro::TuiConfig;
use serde_json::json;

#[test]
fn redacts_sensitive_fields_even_when_allowed() {
    let policy = RedactionPolicy::from_config(&TuiConfig::default());
    let value = json!({
        "password": "secret",
        "token": "abc",
    });
    let redacted = policy
        .redact_with_allowlist(&value, &["password", "token"])
        .expect("redacted value");
    assert_eq!(
        redacted.get("password").and_then(|v| v.as_str()),
        Some("<redacted>")
    );
    assert_eq!(
        redacted.get("token").and_then(|v| v.as_str()),
        Some("<redacted>")
    );
}

#[test]
fn redacts_unknown_fields_by_default() {
    let policy = RedactionPolicy::from_config(&TuiConfig::default());
    let value = json!({
        "allowed": "ok",
        "unknown": "secret",
    });
    let redacted = policy
        .redact_with_allowlist(&value, &["allowed"])
        .expect("redacted value");
    assert_eq!(redacted.get("allowed").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(
        redacted.get("unknown").and_then(|v| v.as_str()),
        Some("<redacted>")
    );
}
