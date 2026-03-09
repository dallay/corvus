use crate::channels::traits::ChannelMessage;
use crate::channels::{Channel, SendMessage};
use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Global mutex that serializes all load → mutate → save sequences on version_check.json.
/// This prevents concurrent async tasks from racing on the same state file.
static STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn state_lock() -> &'static Mutex<()> {
    STATE_LOCK.get_or_init(|| Mutex::new(()))
}

const VERSION_CHECK_FILE: &str = "version_check.json";
const VERSION_CHECK_TTL_SECS: u64 = 24 * 60 * 60;
const VERSION_CHECK_TIMEOUT_SECS: u64 = 2;
const UPDATE_CHECK_DISABLE_ENV: &str = "CORVUS_DISABLE_UPDATE_CHECK";
const INSTALL_SCRIPT_URL: &str = "https://profiletailors.com/install";
const PACKAGE_NAME: &str = "@dallay/corvus";
const CONFIRM_COMMAND_PREFIX: &str = "corvus update confirm";
const UPDATE_STATE_LOCK_FILE: &str = "update_state.lock";
const UPDATE_INSTALL_LOCK_FILE: &str = "update_install.lock";
const UPDATE_HISTORY_FILE: &str = "update_history.jsonl";
const RELEASE_ENDPOINTS: [&str; 2] = [
    "https://api.github.com/repos/profiletailors/corvus/releases/latest",
    "https://api.github.com/repos/dallay/corvus/releases/latest",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallMethod {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Homebrew,
    Cargo,
    ScriptBinary,
    Unknown,
}

impl InstallMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Homebrew => "homebrew",
            Self::Cargo => "cargo",
            Self::ScriptBinary => "script_binary",
            Self::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for InstallMethod {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "npm" => Ok(Self::Npm),
            "pnpm" => Ok(Self::Pnpm),
            "yarn" => Ok(Self::Yarn),
            "bun" => Ok(Self::Bun),
            "homebrew" => Ok(Self::Homebrew),
            "cargo" => Ok(Self::Cargo),
            "script_binary" => Ok(Self::ScriptBinary),
            "unknown" => Ok(Self::Unknown),
            other => anyhow::bail!("unsupported install method: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    Never,
    Prompt,
    AutoManagedService,
}

impl RestartPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Prompt => "prompt",
            Self::AutoManagedService => "auto_managed_service",
        }
    }
}

