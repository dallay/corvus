use crate::config::Config;
use anyhow::Result;
use chrono::Utc;
use std::future::Future;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const STATUS_FLUSH_SECONDS: u64 = 5;

pub async fn run(config: Config, host: String, port: u16) -> Result<()> {
    let initial_backoff = config.reliability.channel_initial_backoff_secs.max(1);
    let max_backoff = config
        .reliability
        .channel_max_backoff_secs
        .max(initial_backoff);

    crate::health::mark_component_ok("daemon");

    if config.heartbeat.enabled {
        let _ =
            crate::heartbeat::engine::HeartbeatEngine::ensure_heartbeat_file(&config.workspace_dir)
                .await;
    }

    let mut handles: Vec<JoinHandle<()>> = vec![spawn_state_writer(config.clone())];

    {
        let gateway_cfg = config.clone();
        let gateway_host = host.clone();
        handles.push(spawn_component_supervisor(
            "gateway",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = gateway_cfg.clone();
                let host = gateway_host.clone();
                async move { crate::gateway::run_gateway(&host, port, cfg).await }
            },
        ));
    }

    {
        if has_supervised_channels(&config) {
            let channels_cfg = config.clone();
            handles.push(spawn_component_supervisor(
                "channels",
                initial_backoff,
                max_backoff,
                move || {
                    let cfg = channels_cfg.clone();
                    async move { crate::channels::start_channels(cfg).await }
                },
            ));
        } else {
            crate::health::mark_component_ok("channels");
            tracing::info!("No real-time channels configured; channel supervisor disabled");
        }
    }

    if config.heartbeat.enabled {
        let heartbeat_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "heartbeat",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = heartbeat_cfg.clone();
                Box::pin(run_heartbeat_worker(cfg))
            },
        ));
    }

    if config.cron.enabled {
        let scheduler_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "scheduler",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = scheduler_cfg.clone();
                async move { crate::cron::scheduler::run(cfg).await }
            },
        ));
    } else {
        crate::health::mark_component_ok("scheduler");
        tracing::info!("Cron disabled; scheduler supervisor not started");
    }

    let mission_scheduler_started = mission_checkpoint_supervision_enabled(&config);
    if mission_scheduler_started {
        let mission_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "mission-checkpoints",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = mission_cfg.clone();
                async move { run_mission_checkpoint_worker(cfg).await }
            },
        ));
    } else {
        crate::health::mark_component_ok("mission-checkpoints");
        tracing::info!("Mission mode disabled; mission checkpoint supervisor not started");
    }

    let updater_started = updater_supervision_enabled(&config);
    if updater_started {
        let update_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "updater",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = update_cfg.clone();
                async move { run_daemon_updater_component(cfg).await }
            },
        ));
    } else {
        crate::health::mark_component_ok("updater");
        tracing::info!("Update daemon watcher disabled; updater supervisor not started");
    }

    let mut component_list = vec!["gateway", "channels", "heartbeat", "scheduler"];
    if mission_scheduler_started {
        component_list.push("mission-checkpoints");
    }
    if updater_started {
        component_list.push("updater");
    }

    println!("🧠 Corvus daemon started");
    println!("   Gateway:  http://{host}:{port}");
    println!("   Components: {}", component_list.join(", "));
    println!("   Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;
    crate::health::mark_component_error("daemon", "shutdown requested");

    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

pub fn state_file_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json")
}

fn spawn_state_writer(config: Config) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = state_file_path(&config);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut interval = tokio::time::interval(Duration::from_secs(STATUS_FLUSH_SECONDS));
        loop {
            interval.tick().await;
            let mut json = crate::health::snapshot_json();
            if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "written_at".into(),
                    serde_json::json!(Utc::now().to_rfc3339()),
                );
            }
            let data = serde_json::to_vec_pretty(&json).unwrap_or_else(|_| b"{}".to_vec());
            let _ = tokio::fs::write(&path, data).await;
        }
    })
}

