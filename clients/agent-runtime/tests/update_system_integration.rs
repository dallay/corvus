use corvus::{config::Config, gateway::admin, update};
use sha2::{Digest, Sha256};
use std::process::Command;

fn run_corvus(workspace: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_corvus"));
    command
        .args(args)
        .env("CORVUS_WORKSPACE", workspace)
        .env("CORVUS_DISABLE_UPDATE_CHECK", "1");
    command.output().expect("corvus command should execute")
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn make_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp workspace");
    std::fs::create_dir_all(dir.path().join("workspace").join("state"))
        .expect("workspace state dir");
    dir
}

fn hash_nonce(nonce: &str) -> String {
    let digest = Sha256::digest(nonce.as_bytes());
    hex::encode(digest)
}

#[test]
fn update_help_lists_full_command_contract() {
    let workspace = make_workspace();
    let output = run_corvus(workspace.path(), &["update", "--help"]);
    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("status"));
    assert!(stdout.contains("check"));
    assert!(stdout.contains("install"));
    assert!(stdout.contains("auto-enable"));
    assert!(stdout.contains("auto-disable"));
    assert!(stdout.contains("history"));
    assert!(stdout.contains("confirm"));
}

#[test]
fn update_status_and_policy_toggles_are_visible_across_commands() {
    let workspace = make_workspace();

    let enable = run_corvus(workspace.path(), &["update", "auto-enable"]);
    assert!(enable.status.success(), "{}", stderr_text(&enable));

    let status_enabled = run_corvus(workspace.path(), &["update", "status"]);
    assert!(
        status_enabled.status.success(),
        "{}",
        stderr_text(&status_enabled)
    );
    let stdout_enabled = stdout_text(&status_enabled);
    assert!(stdout_enabled.contains("policy.auto_install_enabled=true"));

    let disable = run_corvus(workspace.path(), &["update", "auto-disable"]);
    assert!(disable.status.success(), "{}", stderr_text(&disable));

    let status_disabled = run_corvus(workspace.path(), &["update", "status"]);
    assert!(
        status_disabled.status.success(),
        "{}",
        stderr_text(&status_disabled)
    );
    let stdout_disabled = stdout_text(&status_disabled);
    assert!(stdout_disabled.contains("policy.auto_install_enabled=false"));
}

#[test]
fn update_install_reports_busy_when_lock_is_held() {
    let workspace = make_workspace();
    let lock_path = workspace
        .path()
        .join("workspace")
        .join("state")
        .join("update_install.lock");
    std::fs::write(&lock_path, b"lock-holder").expect("create install lock");

    let output = run_corvus(workspace.path(), &["update", "install"]);
    assert!(!output.status.success());
    let combined = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(combined.contains("busy"));
}

#[test]
fn update_check_and_history_commands_are_script_stable() {
    let workspace = make_workspace();

    let check = run_corvus(workspace.path(), &["update", "check"]);
    let check_stdout = stdout_text(&check);
    assert!(
        check_stdout.contains("current_version=") || check_stdout.contains("latest_version="),
        "expected deterministic check output, got: {}",
        check_stdout
    );

    let history = run_corvus(workspace.path(), &["update", "history"]);
    assert!(history.status.success(), "{}", stderr_text(&history));
}

#[test]
fn update_confirm_reports_deterministic_failure_for_unknown_nonce() {
    let workspace = make_workspace();
    let output = run_corvus(workspace.path(), &["update", "confirm", "missing-nonce"]);
    assert!(!output.status.success());
    let combined = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(combined.contains("invalid"));
    assert!(combined.contains("nonce"));
}

#[test]
fn update_confirm_consumes_nonce_and_records_history_event() {
    let workspace = make_workspace();
    let state_path = workspace
        .path()
        .join("workspace")
        .join("state")
        .join("version_check.json");
    let nonce = "nonce-confirm-1";
    let state = serde_json::json!({
        "latest_version": "9.9.9",
        "checked_at_unix": 1,
        "update_available": true,
        "last_notified_version": "9.9.9",
        "pending_confirmations": [
            {
                "version": "9.9.9",
                "channel": "telegram",
                "recipient": "chat-1",
                "authorized_sender": "sender-1",
                "nonce_hash": hash_nonce(nonce),
                "expires_at_unix": 4102444800u64,
                "used": false
            }
        ],
        "notified_conversations": []
    });
    std::fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("serialize state"),
    )
    .expect("write state file");

    let _confirm = run_corvus(workspace.path(), &["update", "confirm", nonce]);

    let history = run_corvus(workspace.path(), &["update", "history"]);
    assert!(history.status.success(), "{}", stderr_text(&history));
    let history_stdout = stdout_text(&history);
    assert!(history_stdout.contains("confirm_install"));

    let confirm_reuse = run_corvus(workspace.path(), &["update", "confirm", nonce]);
    assert!(!confirm_reuse.status.success());
    let combined = format!(
        "{}\n{}",
        stdout_text(&confirm_reuse),
        stderr_text(&confirm_reuse)
    );
    assert!(combined.contains("invalid"));
}

#[test]
fn cli_and_admin_surfaces_share_update_status_facts() {
    let workspace = make_workspace();
    let config = Config {
        workspace_dir: workspace.path().to_path_buf(),
        config_path: workspace.path().join("config.toml"),
        ..Config::default()
    };

    let cli_view = update::get_update_status(&config, env!("CARGO_PKG_VERSION")).expect("status");
    let admin_view = admin::admin_config_view(&config);

    assert_eq!(
        admin_view.updates.status.current_version,
        cli_view.current_version
    );
    assert_eq!(
        admin_view.updates.status.latest_version,
        cli_view.latest_version
    );
    assert_eq!(
        admin_view.updates.status.update_available,
        cli_view.update_available
    );
    assert_eq!(
        admin_view.updates.status.last_check_outcome,
        cli_view.last_check_outcome
    );
    assert_eq!(
        admin_view.updates.status.last_check_at_unix,
        cli_view.last_check_at_unix
    );
    assert_eq!(
        admin_view.updates.status.effective_install_method,
        cli_view.effective_install_method
    );
}