impl std::str::FromStr for RestartPolicy {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "never" => Ok(Self::Never),
            "prompt" => Ok(Self::Prompt),
            "auto_managed_service" => Ok(Self::AutoManagedService),
            other => anyhow::bail!("unsupported restart policy: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallState {
    Idle,
    Installing {
        tx_id: String,
        started_at_unix: u64,
    },
    InstalledPendingRestart {
        version: String,
        installed_at_unix: u64,
    },
    Failed {
        tx_id: String,
        failed_at_unix: u64,
        reason_code: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Success,
    NetworkError,
    ParseError,
    SourceRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdatePolicy {
    pub checks_enabled: bool,
    pub auto_install_enabled: bool,
    pub channel_visibility_enabled: bool,
    pub cli_startup_notice_enabled: bool,
    pub check_interval_minutes: u64,
    pub confirmation_ttl_minutes: u64,
    pub install_method_override: Option<InstallMethod>,
    pub restart_policy: RestartPolicy,
    pub history_max_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStateSnapshot {
    pub schema_version: u32,
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub last_check_at_unix: u64,
    pub last_check_outcome: CheckOutcome,
    pub effective_method: InstallMethod,
    pub detected_method: Option<InstallMethod>,
    pub overridden_method: Option<InstallMethod>,
    pub install_state: InstallState,
    #[serde(default)]
    pending_confirmations: Vec<PendingConfirmation>,
    #[serde(default)]
    notified_conversations: Vec<NotifiedConversation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdatePolicyView {
    pub checks_enabled: bool,
    pub auto_install_enabled: bool,
    pub channel_visibility_enabled: bool,
    pub cli_startup_notice_enabled: bool,
    pub restart_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusView {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub last_check_at_unix: Option<u64>,
    pub last_check_outcome: Option<String>,
    pub effective_install_method: String,
    pub detected_install_method: Option<String>,
    pub install_method_source: String,
    pub policy: UpdatePolicyView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAuditEvent {
    pub event_id: String,
    pub timestamp_unix: u64,
    pub action: String,
    pub outcome: String,
    pub current_version: String,
    pub target_version: Option<String>,
    pub effective_method: String,
    pub actor: String,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateManager {
    workspace_dir: PathBuf,
    policy: UpdatePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCommandOutcome {
    Success,
    NoUpdate,
    Blocked,
    Busy,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCommandOutcome {
    Success,
    InvalidNonce,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartAction {
    None,
    Prompt,
    ManagedService,
}

pub fn restart_action_for_install_state(
    install_state: &InstallState,
    policy: RestartPolicy,
) -> RestartAction {
    match install_state {
        InstallState::InstalledPendingRestart { .. } => match policy {
            RestartPolicy::Never => RestartAction::None,
            RestartPolicy::Prompt => RestartAction::Prompt,
            RestartPolicy::AutoManagedService => RestartAction::ManagedService,
        },
        _ => RestartAction::None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionCheckState {
    latest_version: String,
    checked_at_unix: u64,
    update_available: bool,
    #[serde(default)]
    last_notified_version: Option<String>,
    #[serde(default)]
    pending_confirmations: Vec<PendingConfirmation>,
    #[serde(default)]
    notified_conversations: Vec<NotifiedConversation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct NotifiedConversation {
    version: String,
    channel: String,
    recipient: String,
    authorized_sender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingConfirmation {
    version: String,
    channel: String,
    recipient: String,
    authorized_sender: Option<String>,
    nonce_hash: String,
    expires_at_unix: u64,
    used: bool,
}

#[derive(Debug, Deserialize)]
struct LatestReleaseResponse {
    tag_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateNotice {
    current_version: String,
    latest_version: String,
}

#[derive(Debug, Clone)]
struct NotificationTarget {
    channel: String,
    recipient: String,
    authorized_sender: Option<String>,
}

#[derive(Debug)]
struct UpdateExecutionResult {
    summary: String,
    succeeded: bool,
    reason_code: Option<String>,
}

impl UpdatePolicy {
    pub fn from_config(config: &Config) -> Self {
        let install_method_override = config
            .updates
            .install_method_override
            .as_deref()
            .and_then(|raw| raw.parse::<InstallMethod>().ok());
        let restart_policy = config
            .updates
            .restart_policy
            .parse::<RestartPolicy>()
            .unwrap_or(RestartPolicy::Prompt);

        Self {
            checks_enabled: config.updates.enabled,
            auto_install_enabled: config.updates.auto_install_enabled,
            channel_visibility_enabled: config.updates.channel_visibility_enabled,
            cli_startup_notice_enabled: config.updates.cli_startup_notice_enabled,
            check_interval_minutes: config.updates.check_interval_minutes,
            confirmation_ttl_minutes: config.updates.confirmation_ttl_minutes,
            install_method_override,
            restart_policy,
            history_max_entries: config.updates.history_max_entries.max(1),
        }
    }
}

impl UpdateStateSnapshot {
    fn initial(current_version: &str, policy: &UpdatePolicy) -> Self {
        let detected_method = detect_install_method();
        let (effective_method, overridden_method, _source) = resolve_install_method(
            policy.install_method_override.clone(),
            detected_method.clone(),
        );

        Self {
            schema_version: 2,
            current_version: normalize_version(current_version).unwrap_or_else(|| "0.0.0".into()),
            latest_version: normalize_version(current_version).unwrap_or_else(|| "0.0.0".into()),
            update_available: false,
            last_check_at_unix: 0,
            last_check_outcome: CheckOutcome::Success,
            effective_method,
            detected_method,
            overridden_method,
            install_state: InstallState::Idle,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        }
    }

    fn to_status_view(&self, policy: &UpdatePolicy) -> UpdateStatusView {
        let source = if self.overridden_method.is_some() {
            "override"
        } else if self.detected_method.is_some() {
            "detected"
        } else {
            "unknown"
        };
        UpdateStatusView {
            current_version: self.current_version.clone(),
            latest_version: Some(self.latest_version.clone()),
            update_available: self.update_available,
            last_check_at_unix: Some(self.last_check_at_unix),
            last_check_outcome: Some(format!("{:?}", self.last_check_outcome).to_ascii_lowercase()),
            effective_install_method: self.effective_method.as_str().to_string(),
            detected_install_method: self
                .detected_method
                .as_ref()
                .map(|method| method.as_str().to_string()),
            install_method_source: source.to_string(),
            policy: UpdatePolicyView {
                checks_enabled: policy.checks_enabled,
                auto_install_enabled: policy.auto_install_enabled,
                channel_visibility_enabled: policy.channel_visibility_enabled,
                cli_startup_notice_enabled: policy.cli_startup_notice_enabled,
                restart_policy: policy.restart_policy.as_str().to_string(),
            },
        }
    }
}

impl From<VersionCheckState> for UpdateStateSnapshot {
    fn from(value: VersionCheckState) -> Self {
        Self {
            schema_version: 2,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: value.latest_version,
            update_available: value.update_available,
            last_check_at_unix: value.checked_at_unix,
            last_check_outcome: CheckOutcome::Success,
            effective_method: InstallMethod::Unknown,
            detected_method: None,
            overridden_method: None,
            install_state: InstallState::Idle,
            pending_confirmations: value.pending_confirmations,
            notified_conversations: value.notified_conversations,
        }
    }
}

impl UpdateManager {
    pub fn new(config: &Config) -> Self {
        Self {
            workspace_dir: config.workspace_dir.clone(),
            policy: UpdatePolicy::from_config(config),
        }
    }

    pub fn status_sync(&self, current_version: &str) -> Result<UpdateStatusView> {
        let mut snapshot = load_state_snapshot_sync(&self.workspace_dir)?
            .unwrap_or_else(|| UpdateStateSnapshot::initial(current_version, &self.policy));
        let detected_method = detect_install_method();
        let (effective, overridden, _) = resolve_install_method(
            self.policy.install_method_override.clone(),
            detected_method.clone(),
        );
        snapshot.effective_method = effective;
        snapshot.detected_method = detected_method;
        snapshot.overridden_method = overridden;
        Ok(snapshot.to_status_view(&self.policy))
    }

    pub async fn force_check(
        &self,
        current_version: &str,
        actor: &str,
    ) -> Result<UpdateStatusView> {
        let _state_lock = acquire_file_lock(&update_state_lock_path(&self.workspace_dir), 200)?;
        let mut snapshot = load_state_snapshot_sync(&self.workspace_dir)?
            .unwrap_or_else(|| UpdateStateSnapshot::initial(current_version, &self.policy));

        let current = normalize_version(current_version)
            .ok_or_else(|| anyhow::anyhow!("invalid current version: {current_version}"))?;

        match fetch_latest_release_version().await {
            Ok(latest) => {
                snapshot.latest_version = latest.clone();
                snapshot.last_check_at_unix = now_unix_secs();
                snapshot.update_available =
                    compare_semverish(&latest, &current).is_some_and(|ordering| ordering.is_gt());
                snapshot.last_check_outcome = CheckOutcome::Success;
            }
            Err(_) => {
                snapshot.last_check_at_unix = now_unix_secs();
                snapshot.last_check_outcome = CheckOutcome::NetworkError;
            }
        }

        save_state_snapshot_sync(&self.workspace_dir, &snapshot)?;
        append_audit_event_sync(
            &self.workspace_dir,
            &self.policy,
            UpdateAuditEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp_unix: now_unix_secs(),
                action: "check".to_string(),
                outcome: match snapshot.last_check_outcome {
                    CheckOutcome::Success => "success".to_string(),
                    _ => "failed".to_string(),
                },
                current_version: snapshot.current_version.clone(),
                target_version: Some(snapshot.latest_version.clone()),
                effective_method: snapshot.effective_method.as_str().to_string(),
                actor: actor.to_string(),
                reason_code: None,
            },
        )?;

        Ok(snapshot.to_status_view(&self.policy))
    }

    pub fn set_auto_install_enabled(&self, config: &mut Config, enabled: bool) -> Result<()> {
        config.updates.auto_install_enabled = enabled;
        config.save()
    }

    pub fn install(
        &self,
        current_version: &str,
        actor: &str,
    ) -> Result<(InstallCommandOutcome, String)> {
        let _install_lock =
            match acquire_file_lock(&update_install_lock_path(&self.workspace_dir), 50) {
                Ok(lock) => lock,
                Err(_) => {
                    return Ok((
                        InstallCommandOutcome::Busy,
                        "update install busy: another install transaction is active".to_string(),
                    ));
                }
            };
        let _state_lock = acquire_file_lock(&update_state_lock_path(&self.workspace_dir), 200)?;
        let mut snapshot = load_state_snapshot_sync(&self.workspace_dir)?
            .unwrap_or_else(|| UpdateStateSnapshot::initial(current_version, &self.policy));

        if !snapshot.update_available {
            return Ok((
                InstallCommandOutcome::NoUpdate,
                "no update available".to_string(),
            ));
        }

        let detected_method = detect_install_method();
        let (effective, overridden, source) = resolve_install_method(
            self.policy.install_method_override.clone(),
            detected_method.clone(),
        );
        snapshot.effective_method = effective.clone();
        snapshot.detected_method = detected_method;
        snapshot.overridden_method = overridden;

        if effective == InstallMethod::Unknown {
            snapshot.install_state = InstallState::Failed {
                tx_id: uuid::Uuid::new_v4().to_string(),
                failed_at_unix: now_unix_secs(),
                reason_code: "unsupported_method".to_string(),
            };
            save_state_snapshot_sync(&self.workspace_dir, &snapshot)?;
            append_audit_event_sync(
                &self.workspace_dir,
                &self.policy,
                UpdateAuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp_unix: now_unix_secs(),
                    action: "install".to_string(),
                    outcome: "failed".to_string(),
                    current_version: snapshot.current_version.clone(),
                    target_version: Some(snapshot.latest_version.clone()),
                    effective_method: "unknown".to_string(),
                    actor: actor.to_string(),
                    reason_code: Some("unsupported_method".to_string()),
                },
            )?;
            return Ok((
                InstallCommandOutcome::Blocked,
                "install method unsupported; use manual update instructions".to_string(),
            ));
        }

        if effective == InstallMethod::ScriptBinary {
            let artifact_path = std::env::var("CORVUS_UPDATE_ARTIFACT_PATH").ok();
            let expected_sha = std::env::var("CORVUS_UPDATE_EXPECTED_SHA256").ok();
            let verification_result = match (artifact_path.as_deref(), expected_sha.as_deref()) {
                (Some(path), Some(expected)) => verify_sha256_checksum(Path::new(path), expected),
                _ => Err(anyhow::anyhow!("missing checksum metadata")),
            };
            if let Err(error) = verification_result {
                snapshot.install_state = InstallState::Failed {
                    tx_id: uuid::Uuid::new_v4().to_string(),
                    failed_at_unix: now_unix_secs(),
                    reason_code: "verification_failed".to_string(),
                };
                save_state_snapshot_sync(&self.workspace_dir, &snapshot)?;
                append_audit_event_sync(
                    &self.workspace_dir,
                    &self.policy,
                    UpdateAuditEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        timestamp_unix: now_unix_secs(),
                        action: "verification".to_string(),
                        outcome: "failed".to_string(),
                        current_version: snapshot.current_version.clone(),
                        target_version: Some(snapshot.latest_version.clone()),
                        effective_method: effective.as_str().to_string(),
                        actor: actor.to_string(),
                        reason_code: Some(error.to_string()),
                    },
                )?;
                return Ok((
                    InstallCommandOutcome::Blocked,
                    "install blocked by verification".to_string(),
                ));
            }

            append_audit_event_sync(
                &self.workspace_dir,
                &self.policy,
                UpdateAuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp_unix: now_unix_secs(),
                    action: "verification".to_string(),
                    outcome: "success".to_string(),
                    current_version: snapshot.current_version.clone(),
                    target_version: Some(snapshot.latest_version.clone()),
                    effective_method: effective.as_str().to_string(),
                    actor: actor.to_string(),
                    reason_code: None,
                },
            )?;
        }

        snapshot.install_state = InstallState::InstalledPendingRestart {
            version: snapshot.latest_version.clone(),
            installed_at_unix: now_unix_secs(),
        };
        save_state_snapshot_sync(&self.workspace_dir, &snapshot)?;
        append_audit_event_sync(
            &self.workspace_dir,
            &self.policy,
            UpdateAuditEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp_unix: now_unix_secs(),
                action: "install".to_string(),
                outcome: "success".to_string(),
                current_version: snapshot.current_version.clone(),
                target_version: Some(snapshot.latest_version.clone()),
                effective_method: effective.as_str().to_string(),
                actor: actor.to_string(),
                reason_code: Some(format!("source:{source}")),
            },
        )?;

        Ok((
            InstallCommandOutcome::Success,
            format!(
                "update installed to {} via {}",
                snapshot.latest_version,
                effective.as_str()
            ),
        ))
    }

    pub fn history(&self) -> Result<Vec<UpdateAuditEvent>> {
        read_update_history_sync(&self.workspace_dir)
    }
}

pub fn get_update_status(config: &Config, current_version: &str) -> Result<UpdateStatusView> {
    UpdateManager::new(config).status_sync(current_version)
}

pub async fn run_update_check(config: &Config, current_version: &str) -> Result<UpdateStatusView> {
    UpdateManager::new(config)
        .force_check(current_version, "cli:update-check")
        .await
}

pub fn run_update_install(
    config: &Config,
    current_version: &str,
) -> Result<(InstallCommandOutcome, String)> {
    UpdateManager::new(config).install(current_version, "cli:update-install")
}

pub async fn run_update_confirm(
    config: &Config,
    nonce: &str,
) -> Result<(ConfirmCommandOutcome, String)> {
    process_update_confirmation(config, nonce, "cli:update-confirm").await
}

pub fn set_auto_update_policy(config: &mut Config, enabled: bool) -> Result<()> {
    UpdateManager::new(config).set_auto_install_enabled(config, enabled)
}

pub fn read_update_history(config: &Config) -> Result<Vec<UpdateAuditEvent>> {
    UpdateManager::new(config).history()
}

pub async fn maybe_print_update_notice(config: &Config) {
    if is_update_check_disabled() || !config.updates.cli_startup_notice_enabled {
        return;
    }

    if let Some(notice) = check_for_update(config, env!("CARGO_PKG_VERSION")).await {
        println!();
        println!(
            "⬆️  Update available: v{} (current v{})",
            notice.latest_version, notice.current_version
        );
        println!(
            "   If installed via script/binary: curl -fsSL {} | bash",
            INSTALL_SCRIPT_URL
        );
        println!(
            "   If installed via package manager: npm i -g {}@latest (or pnpm/yarn/bun)",
            PACKAGE_NAME
        );
    }
}

pub async fn run_daemon_update_watcher(config: Config) -> Result<()> {
    if is_update_check_disabled() || !config.updates.enabled {
        return Ok(());
    }

    loop {
        if let Err(error) = poll_and_notify_update(&config, env!("CARGO_PKG_VERSION")).await {
            tracing::warn!("daemon update check failed: {error}");
        }

        let interval_minutes = config.updates.check_interval_minutes.max(1);
        tokio::time::sleep(Duration::from_secs(interval_minutes * 60)).await;
    }
}

async fn process_update_confirmation(
    config: &Config,
    raw_nonce: &str,
    actor: &str,
) -> Result<(ConfirmCommandOutcome, String)> {
    let nonce = raw_nonce.trim();
    if nonce.is_empty() {
        return Ok((
            ConfirmCommandOutcome::InvalidNonce,
            "invalid, expired, or already-used update confirmation nonce".to_string(),
        ));
    }

    let _guard = state_lock().lock().await;
    let state_path = version_check_path(&config.workspace_dir);
    let mut state = match load_state(&state_path).await? {
        Some(state) => state,
        None => {
            return Ok((
                ConfirmCommandOutcome::InvalidNonce,
                "no pending update confirmation was found".to_string(),
            ));
        }
    };

    prune_pending_confirmations(&mut state.pending_confirmations);
    let version = match consume_pending_confirmation(&mut state, nonce, None) {
        Ok(version) => version,
        Err(_) => {
            save_state(&state_path, &state).await?;
            return Ok((
                ConfirmCommandOutcome::InvalidNonce,
                "invalid, expired, or already-used update confirmation nonce".to_string(),
            ));
        }
    };

    save_state(&state_path, &state).await?;

    let result = execute_minimal_update_strategy(&version).await;
    append_confirmation_audit_event(
        &config.workspace_dir,
        &UpdatePolicy::from_config(config),
        &version,
        result.succeeded,
        actor,
        result.reason_code.as_deref(),
    )?;

    let outcome = if result.succeeded {
        ConfirmCommandOutcome::Success
    } else {
        ConfirmCommandOutcome::Failed
    };
    Ok((outcome, result.summary))
}

pub async fn try_handle_channel_update_confirmation(
    config: &Config,
    msg: &ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
) -> bool {
    let Some(raw_nonce) = parse_confirmation_nonce(&msg.content) else {
        return false;
    };

    let Some(channel) = target_channel else {
        return true;
    };

    let _guard = state_lock().lock().await;

    let state_path = version_check_path(&config.workspace_dir);
    let mut state = match load_state(&state_path).await {
        Ok(Some(state)) => state,
        Ok(None) => {
            let _ = channel
                .send(&SendMessage::new(
                    "No pending update confirmation was found.",
                    &msg.reply_target,
                ))
                .await;
            return true;
        }
        Err(error) => {
            let _ = channel
                .send(&SendMessage::new(
                    format!("Update state read failed: {error}"),
                    &msg.reply_target,
                ))
                .await;
            return true;
        }
    };

    prune_pending_confirmations(&mut state.pending_confirmations);

    if !is_sender_authorized(config, &msg.channel, &msg.sender) {
        let _ = channel
            .send(&SendMessage::new(
                "Unauthorized sender for update confirmation.",
                &msg.reply_target,
            ))
            .await;
        return true;
    }

    let Some(version) = consume_pending_confirmation(
        &mut state,
        raw_nonce,
        Some((&msg.channel, &msg.reply_target, &msg.sender)),
    )
    .ok() else {
        let _ = channel
            .send(&SendMessage::new(
                "Invalid, expired, or already-used update confirmation nonce.",
                &msg.reply_target,
            ))
            .await;
        let _ = save_state(&state_path, &state).await;
        return true;
    };

    if let Err(error) = save_state(&state_path, &state).await {
        let _ = channel
            .send(&SendMessage::new(
                format!("Failed to persist confirmation state: {error}"),
                &msg.reply_target,
            ))
            .await;
        return true;
    }

    let result = execute_minimal_update_strategy(&version).await;
    let _ = append_confirmation_audit_event(
        &config.workspace_dir,
        &UpdatePolicy::from_config(config),
        &version,
        result.succeeded,
        "channel:update-confirm",
        result.reason_code.as_deref(),
    );
    let _ = channel
        .send(&SendMessage::new(result.summary, &msg.reply_target))
        .await;

    true
}

pub async fn maybe_send_opportunistic_update_notice(
    config: &Config,
    msg: &ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    current_version: &str,
) -> Result<bool> {
    if is_update_check_disabled()
        || !config.updates.enabled
        || !config.updates.channel_visibility_enabled
    {
        return Ok(false);
    }

    let Some(channel) = target_channel else {
        return Ok(false);
    };

    if !is_sender_authorized(config, &msg.channel, &msg.sender) {
        return Ok(false);
    }

    let Some(current) = normalize_version(current_version) else {
        tracing::warn!(
            "invalid current version for opportunistic update notice: {current_version}"
        );
        return Ok(false);
    };

    let _guard = state_lock().lock().await;

    let state_path = version_check_path(&config.workspace_dir);
    let mut state = match load_state(&state_path).await {
        Ok(Some(state)) => state,
        Ok(None) => VersionCheckState {
            latest_version: current.clone(),
            checked_at_unix: 0,
            update_available: false,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        },
        Err(error) => {
            tracing::warn!("opportunistic update notice skipped: state load failed: {error}");
            return Ok(false);
        }
    };

    prune_pending_confirmations(&mut state.pending_confirmations);

    if is_stale(&state) {
        if let Ok(fetched) = fetch_latest_release_version().await {
            state.latest_version = fetched.clone();
            state.checked_at_unix = now_unix_secs();
            state.update_available =
                compare_semverish(&fetched, &current).is_some_and(|ordering| ordering.is_gt());
        } else {
            state.checked_at_unix = now_unix_secs();
        }
    }

    let has_update = state.update_available
        && compare_semverish(&state.latest_version, &current)
            .is_some_and(|ordering| ordering.is_gt());

    if !has_update {
        state.pending_confirmations.clear();
        state.notified_conversations.clear();
        save_state(&state_path, &state).await?;
        return Ok(false);
    }

    state
        .notified_conversations
        .retain(|notice| notice.version == state.latest_version);

    if state.notified_conversations.iter().any(|notice| {
        notice.version == state.latest_version
            && notice.channel.eq_ignore_ascii_case(&msg.channel)
            && notice.recipient == msg.reply_target
            && notice.authorized_sender == msg.sender
    }) {
        save_state(&state_path, &state).await?;
        return Ok(false);
    }

    let now = now_unix_secs();
    let ttl_secs = config.updates.confirmation_ttl_minutes.max(1) * 60;
    let expires_at_unix = now.saturating_add(ttl_secs);
    let nonce = uuid::Uuid::new_v4().simple().to_string();

    let sent = send_update_notification_via_channel(
        channel,
        &msg.reply_target,
        &state.latest_version,
        &current,
        &nonce,
        expires_at_unix,
    )
    .await;

    if sent {
        state.pending_confirmations.push(PendingConfirmation {
            version: state.latest_version.clone(),
            channel: msg.channel.clone(),
            recipient: msg.reply_target.clone(),
            authorized_sender: Some(msg.sender.clone()),
            nonce_hash: hash_nonce(&nonce),
            expires_at_unix,
            used: false,
        });
        state.notified_conversations.push(NotifiedConversation {
            version: state.latest_version.clone(),
            channel: msg.channel.clone(),
            recipient: msg.reply_target.clone(),
            authorized_sender: msg.sender.clone(),
        });
    }

    save_state(&state_path, &state).await?;
    Ok(sent)
}

async fn poll_and_notify_update(config: &Config, current_version: &str) -> Result<()> {
    let current = normalize_version(current_version)
        .ok_or_else(|| anyhow::anyhow!("invalid current version: {current_version}"))?;

    let _guard = state_lock().lock().await;

    let state_path = version_check_path(&config.workspace_dir);

    let mut state = load_state(&state_path).await?.unwrap_or(VersionCheckState {
        latest_version: current.clone(),
        checked_at_unix: 0,
        update_available: false,
        last_notified_version: None,
        pending_confirmations: Vec::new(),
        notified_conversations: Vec::new(),
    });

    prune_pending_confirmations(&mut state.pending_confirmations);

    if is_stale(&state) {
        if let Ok(fetched) = fetch_latest_release_version().await {
            state.latest_version = fetched.clone();
            state.checked_at_unix = now_unix_secs();
            state.update_available =
                compare_semverish(&fetched, &current).is_some_and(|ordering| ordering.is_gt());
        } else {
            state.checked_at_unix = now_unix_secs();
        }
    }

    let has_update = state.update_available
        && compare_semverish(&state.latest_version, &current)
            .is_some_and(|ordering| ordering.is_gt());

    if !has_update {
        state.pending_confirmations.clear();
        state.notified_conversations.clear();
        save_state(&state_path, &state).await?;
        return Ok(());
    }

    if state.last_notified_version.as_deref() == Some(state.latest_version.as_str()) {
        save_state(&state_path, &state).await?;
        return Ok(());
    }

    let targets = collect_notification_targets(config);
    if targets.is_empty() {
        tracing::info!(
            "Update v{} available but no notification destinations configured",
            state.latest_version
        );
        save_state(&state_path, &state).await?;
        return Ok(());
    }

    let now = now_unix_secs();
    let ttl_secs = config.updates.confirmation_ttl_minutes.max(1) * 60;
    let expires_at_unix = now.saturating_add(ttl_secs);

    let mut confirmations = Vec::new();
    for target in targets {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let sent = send_update_notification(
            config,
            &target,
            &state.latest_version,
            &current,
            &nonce,
            expires_at_unix,
        )
        .await;
        if sent {
            confirmations.push(PendingConfirmation {
                version: state.latest_version.clone(),
                channel: target.channel,
                recipient: target.recipient,
                authorized_sender: target.authorized_sender,
                nonce_hash: hash_nonce(&nonce),
                expires_at_unix,
                used: false,
            });
        }
    }

    if !confirmations.is_empty() {
        state.pending_confirmations.extend(confirmations);
        state.last_notified_version = Some(state.latest_version.clone());
    }

    save_state(&state_path, &state).await?;
    Ok(())
}

fn parse_confirmation_nonce(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    let mut parts = trimmed.split_whitespace();
    let first = parts.next()?;
    let second = parts.next()?;
    let third = parts.next()?;
    let nonce = parts.next()?;

    if first.eq_ignore_ascii_case("corvus")
        && second.eq_ignore_ascii_case("update")
        && third.eq_ignore_ascii_case("confirm")
        && parts.next().is_none()
    {
        return Some(nonce);
    }

    None
}

fn hash_nonce(nonce: &str) -> String {
    let digest = Sha256::digest(nonce.as_bytes());
    hex::encode(digest)
}

fn consume_pending_confirmation(
    state: &mut VersionCheckState,
    raw_nonce: &str,
    channel_scope: Option<(&str, &str, &str)>,
) -> Result<String> {
    let nonce_hash = hash_nonce(raw_nonce);
    let Some(pending) = state.pending_confirmations.iter_mut().find(|pending| {
        if pending.used
            || pending.nonce_hash != nonce_hash
            || pending.expires_at_unix <= now_unix_secs()
        {
            return false;
        }

        if let Some((channel, recipient, sender)) = channel_scope {
            pending.channel.eq_ignore_ascii_case(channel)
                && pending.recipient.eq_ignore_ascii_case(recipient)
                && pending
                    .authorized_sender
                    .as_ref()
                    .is_none_or(|authorized| authorized == sender)
        } else {
            true
        }
    }) else {
        anyhow::bail!("pending confirmation not found")
    };

    pending.used = true;
    Ok(pending.version.clone())
}

fn append_confirmation_audit_event(
    workspace_dir: &Path,
    policy: &UpdatePolicy,
    target_version: &str,
    success: bool,
    actor: &str,
    reason_code: Option<&str>,
) -> Result<()> {
    let detected = detect_install_method();
    let (effective, _, _) =
        resolve_install_method(policy.install_method_override.clone(), detected);
    append_audit_event_sync(
        workspace_dir,
        policy,
        UpdateAuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp_unix: now_unix_secs(),
            action: "confirm_install".to_string(),
            outcome: if success { "success" } else { "failed" }.to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            target_version: Some(target_version.to_string()),
            effective_method: effective.as_str().to_string(),
            actor: actor.to_string(),
            reason_code: reason_code.map(std::string::ToString::to_string),
        },
    )
}

fn prune_pending_confirmations(confirmations: &mut Vec<PendingConfirmation>) {
    let now = now_unix_secs();
    confirmations.retain(|pending| !pending.used && pending.expires_at_unix > now);
}

fn is_sender_authorized(config: &Config, channel: &str, sender: &str) -> bool {
    let sender = sender.trim();
    if sender.is_empty() {
        return false;
    }

    match channel {
        "telegram" => config
            .channels_config
            .telegram
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "discord" => config
            .channels_config
            .discord
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "slack" => config
            .channels_config
            .slack
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "mattermost" => config
            .channels_config
            .mattermost
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "imessage" => config
            .channels_config
            .imessage
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_contacts, sender)),
        "matrix" => config
            .channels_config
            .matrix
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "signal" => config
            .channels_config
            .signal
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_from, sender)),
        "whatsapp" => config
            .channels_config
            .whatsapp
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_numbers, sender)),
        "email" => config
            .channels_config
            .email
            .as_ref()
            .is_some_and(|cfg| allowlist_match_email(&cfg.allowed_senders, sender)),
        "irc" => config
            .channels_config
            .irc
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "lark" => config
            .channels_config
            .lark
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "dingtalk" => config
            .channels_config
            .dingtalk
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        "qq" => config
            .channels_config
            .qq
            .as_ref()
            .is_some_and(|cfg| allowlist_match(&cfg.allowed_users, sender)),
        _ => false,
    }
}

fn allowlist_match(allowlist: &[String], sender: &str) -> bool {
    if allowlist.is_empty() {
        return false;
    }
    if allowlist.iter().any(|entry| entry == "*") {
        return true;
    }
    allowlist
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(sender))
}

fn allowlist_match_email(allowlist: &[String], sender: &str) -> bool {
    if allowlist.is_empty() {
        return false;
    }
    if allowlist.iter().any(|entry| entry == "*") {
        return true;
    }

    let sender_lower = sender.to_ascii_lowercase();
    allowlist.iter().any(|entry| {
        if entry.starts_with('@') {
            sender_lower.ends_with(&entry.to_ascii_lowercase())
        } else if entry.contains('@') {
            entry.eq_ignore_ascii_case(sender)
        } else {
            sender_lower.ends_with(&format!("@{}", entry.to_ascii_lowercase()))
        }
    })
}

fn collect_notification_targets(config: &Config) -> Vec<NotificationTarget> {
    let mut targets = Vec::new();

    let mut push_target = |channel: &str, recipient: String, authorized_sender: Option<String>| {
        if recipient.trim().is_empty() {
            return;
        }
        targets.push(NotificationTarget {
            channel: channel.to_string(),
            recipient,
            authorized_sender,
        });
    };

    collect_from_notify_destinations(config, &mut push_target);
    collect_from_registered_channels(config, &mut push_target);

    targets.sort_by(|left, right| {
        left.channel
            .cmp(&right.channel)
            .then(left.recipient.cmp(&right.recipient))
            .then(left.authorized_sender.cmp(&right.authorized_sender))
    });
    targets.dedup_by(|left, right| {
        left.channel == right.channel
            && left.recipient == right.recipient
            && left.authorized_sender == right.authorized_sender
    });
    targets
}

fn collect_from_notify_destinations(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    for (channel, recipients) in &config.updates.notify_destinations {
        for recipient in recipients {
            let authorized_sender = infer_authorized_sender(channel, recipient);
            push_target(channel, recipient.clone(), authorized_sender);
        }
    }
}

fn collect_from_registered_channels(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    collect_slack_targets(config, push_target);
    collect_mattermost_targets(config, push_target);
    collect_matrix_targets(config, push_target);
    collect_signal_targets(config, push_target);
    collect_irc_targets(config, push_target);
    collect_imessage_targets(config, push_target);
    collect_whatsapp_targets(config, push_target);
    collect_email_targets(config, push_target);
    collect_telegram_targets(config, push_target);
    collect_discord_targets(config, push_target);
    collect_lark_targets(config, push_target);
    collect_dingtalk_targets(config, push_target);
    collect_qq_targets(config, push_target);
}

fn collect_slack_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(slack) = &config.channels_config.slack {
        if let Some(channel_id) = &slack.channel_id {
            push_target("slack", channel_id.clone(), None);
        }
    }
}

fn collect_mattermost_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(mattermost) = &config.channels_config.mattermost {
        if let Some(channel_id) = &mattermost.channel_id {
            push_target("mattermost", channel_id.clone(), None);
        }
    }
}

fn collect_matrix_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(matrix) = &config.channels_config.matrix {
        push_target("matrix", matrix.room_id.clone(), None);
    }
}