fn spawn_component_supervisor<F, Fut>(
    name: &'static str,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
    mut run_component: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);
        let mut gateway_port_conflict_logged = false;

        loop {
            crate::health::mark_component_ok(name);
            match run_component().await {
                Ok(()) => {
                    crate::health::mark_component_error(name, "component exited unexpectedly");
                    tracing::warn!("Daemon component '{name}' exited unexpectedly");
                    // Clean exit — reset backoff since the component ran successfully
                    backoff = initial_backoff_secs.max(1);
                }
                Err(e) => {
                    let is_gateway_addr_in_use = name == "gateway" && is_addr_in_use_error(&e);
                    let err_str = e.to_string();
                    crate::health::mark_component_error(name, &err_str);
                    tracing::error!("Daemon component '{name}' failed: {err_str}");
                    if is_gateway_addr_in_use && !gateway_port_conflict_logged {
                        gateway_port_conflict_logged = true;
                        tracing::warn!(
                            "Gateway port is already in use. This usually means another daemon/gateway instance is already running. If this happened after an upgrade, run `corvus service restart` instead of starting a second daemon process."
                        );
                    }
                }
            }

            crate::health::bump_component_restart(name);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            // Double backoff AFTER sleeping so first error uses initial_backoff
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

fn is_addr_in_use_error(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .any(|io_error| io_error.kind() == std::io::ErrorKind::AddrInUse)
}

async fn run_heartbeat_worker(config: Config) -> Result<()> {
    let observer: std::sync::Arc<dyn crate::observability::Observer> =
        std::sync::Arc::from(crate::observability::create_observer(&config.observability));
    let engine = crate::heartbeat::engine::HeartbeatEngine::new(
        config.heartbeat.clone(),
        config.workspace_dir.clone(),
        observer,
    );

    let interval_mins = config.heartbeat.interval_minutes.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(u64::from(interval_mins) * 60));

    loop {
        interval.tick().await;

        let tasks = engine.collect_tasks().await?;
        if tasks.is_empty() {
            continue;
        }

        for task in tasks {
            let prompt = format!("[Heartbeat Task] {task}");
            let temp = config.default_temperature;
            if let Err(e) =
                crate::agent::run(config.clone(), Some(prompt), None, None, temp, vec![]).await
            {
                crate::health::mark_component_error("heartbeat", e.to_string());
                tracing::warn!("Heartbeat task failed: {e}");
            } else {
                crate::health::mark_component_ok("heartbeat");
            }
        }
    }
}

async fn run_mission_checkpoint_worker(config: Config) -> Result<()> {
    let poll_seconds = config.reliability.scheduler_poll_secs.max(1);
    let mut interval = tokio::time::interval(Duration::from_secs(poll_seconds));

    loop {
        interval.tick().await;
        crate::health::mark_component_ok("mission-checkpoints");
    }
}

fn has_supervised_channels(config: &Config) -> bool {
    config.channels_config.telegram.is_some()
        || config.channels_config.discord.is_some()
        || config.channels_config.slack.is_some()
        || config.channels_config.imessage.is_some()
        || config.channels_config.matrix.is_some()
        || config.channels_config.signal.is_some()
        || config.channels_config.whatsapp.is_some()
        || config.channels_config.email.is_some()
        || config.channels_config.irc.is_some()
        || config.channels_config.lark.is_some()
        || config.channels_config.dingtalk.is_some()
}

fn mission_checkpoint_supervision_enabled(config: &Config) -> bool {
    config.mission.enabled
}

fn updater_supervision_enabled(config: &Config) -> bool {
    config.updates.enabled && !crate::update::is_update_check_disabled()
}

fn updater_check_interval(config: &Config) -> Duration {
    Duration::from_secs(config.updates.check_interval_minutes.max(1) * 60)
}

fn should_emit_update_notification(
    config: &Config,
    status: &crate::update::UpdateStatusView,
    last_notified_version: Option<&str>,
) -> bool {
    if !config.updates.enabled || !config.updates.channel_visibility_enabled {
        return false;
    }

    if !status.update_available {
        return false;
    }

    let Some(latest) = status.latest_version.as_deref() else {
        return false;
    };

    last_notified_version != Some(latest)
}

