use std::process::Command;

#[test]
fn cli_preview_outputs_loop_lifecycle_with_approval_interruption() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_corvus"))
        .args(["agent", "--message", "needs-approval"])
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("CORVUS_UNIFIED_LOOP_PREVIEW", "1")
        .env("CORVUS_UNIFIED_LOOP_ONLY", "1")
        .env("RUST_LOG", "off")
        .output()
        .expect("run corvus binary");

    assert!(
        output.status.success(),
        "CLI should exit successfully in unified-loop-only mode"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("loop_event=Start"));
    assert!(stdout.contains("loop_event=ApprovalRequired(\"tool-1\")"));
    assert!(stdout.contains("loop_event=Error(\"approval denied\")"));
}

#[test]
fn cli_preview_propagates_session_and_timeout_abort_semantics() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_corvus"))
        .args(["agent", "--message", "timeout"])
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("CORVUS_UNIFIED_LOOP_PREVIEW", "1")
        .env("CORVUS_UNIFIED_LOOP_ONLY", "1")
        .env("CORVUS_SESSION_ID", "session-cli-e2e")
        .env("RUST_LOG", "off")
        .output()
        .expect("run corvus binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("loop_session=session-cli-e2e"));
    assert!(stdout.contains("retrying after recoverable error"));
}

#[test]
fn cli_non_preview_timeout_abort_is_session_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_corvus"))
        .args(["agent", "--message", "timeout"])
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("CORVUS_SESSION_ID", "session-cli-prod")
        .env("RUST_LOG", "off")
        .output()
        .expect("run corvus binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[session:session-cli-prod] request aborted due to timeout semantics"));
}

#[test]
fn cli_non_preview_approval_unblocks_with_override() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_corvus"))
        .args(["agent", "--message", "needs-approval"])
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("CORVUS_UNIFIED_APPROVE", "1")
        .env("CORVUS_UNIFIED_CANONICAL_ONLY", "1")
        .env("CORVUS_SESSION_ID", "session-cli-prod")
        .env("RUST_LOG", "off")
        .output()
        .expect("run corvus binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("request blocked"));
    assert!(stdout.contains("loop_session=session-cli-prod"));
}