fn collect_signal_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(signal) = &config.channels_config.signal {
        if let Some(group_id) = &signal.group_id {
            push_target("signal", format!("group:{group_id}"), None);
        }
    }
}

fn collect_irc_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(irc) = &config.channels_config.irc {
        for room in &irc.channels {
            push_target("irc", room.clone(), None);
        }
    }
}

fn collect_imessage_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(imessage) = &config.channels_config.imessage {
        for contact in &imessage.allowed_contacts {
            if contact != "*" {
                push_target("imessage", contact.clone(), Some(contact.clone()));
            }
        }
    }
}

fn collect_whatsapp_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(whatsapp) = &config.channels_config.whatsapp {
        for number in &whatsapp.allowed_numbers {
            if number != "*" {
                push_target("whatsapp", number.clone(), Some(number.clone()));
            }
        }
    }
}

fn collect_email_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(email) = &config.channels_config.email {
        for sender in &email.allowed_senders {
            if is_valid_email_sender(sender) {
                push_target("email", sender.clone(), Some(sender.clone()));
            }
        }
    }
}

fn is_valid_email_sender(sender: &str) -> bool {
    sender.contains('@') && sender != "*" && !sender.starts_with('@')
}

fn collect_telegram_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(telegram) = &config.channels_config.telegram {
        for user in &telegram.allowed_users {
            if is_numeric_user(user) {
                push_target("telegram", user.clone(), Some(user.clone()));
            }
        }
    }
}