async fn run_daemon_updater_component(config: Config) -> Result<()> {
    if !config.updates.enabled || crate::update::is_update_check_disabled() {
        return Ok(());
    }

    let mut last_notified_version: Option<String> = None;
    let status = crate::update::run_update_check(&config, env!("CARGO_PKG_VERSION")).await?;
    if should_emit_update_notification(&config, &status, last_notified_version.as_deref()) {
        last_notified_version = status.latest_version.clone();
        tracing::info!(
            latest_version = ?last_notified_version,
            "daemon updater canonical status indicates update notification"
        );
    }

    let _interval = updater_check_interval(&config);
    crate::update::run_daemon_update_watcher(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct HealthComponentGuard {
        name: &'static str,
    }

    impl HealthComponentGuard {
        fn new(name: &'static str) -> Self {
            crate::health::clear_component(name);
            Self { name }
        }
    }

    impl Drop for HealthComponentGuard {
        fn drop(&mut self) {
            crate::health::clear_component(self.name);
        }
    }

    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    #[test]
    fn state_file_path_uses_config_directory() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let path = state_file_path(&config);
        assert_eq!(path, tmp.path().join("daemon_state.json"));
    }

    #[tokio::test]
    async fn supervisor_marks_error_and_restart_on_failure() {
        let handle = spawn_component_supervisor("daemon-test-fail", 1, 1, || async {
            anyhow::bail!("boom")
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-fail"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("boom"));
    }

    #[tokio::test]
    async fn supervisor_marks_unexpected_exit_as_error() {
        let handle = spawn_component_supervisor("daemon-test-exit", 1, 1, || async { Ok(()) });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-exit"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("component exited unexpectedly"));
    }

    #[tokio::test]
    async fn supervisor_logs_hint_on_gateway_addr_in_use() {
        let _health_guard = HealthComponentGuard::new("gateway");
        let handle = spawn_component_supervisor("gateway", 1, 1, || async {
            Err(anyhow::Error::new(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "Address already in use",
            )))
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["gateway"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("Address already in use"));
    }

    #[test]
    fn detects_no_supervised_channels() {
        let config = Config::default();
        assert!(!has_supervised_channels(&config));
    }

    #[test]
    fn detects_supervised_channels_present() {
        let mut config = Config::default();
        config.channels_config.telegram = Some(crate::config::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec![],
            stream_mode: crate::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn detects_dingtalk_as_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.dingtalk = Some(crate::config::schema::DingTalkConfig {
            client_id: "client_id".into(),
            client_secret: "client_secret".into(),
            allowed_users: vec!["*".into()],
        });
        assert!(has_supervised_channels(&config));
    }

    #[test]
    fn mission_checkpoint_supervision_follows_mission_enablement() {
        let mut config = Config::default();
        config.mission.enabled = false;
        assert!(!mission_checkpoint_supervision_enabled(&config));

        config.mission.enabled = true;
        assert!(mission_checkpoint_supervision_enabled(&config));
    }

    #[test]
    fn updater_supervision_follows_update_policy() {
        let mut config = Config::default();
        config.updates.enabled = true;
        assert!(updater_supervision_enabled(&config));

        config.updates.enabled = false;
        assert!(!updater_supervision_enabled(&config));
    }

    #[test]
    fn updater_interval_uses_configured_minutes_with_floor() {
        let mut config = Config::default();
        config.updates.check_interval_minutes = 30;
        assert_eq!(updater_check_interval(&config), Duration::from_secs(1800));

        config.updates.check_interval_minutes = 0;
        assert_eq!(updater_check_interval(&config), Duration::from_secs(60));
    }

    #[test]
    fn updater_notification_dedupes_by_latest_version() {
        let config = Config::default();
        let status = crate::update::UpdateStatusView {
            current_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            update_available: true,
            last_check_at_unix: Some(1),
            last_check_outcome: Some("success".to_string()),
            effective_install_method: "unknown".to_string(),
            detected_install_method: None,
            install_method_source: "unknown".to_string(),
            policy: crate::update::UpdatePolicyView {
                checks_enabled: true,
                auto_install_enabled: false,
                channel_visibility_enabled: true,
                cli_startup_notice_enabled: true,
                restart_policy: "prompt".to_string(),
            },
        };

        assert!(should_emit_update_notification(&config, &status, None));
        assert!(!should_emit_update_notification(
            &config,
            &status,
            Some("1.1.0"),
        ));
    }

    #[test]
    fn updater_notification_respects_visibility_policy() {
        let mut config = Config::default();
        config.updates.enabled = true;
        config.updates.channel_visibility_enabled = false;
        let status = crate::update::UpdateStatusView {
            current_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            update_available: true,
            last_check_at_unix: Some(1),
            last_check_outcome: Some("success".to_string()),
            effective_install_method: "unknown".to_string(),
            detected_install_method: None,
            install_method_source: "unknown".to_string(),
            policy: crate::update::UpdatePolicyView {
                checks_enabled: true,
                auto_install_enabled: false,
                channel_visibility_enabled: false,
                cli_startup_notice_enabled: true,
                restart_policy: "prompt".to_string(),
            },
        };

        assert!(!should_emit_update_notification(&config, &status, None));
    }

    #[tokio::test]
    async fn daemon_updater_component_exits_cleanly_when_updates_disabled() {
        let mut config = Config::default();
        config.updates.enabled = false;
        let result = run_daemon_updater_component(config).await;
        assert!(result.is_ok());
    }
}
