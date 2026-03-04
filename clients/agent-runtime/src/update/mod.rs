use crate::channels::traits::ChannelMessage;
use crate::channels::{
    Channel, DingTalkChannel, DiscordChannel, EmailChannel, IMessageChannel, IrcChannel,
    LarkChannel, MatrixChannel, QQChannel, SendMessage, SignalChannel, SlackChannel,
    TelegramChannel, WhatsAppChannel,
};
use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
const RELEASE_ENDPOINTS: [&str; 2] = [
    "https://api.github.com/repos/profiletailors/corvus/releases/latest",
    "https://api.github.com/repos/dallay/corvus/releases/latest",
];

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
}

pub async fn maybe_print_update_notice(config: &Config) {
    if is_update_check_disabled() {
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

    let nonce_hash = hash_nonce(raw_nonce);
    let Some(pending) = state.pending_confirmations.iter_mut().find(|pending| {
        !pending.used
            && pending.nonce_hash == nonce_hash
            && pending.channel.eq_ignore_ascii_case(&msg.channel)
            && pending.recipient.eq_ignore_ascii_case(&msg.reply_target)
            && pending.expires_at_unix > now_unix_secs()
            && pending
                .authorized_sender
                .as_ref()
                .is_none_or(|sender| sender == &msg.sender)
    }) else {
        let _ = channel
            .send(&SendMessage::new(
                "Invalid, expired, or already-used update confirmation nonce.",
                &msg.reply_target,
            ))
            .await;
        let _ = save_state(&state_path, &state).await;
        return true;
    };

    pending.used = true;
    let version = pending.version.clone();

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
    if is_update_check_disabled() || !config.updates.enabled {
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

fn build_channel(config: &Config, channel_name: &str) -> Option<Arc<dyn Channel>> {
    match channel_name {
        "telegram" => config.channels_config.telegram.as_ref().map(|cfg| {
            Arc::new(TelegramChannel::new(
                cfg.bot_token.clone(),
                cfg.allowed_users.clone(),
            )) as Arc<dyn Channel>
        }),
        "discord" => config.channels_config.discord.as_ref().map(|cfg| {
            Arc::new(DiscordChannel::new(
                cfg.bot_token.clone(),
                cfg.guild_id.clone(),
                cfg.allowed_users.clone(),
                cfg.listen_to_bots,
                cfg.mention_only,
            )) as Arc<dyn Channel>
        }),
        "slack" => config.channels_config.slack.as_ref().map(|cfg| {
            Arc::new(SlackChannel::new(
                cfg.bot_token.clone(),
                cfg.channel_id.clone(),
                cfg.allowed_users.clone(),
            )) as Arc<dyn Channel>
        }),
        "mattermost" => config.channels_config.mattermost.as_ref().map(|cfg| {
            Arc::new(crate::channels::MattermostChannel::new(
                cfg.url.clone(),
                cfg.bot_token.clone(),
                cfg.channel_id.clone(),
                cfg.allowed_users.clone(),
                cfg.thread_replies.unwrap_or(true),
            )) as Arc<dyn Channel>
        }),
        "imessage" => config.channels_config.imessage.as_ref().map(|cfg| {
            Arc::new(IMessageChannel::new(cfg.allowed_contacts.clone())) as Arc<dyn Channel>
        }),
        "matrix" => config.channels_config.matrix.as_ref().map(|cfg| {
            Arc::new(MatrixChannel::new(
                cfg.homeserver.clone(),
                cfg.access_token.clone(),
                cfg.room_id.clone(),
                cfg.allowed_users.clone(),
            )) as Arc<dyn Channel>
        }),
        "signal" => config.channels_config.signal.as_ref().map(|cfg| {
            Arc::new(SignalChannel::new(
                cfg.http_url.clone(),
                cfg.account.clone(),
                cfg.group_id.clone(),
                cfg.allowed_from.clone(),
                cfg.ignore_attachments,
                cfg.ignore_stories,
            )) as Arc<dyn Channel>
        }),
        "whatsapp" => config.channels_config.whatsapp.as_ref().map(|cfg| {
            Arc::new(WhatsAppChannel::new(
                cfg.access_token.clone(),
                cfg.phone_number_id.clone(),
                cfg.verify_token.clone(),
                cfg.allowed_numbers.clone(),
            )) as Arc<dyn Channel>
        }),
        "email" => config
            .channels_config
            .email
            .as_ref()
            .map(|cfg| Arc::new(EmailChannel::new(cfg.clone())) as Arc<dyn Channel>),
        "irc" => config.channels_config.irc.as_ref().map(|cfg| {
            Arc::new(IrcChannel::new(crate::channels::irc::IrcChannelConfig {
                server: cfg.server.clone(),
                port: cfg.port,
                nickname: cfg.nickname.clone(),
                username: cfg.username.clone(),
                channels: cfg.channels.clone(),
                allowed_users: cfg.allowed_users.clone(),
                server_password: cfg.server_password.clone(),
                nickserv_password: cfg.nickserv_password.clone(),
                sasl_password: cfg.sasl_password.clone(),
                verify_tls: cfg.verify_tls.unwrap_or(true),
            })) as Arc<dyn Channel>
        }),
        "lark" => config
            .channels_config
            .lark
            .as_ref()
            .map(|cfg| Arc::new(LarkChannel::from_config(cfg)) as Arc<dyn Channel>),
        "dingtalk" => config.channels_config.dingtalk.as_ref().map(|cfg| {
            Arc::new(DingTalkChannel::new(
                cfg.client_id.clone(),
                cfg.client_secret.clone(),
                cfg.allowed_users.clone(),
            )) as Arc<dyn Channel>
        }),
        "qq" => config.channels_config.qq.as_ref().map(|cfg| {
            Arc::new(QQChannel::new(
                cfg.app_id.clone(),
                cfg.app_secret.clone(),
                cfg.allowed_users.clone(),
            )) as Arc<dyn Channel>
        }),
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
    let Some(channel) = build_channel(config, &target.channel) else {
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
    let state = serde_json::from_str::<VersionCheckState>(&raw)
        .context("failed to parse version check state")?;
    Ok(Some(state))
}

async fn save_state(path: &Path, state: &VersionCheckState) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "failed to create version check state directory {}",
                parent.display()
            )
        })?;
    }

    let body = serde_json::to_vec_pretty(state).context("failed to serialize version state")?;
    tokio::fs::write(path, body)
        .await
        .with_context(|| format!("failed to write version check state at {}", path.display()))
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
}