fn is_numeric_user(user: &str) -> bool {
    if user == "*" {
        return false;
    }
    let digits = user.strip_prefix('-').unwrap_or(user);
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

fn collect_discord_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(discord) = &config.channels_config.discord {
        for user in &discord.allowed_users {
            if user != "*" {
                push_target("discord", user.clone(), Some(user.clone()));
            }
        }
    }
}

fn collect_lark_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(lark) = &config.channels_config.lark {
        for user in &lark.allowed_users {
            if user != "*" {
                push_target("lark", user.clone(), Some(user.clone()));
            }
        }
    }
}

fn collect_dingtalk_targets(
    config: &Config,
    push_target: &mut impl FnMut(&str, String, Option<String>),
) {
    if let Some(dingtalk) = &config.channels_config.dingtalk {
        for user in &dingtalk.allowed_users {
            if user != "*" {
                push_target("dingtalk", user.clone(), Some(user.clone()));
            }
        }
    }
}

fn collect_qq_targets(config: &Config, push_target: &mut impl FnMut(&str, String, Option<String>)) {
    if let Some(qq) = &config.channels_config.qq {
        for user in &qq.allowed_users {
            if user != "*" {
                push_target("qq", user.clone(), Some(user.clone()));
            }
        }
    }
}

fn infer_authorized_sender(channel: &str, recipient: &str) -> Option<String> {
    match channel {
        "telegram" | "signal" | "whatsapp" | "imessage" | "email" | "discord" | "lark"
        | "dingtalk" | "qq" => Some(recipient.to_string()),
        _ => None,
    }
}

async fn send_update_notification(
    config: &Config,
    target: &NotificationTarget,
    latest_version: &str,
    current_version: &str,
    nonce: &str,
    expires_at_unix: u64,
) -> bool {
    let channel_name = target.channel.to_ascii_lowercase();
    let Some(channel) = crate::channels::build_channel(config, &channel_name) else {
        tracing::warn!(
            "update notification skipped: unsupported/unconfigured channel={}",
            target.channel
        );
        return false;
    };

    send_update_notification_via_channel(
        &channel,
        &target.recipient,
        latest_version,
        current_version,
        nonce,
        expires_at_unix,
    )
    .await
}

async fn send_update_notification_via_channel(
    channel: &Arc<dyn Channel>,
    recipient: &str,
    latest_version: &str,
    current_version: &str,
    nonce: &str,
    expires_at_unix: u64,
) -> bool {
    let expires_iso = chrono::DateTime::<chrono::Utc>::from_timestamp(expires_at_unix as i64, 0)
        .map_or_else(|| expires_at_unix.to_string(), |dt| dt.to_rfc3339());

    let body = format!(
        "⬆️ Corvus update available: v{latest_version} (current v{current_version})\n\
         Confirm from this same authorized sender with:\n\
         `{CONFIRM_COMMAND_PREFIX} {nonce}`\n\
         Nonce expires at {expires_iso}."
    );

    let result = channel.send(&SendMessage::new(body, recipient)).await;

    if let Err(error) = result {
        tracing::warn!(
            "update notification failed: channel={} recipient_len={} error_kind=send_failed",
            channel.name(),
            recipient.len(),
        );
        let _ = error;
        return false;
    }

    true
}

async fn execute_minimal_update_strategy(target_version: &str) -> UpdateExecutionResult {
    let package_managers: [(&str, [&str; 4]); 4] = [
        ("npm", ["i", "-g", "@dallay/corvus@latest", ""]),
        ("pnpm", ["add", "-g", "@dallay/corvus@latest", ""]),
        ("yarn", ["global", "add", "@dallay/corvus@latest", ""]),
        ("bun", ["add", "-g", "@dallay/corvus@latest", ""]),
    ];

    for (bin, args) in package_managers {
        let filtered_args = args
            .into_iter()
            .filter(|arg| !arg.is_empty())
            .collect::<Vec<_>>();

        let output = match Command::new(bin).args(&filtered_args).spawn() {
            Err(_) => continue,
            Ok(child) => {
                // Hold the PID before consuming child so we can signal on timeout.
                let child_id = child.id();
                match tokio::time::timeout(Duration::from_secs(60), child.wait_with_output()).await
                {
                    Ok(Ok(out)) => out,
                    Ok(Err(_)) => continue,
                    Err(_timeout) => {
                        tracing::warn!("update command timed out: manager={bin}");
                        // Best-effort kill by PID — child is already consumed by
                        // wait_with_output, so we use the OS directly.
                        if let Some(pid) = child_id {
                            let _ = std::process::Command::new("kill")
                                .arg(pid.to_string())
                                .status();
                        }
                        continue;
                    }
                }
            }
        };

        if output.status.success() {
            return UpdateExecutionResult {
                summary: format!(
                    "✅ Update command executed with `{}`. Please restart daemon/service to run v{}.",
                    std::iter::once(bin)
                        .chain(filtered_args.iter().copied())
                        .collect::<Vec<_>>()
                        .join(" "),
                    target_version
                ),
                succeeded: true,
                reason_code: None,
            };
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "update command failed: manager={} stderr_bytes={}",
            bin,
            stderr.len()
        );
    }

    UpdateExecutionResult {
        summary: format!(
            "⚠️ Update confirmation accepted for v{target_version}, but automatic installation is not available in this runtime. Run one of:\n\
             - npm i -g {PACKAGE_NAME}@latest\n\
             - curl -fsSL {INSTALL_SCRIPT_URL} | bash\n\
             Then restart the daemon/service."
        ),
        succeeded: false,
        reason_code: Some("no_supported_runtime_installer".to_string()),
    }
}

async fn check_for_update(config: &Config, current_version: &str) -> Option<UpdateNotice> {
    let current = normalize_version(current_version)?;
    let state_path = version_check_path(&config.workspace_dir);
    let cached_state = load_state(&state_path).await.ok().flatten();

    if let Some(cached) = cached_state.as_ref().filter(|state| !is_stale(state)) {
        return notice_from_state(current.clone(), cached);
    }

    let fetched = fetch_latest_release_version().await.ok();

    if let Some(latest_version) = fetched {
        let update_available =
            compare_semverish(&latest_version, &current).is_some_and(|ordering| ordering.is_gt());
        let state = VersionCheckState {
            latest_version,
            checked_at_unix: now_unix_secs(),
            update_available,
            last_notified_version: cached_state
                .as_ref()
                .and_then(|state| state.last_notified_version.clone()),
            pending_confirmations: cached_state
                .as_ref()
                .map_or_else(Vec::new, |state| state.pending_confirmations.clone()),
            notified_conversations: cached_state
                .as_ref()
                .map_or_else(Vec::new, |state| state.notified_conversations.clone()),
        };

        let _ = save_state(&state_path, &state).await;
        return notice_from_state(current, &state);
    }

    cached_state
        .as_ref()
        .and_then(|state| notice_from_state(current, state))
}

fn notice_from_state(current_version: String, state: &VersionCheckState) -> Option<UpdateNotice> {
    if !state.update_available {
        return None;
    }

    if compare_semverish(&state.latest_version, &current_version)
        .is_some_and(|ordering| ordering.is_gt())
    {
        Some(UpdateNotice {
            current_version,
            latest_version: state.latest_version.clone(),
        })
    } else {
        None
    }
}

pub(crate) fn is_update_check_disabled() -> bool {
    std::env::var(UPDATE_CHECK_DISABLE_ENV)
        .ok()
        .is_some_and(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

fn version_check_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(VERSION_CHECK_FILE)
}

fn update_state_lock_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(UPDATE_STATE_LOCK_FILE)
}

fn update_install_lock_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(UPDATE_INSTALL_LOCK_FILE)
}

fn update_history_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(UPDATE_HISTORY_FILE)
}

struct FileLockGuard {
    path: PathBuf,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_file_lock(path: &Path, timeout_ms: u64) -> Result<FileLockGuard> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create lock parent directory {}",
                parent.display()
            )
        })?;
    }

    let started = std::time::Instant::now();
    loop {
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(mut file) => {
                file.write_all(std::process::id().to_string().as_bytes())?;
                file.sync_all()?;
                return Ok(FileLockGuard {
                    path: path.to_path_buf(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if started.elapsed() >= Duration::from_millis(timeout_ms) {
                    anyhow::bail!("lock busy: {}", path.display());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to acquire lock {}", path.display()));
            }
        }
    }
}

fn resolve_install_method(
    override_method: Option<InstallMethod>,
    detected_method: Option<InstallMethod>,
) -> (InstallMethod, Option<InstallMethod>, &'static str) {
    if let Some(method) = override_method {
        return (method.clone(), Some(method), "override");
    }
    if let Some(method) = detected_method {
        return (method, None, "detected");
    }
    (InstallMethod::Unknown, None, "unknown")
}

#[derive(Debug, Clone)]
struct InstallDetectionContext {
    current_exe: Option<PathBuf>,
    npm_user_agent: Option<String>,
    cargo_home: Option<PathBuf>,
    home_dir: Option<PathBuf>,
}

impl InstallDetectionContext {
    fn from_runtime() -> Self {
        Self {
            current_exe: std::env::current_exe().ok(),
            npm_user_agent: std::env::var("npm_config_user_agent").ok(),
            cargo_home: std::env::var_os("CARGO_HOME").map(PathBuf::from),
            home_dir: std::env::var_os("HOME").map(PathBuf::from),
        }
    }
}

fn detect_install_method() -> Option<InstallMethod> {
    if let Ok(test_override) = std::env::var("CORVUS_TEST_INSTALL_METHOD") {
        return test_override.parse::<InstallMethod>().ok();
    }

    let context = InstallDetectionContext::from_runtime();
    detect_install_method_with_context(&context)
}

fn detect_install_method_with_context(context: &InstallDetectionContext) -> Option<InstallMethod> {
    if let Some(method) = detect_install_method_from_user_agent(context.npm_user_agent.as_deref()) {
        return Some(method);
    }

    let mut candidates = Vec::new();
    if let Some(exe) = context.current_exe.as_ref() {
        candidates.push(exe.clone());
        if let Ok(target) = fs::read_link(exe) {
            let resolved = if target.is_absolute() {
                target
            } else {
                exe.parent()
                    .map_or(target.clone(), |parent| parent.join(target))
            };
            candidates.push(resolved);
        }
    }

    for candidate in &candidates {
        if let Some(method) = detect_install_method_from_path(candidate, context) {
            return Some(method);
        }
    }

    None
}

fn detect_install_method_from_user_agent(user_agent: Option<&str>) -> Option<InstallMethod> {
    let normalized = user_agent?.trim().to_ascii_lowercase();
    if normalized.starts_with("pnpm/") {
        return Some(InstallMethod::Pnpm);
    }
    if normalized.starts_with("yarn/") {
        return Some(InstallMethod::Yarn);
    }
    if normalized.starts_with("bun/") {
        return Some(InstallMethod::Bun);
    }
    if normalized.starts_with("npm/") {
        return Some(InstallMethod::Npm);
    }
    None
}

fn detect_install_method_from_path(
    executable_path: &Path,
    context: &InstallDetectionContext,
) -> Option<InstallMethod> {
    let normalized = executable_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    if normalized.contains("/cellar/") || normalized.contains("/homebrew/") {
        return Some(InstallMethod::Homebrew);
    }

    if is_cargo_install_path(executable_path, context) || normalized.contains("/.cargo/bin/") {
        return Some(InstallMethod::Cargo);
    }

    if normalized.contains("/.pnpm/")
        || normalized.contains("/pnpm/global/")
        || normalized.contains("/share/pnpm/")
    {
        return Some(InstallMethod::Pnpm);
    }

    if normalized.contains("/.yarn/") || normalized.contains("/yarn/global/") {
        return Some(InstallMethod::Yarn);
    }

    if normalized.contains("/.bun/") || normalized.contains("/bun/") {
        return Some(InstallMethod::Bun);
    }

    if normalized.contains("/node_modules/.bin/") || normalized.contains("/lib/node_modules/") {
        return Some(InstallMethod::Npm);
    }

    let stem = executable_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if stem == "corvus"
        && (normalized.contains("/usr/local/bin/")
            || normalized.contains("/usr/bin/")
            || normalized.contains("/opt/bin/")
            || normalized.contains("/opt/local/bin/")
            || normalized.ends_with("/corvus"))
    {
        return Some(InstallMethod::ScriptBinary);
    }

    None
}

fn is_cargo_install_path(path: &Path, context: &InstallDetectionContext) -> bool {
    if let Some(cargo_home) = context.cargo_home.as_ref() {
        let bin = cargo_home.join("bin");
        if path.starts_with(&bin) {
            return true;
        }
    }

    if let Some(home_dir) = context.home_dir.as_ref() {
        let default_cargo_bin = home_dir.join(".cargo").join("bin");
        if path.starts_with(default_cargo_bin) {
            return true;
        }
    }

    false
}

fn save_state_snapshot_sync(workspace_dir: &Path, snapshot: &UpdateStateSnapshot) -> Result<()> {
    let path = version_check_path(workspace_dir);
    let body =
        serde_json::to_vec_pretty(snapshot).context("failed to serialize update snapshot")?;
    atomic_write_sync(&path, &body)
}

fn atomic_write_sync(path: &Path, body: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let temp_path = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));

    let mut temp_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .with_context(|| format!("failed to create temp file {}", temp_path.display()))?;
    temp_file.write_all(body)?;
    temp_file.sync_all()?;
    drop(temp_file);

    fs::rename(&temp_path, path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))?;
    sync_directory_sync(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("state path missing parent"))?,
    )?;
    Ok(())
}

fn sync_directory_sync(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let directory = fs::File::open(path)
            .with_context(|| format!("failed to open directory {}", path.display()))?;
        directory.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn load_state_snapshot_sync(workspace_dir: &Path) -> Result<Option<UpdateStateSnapshot>> {
    let path = version_check_path(workspace_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read update snapshot at {}", path.display()))?;

    if let Ok(snapshot) = serde_json::from_str::<UpdateStateSnapshot>(&raw) {
        return Ok(Some(snapshot));
    }
    if let Ok(legacy) = serde_json::from_str::<VersionCheckState>(&raw) {
        return Ok(Some(legacy.into()));
    }
    anyhow::bail!("failed to parse update snapshot")
}

fn append_audit_event_sync(
    workspace_dir: &Path,
    policy: &UpdatePolicy,
    event: UpdateAuditEvent,
) -> Result<()> {
    let history_path = update_history_path(workspace_dir);
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut events = read_update_history_sync(workspace_dir).unwrap_or_default();
    events.push(event);
    let max_entries = policy.history_max_entries.max(1) as usize;
    if events.len() > max_entries {
        let drain = events.len() - max_entries;
        events.drain(0..drain);
    }
    let mut payload = Vec::new();
    for item in &events {
        payload.extend(serde_json::to_vec(item)?);
        payload.push(b'\n');
    }
    atomic_write_sync(&history_path, &payload)
}

fn read_update_history_sync(workspace_dir: &Path) -> Result<Vec<UpdateAuditEvent>> {
    let history_path = update_history_path(workspace_dir);
    if !history_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&history_path)?;
    let mut events = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<UpdateAuditEvent>(trimmed) {
            events.push(event);
        }
    }
    Ok(events)
}

fn verify_sha256_checksum(path: &Path, expected_hex: &str) -> Result<()> {
    let expected = expected_hex.trim().to_ascii_lowercase();
    if expected.is_empty() {
        anyhow::bail!("missing checksum metadata")
    }
    let bytes =
        fs::read(path).with_context(|| format!("failed to read artifact {}", path.display()))?;
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != expected {
        anyhow::bail!("digest mismatch")
    }
    Ok(())
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn is_stale(state: &VersionCheckState) -> bool {
    now_unix_secs().saturating_sub(state.checked_at_unix) > VERSION_CHECK_TTL_SECS
}

async fn load_state(path: &Path) -> Result<Option<VersionCheckState>> {
    if !tokio::fs::try_exists(path)
        .await
        .with_context(|| format!("failed to check version check state at {}", path.display()))?
    {
        return Ok(None);
    }

    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read version check state at {}", path.display()))?;
    if let Ok(state) = serde_json::from_str::<VersionCheckState>(&raw) {
        return Ok(Some(state));
    }
    let snapshot = serde_json::from_str::<UpdateStateSnapshot>(&raw)
        .context("failed to parse version check state")?;
    Ok(Some(VersionCheckState {
        latest_version: snapshot.latest_version,
        checked_at_unix: snapshot.last_check_at_unix,
        update_available: snapshot.update_available,
        last_notified_version: None,
        pending_confirmations: snapshot.pending_confirmations,
        notified_conversations: snapshot.notified_conversations,
    }))
}

#[allow(clippy::unused_async)]
async fn save_state(path: &Path, state: &VersionCheckState) -> Result<()> {
    let body = serde_json::to_vec_pretty(state).context("failed to serialize version state")?;
    atomic_write_sync(path, &body)
}

async fn fetch_latest_release_version() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(VERSION_CHECK_TIMEOUT_SECS))
        .build()
        .context("failed to build update-check client")?;

    for endpoint in RELEASE_ENDPOINTS {
        let response = client
            .get(endpoint)
            .header(reqwest::header::USER_AGENT, "corvus-update-check")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await;

        let Ok(response) = response else {
            continue;
        };

        let Ok(response) = response.error_for_status() else {
            continue;
        };

        let payload: LatestReleaseResponse = response
            .json()
            .await
            .context("failed to parse release metadata")?;

        if let Some(normalized) = normalize_version(&payload.tag_name) {
            return Ok(normalized);
        }
    }

    anyhow::bail!("failed to resolve latest release version from release endpoints")
}

fn normalize_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
        .to_string();

    parse_semverish(&normalized).map(|_| normalized)
}

fn compare_semverish(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left_parsed = parse_semverish(left)?;
    let right_parsed = parse_semverish(right)?;

    let core_ordering = left_parsed
        .0
        .cmp(&right_parsed.0)
        .then(left_parsed.1.cmp(&right_parsed.1))
        .then(left_parsed.2.cmp(&right_parsed.2));
    if !core_ordering.is_eq() {
        return Some(core_ordering);
    }

    Some(compare_prerelease(
        left_parsed.3.as_deref(),
        right_parsed.3.as_deref(),
    ))
}

fn compare_prerelease(left: Option<&str>, right: Option<&str>) -> std::cmp::Ordering {
    match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => {
            let left_parts: Vec<&str> = left.split('.').collect();
            let right_parts: Vec<&str> = right.split('.').collect();

            for (l, r) in left_parts.iter().zip(right_parts.iter()) {
                let left_numeric = l.parse::<u64>();
                let right_numeric = r.parse::<u64>();

                let ordering = match (left_numeric, right_numeric) {
                    (Ok(a), Ok(b)) => a.cmp(&b),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => l.cmp(r),
                };

                if !ordering.is_eq() {
                    return ordering;
                }
            }

            left_parts.len().cmp(&right_parts.len())
        }
    }
}

fn parse_semverish(version: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let version = version.trim();
    if version.is_empty() {
        return None;
    }

    let without_build = version.split_once('+').map_or(version, |(core, _)| core);
    let (core, prerelease_raw) = without_build
        .split_once('-')
        .map_or((without_build, None), |(core, prerelease)| {
            (core, Some(prerelease.trim()))
        });

    let core = core.trim();
    if core.is_empty() {
        return None;
    }

    let mut parts = core.split('.');

    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    let prerelease = prerelease_raw
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    Some((major, minor, patch, prerelease))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn normalize_version_accepts_with_or_without_v_prefix() {
        assert_eq!(normalize_version("v0.1.7"), Some("0.1.7".to_string()));
        assert_eq!(normalize_version("0.1.7"), Some("0.1.7".to_string()));
        assert_eq!(normalize_version("V1.2.3"), Some("1.2.3".to_string()));
    }

    #[test]
    fn normalize_version_rejects_invalid_values() {
        assert_eq!(normalize_version("latest"), None);
        assert_eq!(normalize_version("v1"), None);
        assert_eq!(normalize_version(""), None);
    }

    #[test]
    fn compare_semverish_orders_versions() {
        assert_eq!(
            compare_semverish("0.1.8", "0.1.7"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_semverish("0.1.7", "0.1.7"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            compare_semverish("0.1.6", "0.1.7"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn parse_semverish_accepts_pre_release_patch_suffix() {
        assert_eq!(
            parse_semverish("1.2.3-beta.1"),
            Some((1, 2, 3, Some("beta.1".to_string())))
        );
    }

    #[test]
    fn compare_semverish_treats_prerelease_as_lower_precedence() {
        assert_eq!(
            compare_semverish("1.0.0", "1.0.0-beta.1"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_semverish("1.0.0-beta.1", "1.0.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn stale_cache_detection_works() {
        let now = now_unix_secs();
        let fresh = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now.saturating_sub(30),
            update_available: true,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        };
        let stale = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now.saturating_sub(VERSION_CHECK_TTL_SECS + 10),
            update_available: true,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        };

        assert!(!is_stale(&fresh));
        assert!(is_stale(&stale));
    }

    #[test]
    fn notice_requires_newer_latest_version() {
        let update = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now_unix_secs(),
            update_available: true,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        };
        let no_update_same = VersionCheckState {
            latest_version: "0.1.7".into(),
            checked_at_unix: now_unix_secs(),
            update_available: true,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        };
        let no_update_flag = VersionCheckState {
            latest_version: "0.1.8".into(),
            checked_at_unix: now_unix_secs(),
            update_available: false,
            last_notified_version: None,
            pending_confirmations: Vec::new(),
            notified_conversations: Vec::new(),
        };

        assert!(notice_from_state("0.1.7".into(), &update).is_some());
        assert!(notice_from_state("0.1.7".into(), &no_update_same).is_none());
        assert!(notice_from_state("0.1.7".into(), &no_update_flag).is_none());
    }

    #[test]
    fn parse_confirmation_nonce_accepts_command() {
        assert_eq!(
            parse_confirmation_nonce("corvus update confirm abc123"),
            Some("abc123")
        );
        assert_eq!(
            parse_confirmation_nonce("  Corvus   Update   Confirm   xyz   "),
            Some("xyz")
        );
    }

    #[test]
    fn parse_confirmation_nonce_rejects_non_matching_command() {
        assert!(parse_confirmation_nonce("update confirm abc").is_none());
        assert!(parse_confirmation_nonce("corvus update confirm").is_none());
        assert!(parse_confirmation_nonce("corvus update confirm abc extra").is_none());
    }

    #[test]
    fn prune_pending_confirmations_removes_used_and_expired() {
        let now = now_unix_secs();
        let mut pending = vec![
            PendingConfirmation {
                version: "1.0.0".into(),
                channel: "telegram".into(),
                recipient: "1".into(),
                authorized_sender: Some("1".into()),
                nonce_hash: "a".into(),
                expires_at_unix: now + 30,
                used: false,
            },
            PendingConfirmation {
                version: "1.0.0".into(),
                channel: "telegram".into(),
                recipient: "2".into(),
                authorized_sender: Some("2".into()),
                nonce_hash: "b".into(),
                expires_at_unix: now.saturating_sub(1),
                used: false,
            },
            PendingConfirmation {
                version: "1.0.0".into(),
                channel: "telegram".into(),
                recipient: "3".into(),
                authorized_sender: Some("3".into()),
                nonce_hash: "c".into(),
                expires_at_unix: now + 30,
                used: true,
            },
        ];

        prune_pending_confirmations(&mut pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].recipient, "1");
    }

    #[test]
    fn collect_notification_targets_uses_configured_and_inferred_destinations() {
        let mut cfg = Config::default();
        cfg.updates.notify_destinations.insert(
            "slack".to_string(),
            vec!["C123".to_string(), "C123".to_string()],
        );
        cfg.channels_config.matrix = Some(config::schema::MatrixConfig {
            homeserver: "https://matrix.example".into(),
            access_token: "token".into(),
            room_id: "!room:matrix.example".into(),
            allowed_users: vec!["@alice:matrix.example".into()],
        });

        let targets = collect_notification_targets(&cfg);
        assert!(targets
            .iter()
            .any(|target| { target.channel == "slack" && target.recipient == "C123" }));
        assert!(targets.iter().any(|target| {
            target.channel == "matrix" && target.recipient == "!room:matrix.example"
        }));
    }

    #[test]
    fn sender_authorization_checks_allowlists() {
        let mut cfg = Config::default();
        cfg.channels_config.telegram = Some(config::schema::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec!["123".into()],
            stream_mode: config::StreamMode::default(),
            draft_update_interval_ms: 1000,
        });

        assert!(is_sender_authorized(&cfg, "telegram", "123"));
        assert!(!is_sender_authorized(&cfg, "telegram", "999"));
    }

    #[tokio::test]
    async fn save_and_load_state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state").join("version_check.json");
        let state = VersionCheckState {
            latest_version: "0.1.9".into(),
            checked_at_unix: 123,
            update_available: true,
            last_notified_version: Some("0.1.9".into()),
            pending_confirmations: vec![PendingConfirmation {
                version: "0.1.9".into(),
                channel: "telegram".into(),
                recipient: "123".into(),
                authorized_sender: Some("123".into()),
                nonce_hash: hash_nonce("nonce"),
                expires_at_unix: 999,
                used: false,
            }],
            notified_conversations: vec![NotifiedConversation {
                version: "0.1.9".into(),
                channel: "telegram".into(),
                recipient: "123".into(),
                authorized_sender: "123".into(),
            }],
        };

        save_state(&path, &state).await.unwrap();
        let loaded = load_state(&path).await.unwrap().unwrap();

        assert_eq!(loaded.latest_version, "0.1.9");
        assert_eq!(loaded.checked_at_unix, 123);
        assert!(loaded.update_available);
        assert_eq!(loaded.last_notified_version.as_deref(), Some("0.1.9"));
        assert_eq!(loaded.pending_confirmations.len(), 1);
        assert_eq!(loaded.notified_conversations.len(), 1);
    }

    #[derive(Default)]
    struct TestChannel {
        sent_messages: tokio::sync::Mutex<Vec<(String, String)>>,
    }

    #[async_trait::async_trait]
    impl Channel for TestChannel {
        fn name(&self) -> &str {
            "test-channel"
        }

        async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
            self.sent_messages
                .lock()
                .await
                .push((message.recipient.clone(), message.content.clone()));
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<crate::channels::traits::ChannelMessage>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn opportunistic_notice_uses_reply_target_and_dedupes() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = dir.path().to_path_buf();
        cfg.channels_config.telegram = Some(config::schema::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec!["123".into()],
            stream_mode: config::StreamMode::default(),
            draft_update_interval_ms: 1000,
        });

        let state_path = version_check_path(&cfg.workspace_dir);
        save_state(
            &state_path,
            &VersionCheckState {
                latest_version: "9.9.9".into(),
                checked_at_unix: now_unix_secs(),
                update_available: true,
                last_notified_version: None,
                pending_confirmations: Vec::new(),
                notified_conversations: Vec::new(),
            },
        )
        .await
        .unwrap();

        let channel_impl = Arc::new(TestChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let msg = ChannelMessage {
            id: "msg-1".into(),
            sender: "123".into(),
            reply_target: "chat-1".into(),
            content: "hi".into(),
            channel: "telegram".into(),
            timestamp: 1,
        };

        let first =
            maybe_send_opportunistic_update_notice(&cfg, &msg, Some(&channel), "1.0.0").await;
        let second =
            maybe_send_opportunistic_update_notice(&cfg, &msg, Some(&channel), "1.0.0").await;

        assert!(first.unwrap());
        assert!(!second.unwrap());

        let sent = channel_impl.sent_messages.lock().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "chat-1");
        assert!(sent[0].1.contains("Corvus update available"));
        assert!(sent[0].1.contains("corvus update confirm"));

        let state = load_state(&state_path).await.unwrap().unwrap();
        assert_eq!(state.pending_confirmations.len(), 1);
        assert_eq!(state.notified_conversations.len(), 1);
        assert_eq!(state.pending_confirmations[0].recipient, "chat-1");
        assert_eq!(
            state.pending_confirmations[0].authorized_sender.as_deref(),
            Some("123")
        );
    }

    #[tokio::test]
    async fn opportunistic_notice_skips_when_no_update_available() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = dir.path().to_path_buf();
        cfg.channels_config.telegram = Some(config::schema::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec!["123".into()],
            stream_mode: config::StreamMode::default(),
            draft_update_interval_ms: 1000,
        });

        let state_path = version_check_path(&cfg.workspace_dir);
        save_state(
            &state_path,
            &VersionCheckState {
                latest_version: "1.0.0".into(),
                checked_at_unix: now_unix_secs(),
                update_available: false,
                last_notified_version: None,
                pending_confirmations: Vec::new(),
                notified_conversations: Vec::new(),
            },
        )
        .await
        .unwrap();

        let channel_impl = Arc::new(TestChannel::default());
        let channel: Arc<dyn Channel> = channel_impl.clone();
        let msg = ChannelMessage {
            id: "msg-1".into(),
            sender: "123".into(),
            reply_target: "chat-1".into(),
            content: "hi".into(),
            channel: "telegram".into(),
            timestamp: 1,
        };

        let sent =
            maybe_send_opportunistic_update_notice(&cfg, &msg, Some(&channel), "1.0.0").await;

        assert!(!sent.unwrap());
        assert!(channel_impl.sent_messages.lock().await.is_empty());
    }

    #[test]
    fn install_method_resolution_prefers_override_then_detected_then_unknown() {
        let (effective, overridden, source) =
            resolve_install_method(Some(InstallMethod::Cargo), Some(InstallMethod::Npm));
        assert_eq!(effective, InstallMethod::Cargo);
        assert_eq!(overridden, Some(InstallMethod::Cargo));
        assert_eq!(source, "override");

        let (effective, overridden, source) =
            resolve_install_method(None, Some(InstallMethod::Npm));
        assert_eq!(effective, InstallMethod::Npm);
        assert_eq!(overridden, None);
        assert_eq!(source, "detected");

        let (effective, overridden, source) = resolve_install_method(None, None);
        assert_eq!(effective, InstallMethod::Unknown);
        assert_eq!(overridden, None);
        assert_eq!(source, "unknown");
    }

    #[test]
    fn install_method_detection_matrix_covers_supported_runtime_patterns() {
        let context = InstallDetectionContext {
            current_exe: None,
            npm_user_agent: Some("pnpm/9.0.0 npm/? node/?".to_string()),
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Pnpm)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from(
                "/opt/homebrew/Cellar/corvus/1.2.3/bin/corvus",
            )),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Homebrew)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from("/Users/dev/.cargo/bin/corvus")),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: Some(PathBuf::from("/Users/dev")),
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Cargo)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from("/Users/dev/.bun/bin/corvus")),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Bun)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from(
                "/Users/dev/.local/share/pnpm/global/5/node_modules/.bin/corvus",
            )),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Pnpm)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from("/Users/dev/.yarn/bin/corvus")),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Yarn)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from(
                "/usr/local/lib/node_modules/@dallay/corvus/bin/corvus.js",
            )),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::Npm)
        );

        let context = InstallDetectionContext {
            current_exe: Some(PathBuf::from("/usr/local/bin/corvus")),
            npm_user_agent: None,
            cargo_home: None,
            home_dir: None,
        };
        assert_eq!(
            detect_install_method_with_context(&context),
            Some(InstallMethod::ScriptBinary)
        );
    }

    #[test]
    fn consume_pending_confirmation_honors_scope_and_marks_nonce_used() {
        let nonce = "nonce-abc";
        let mut state = VersionCheckState {
            latest_version: "1.2.3".to_string(),
            checked_at_unix: now_unix_secs(),
            update_available: true,
            last_notified_version: None,
            pending_confirmations: vec![PendingConfirmation {
                version: "1.2.3".to_string(),
                channel: "telegram".to_string(),
                recipient: "chat-1".to_string(),
                authorized_sender: Some("sender-1".to_string()),
                nonce_hash: hash_nonce(nonce),
                expires_at_unix: now_unix_secs() + 60,
                used: false,
            }],
            notified_conversations: Vec::new(),
        };

        let version = consume_pending_confirmation(
            &mut state,
            nonce,
            Some(("telegram", "chat-1", "sender-1")),
        )
        .unwrap();
        assert_eq!(version, "1.2.3");
        assert!(state.pending_confirmations[0].used);
    }

    #[test]
    fn update_manager_install_returns_busy_when_install_lock_held() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = dir.path().to_path_buf();

        let lock_path = update_install_lock_path(&cfg.workspace_dir);
        let _held = acquire_file_lock(&lock_path, 10).unwrap();

        let manager = UpdateManager::new(&cfg);
        let (outcome, message) = manager.install("1.0.0", "test").unwrap();
        assert_eq!(outcome, InstallCommandOutcome::Busy);
        assert!(message.contains("busy"));
    }

    #[test]
    fn load_snapshot_ignores_partial_temp_file_and_keeps_valid_state() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();
        let snapshot = UpdateStateSnapshot::initial(
            "1.0.0",
            &UpdatePolicy {
                checks_enabled: true,
                auto_install_enabled: false,
                channel_visibility_enabled: true,
                cli_startup_notice_enabled: true,
                check_interval_minutes: 30,
                confirmation_ttl_minutes: 30,
                install_method_override: None,
                restart_policy: RestartPolicy::Prompt,
                history_max_entries: 200,
            },
        );
        save_state_snapshot_sync(workspace, &snapshot).unwrap();

        let temp = version_check_path(workspace).with_extension("tmp.partial");
        if let Some(parent) = temp.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&temp, "{\"broken\":").unwrap();

        let loaded = load_state_snapshot_sync(workspace).unwrap().unwrap();
        assert_eq!(loaded.current_version, snapshot.current_version);
    }

    #[test]
    fn verification_fails_closed_on_mismatch_and_audit_history_records_event() {
        let dir = tempfile::tempdir().unwrap();
        let artifact_path = dir.path().join("artifact.bin");
        fs::write(&artifact_path, b"v1").unwrap();

        let err = verify_sha256_checksum(&artifact_path, "deadbeef").unwrap_err();
        assert!(err.to_string().contains("digest mismatch"));

        let policy = UpdatePolicy {
            checks_enabled: true,
            auto_install_enabled: false,
            channel_visibility_enabled: true,
            cli_startup_notice_enabled: true,
            check_interval_minutes: 30,
            confirmation_ttl_minutes: 30,
            install_method_override: None,
            restart_policy: RestartPolicy::Prompt,
            history_max_entries: 10,
        };
        append_audit_event_sync(
            dir.path(),
            &policy,
            UpdateAuditEvent {
                event_id: "event-1".to_string(),
                timestamp_unix: 1,
                action: "verification".to_string(),
                outcome: "failed".to_string(),
                current_version: "1.0.0".to_string(),
                target_version: Some("1.0.1".to_string()),
                effective_method: "script_binary".to_string(),
                actor: "test".to_string(),
                reason_code: Some("digest mismatch".to_string()),
            },
        )
        .unwrap();

        let history = read_update_history_sync(dir.path()).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].action, "verification");
        assert_eq!(history[0].outcome, "failed");
    }

    #[test]
    fn verification_success_allows_activation_and_records_success_audit_events() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = dir.path().to_path_buf();
        cfg.updates.install_method_override = Some("script_binary".to_string());

        let policy = UpdatePolicy::from_config(&cfg);
        let mut snapshot = UpdateStateSnapshot::initial("1.0.0", &policy);
        snapshot.latest_version = "1.0.1".to_string();
        snapshot.update_available = true;
        save_state_snapshot_sync(&cfg.workspace_dir, &snapshot).unwrap();

        let artifact_path = dir.path().join("artifact-ok.bin");
        fs::write(&artifact_path, b"verified-artifact").unwrap();
        let digest = Sha256::digest(b"verified-artifact");
        let expected_sha = hex::encode(digest);

        unsafe {
            std::env::set_var("CORVUS_UPDATE_ARTIFACT_PATH", artifact_path.as_os_str());
            std::env::set_var("CORVUS_UPDATE_EXPECTED_SHA256", &expected_sha);
        }

        let manager = UpdateManager::new(&cfg);
        let (outcome, message) = manager
            .install("1.0.0", "test-verification-success")
            .unwrap();

        unsafe {
            std::env::remove_var("CORVUS_UPDATE_ARTIFACT_PATH");
            std::env::remove_var("CORVUS_UPDATE_EXPECTED_SHA256");
        }

        assert_eq!(outcome, InstallCommandOutcome::Success);
        assert!(message.contains("update installed"));

        let loaded = load_state_snapshot_sync(&cfg.workspace_dir)
            .unwrap()
            .unwrap();
        assert!(matches!(
            loaded.install_state,
            InstallState::InstalledPendingRestart { .. }
        ));

        let history = read_update_history_sync(&cfg.workspace_dir).unwrap();
        assert!(history
            .iter()
            .any(|event| event.action == "verification" && event.outcome == "success"));
        assert!(history
            .iter()
            .any(|event| event.action == "install" && event.outcome == "success"));
    }
}
