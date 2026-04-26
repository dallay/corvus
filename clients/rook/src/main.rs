//! Rook — local-first AI provider gateway.
//!
//! Entry point. Parses subcommands and dispatches to the appropriate module.

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use rook::config::{discover_default_config_path_from_env, RookConfig, RookConfigExportView};
use rook::doctor;
use rook::registry::RookRegistry;
use rook::server::ServerConfig;

const DEFAULT_ROOK_DB_PATH: &str = "./rook.db";

#[derive(Parser)]
#[command(
    name = "rook",
    version,
    about = "Corvus Rook — local-first AI provider gateway",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the OpenAI-compatible HTTP gateway and embedded dashboard
    Serve {
        /// Host address to bind to
        #[arg(long)]
        host: Option<String>,

        /// TCP port to listen on
        #[arg(long)]
        port: Option<u16>,

        /// Enable the operator TUI alongside the HTTP server
        #[arg(long, action = ArgAction::SetTrue)]
        tui: bool,

        /// Path to the on-disk SQLite database file
        #[arg(long)]
        db_path: Option<String>,

        /// Enable inbound bearer auth for `/api/*` and `/v1/*`
        #[arg(long, action = ArgAction::SetTrue)]
        inbound_auth_enabled: bool,

        /// Static inbound bearer token for protected HTTP entrypoints
        #[arg(long)]
        inbound_auth_token: Option<String>,

        /// Global max requests allowed per window for `/api/*`
        #[arg(long)]
        api_rate_limit_max_requests: Option<u32>,

        /// Window size in seconds for `/api/*` global rate limiting
        #[arg(long)]
        api_rate_limit_window_seconds: Option<u64>,

        /// Global max requests allowed per window for `GET /v1/models`
        #[arg(long)]
        models_rate_limit_max_requests: Option<u32>,

        /// Window size in seconds for `GET /v1/models` global rate limiting
        #[arg(long)]
        models_rate_limit_window_seconds: Option<u64>,

        /// Global max requests allowed per window for `POST /v1/chat/completions`
        #[arg(long)]
        chat_rate_limit_max_requests: Option<u32>,

        /// Window size in seconds for `POST /v1/chat/completions` global rate limiting
        #[arg(long)]
        chat_rate_limit_window_seconds: Option<u64>,

        /// Replay window in seconds for keyed `POST /v1/chat/completions` idempotency
        #[arg(long)]
        chat_idempotency_replay_window_seconds: Option<u64>,
    },
    /// Launch the operator TUI
    Tui,
    /// Run diagnostics and check configuration
    Doctor,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Export current configuration to stdout
    Export,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    run_cli(cli).await
}

async fn run_cli(cli: Cli) -> Result<()> {
    run_cli_with_tui_runner_and_output(cli, |registry, dashboard_url| async move {
        rook::tui::run_standalone(registry, dashboard_url).await
    }, |line| {
        println!("{line}");
        Ok(())
    })
    .await
}

async fn run_cli_with_tui_runner_and_output<F, Fut, O>(cli: Cli, tui_runner: F, output: O) -> Result<()>
where
    F: Fn(RookRegistry, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), rook::domain::RookError>>,
    O: Fn(String) -> Result<()>,
{
    let env = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    run_cli_with_tui_runner_output_and_env(cli, tui_runner, output, &env).await
}

async fn run_cli_with_tui_runner_output_and_env<F, Fut, O>(
    cli: Cli,
    tui_runner: F,
    output: O,
    env: &std::collections::HashMap<String, String>,
) -> Result<()>
where
    F: Fn(RookRegistry, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), rook::domain::RookError>>,
    O: Fn(String) -> Result<()>,
{
    match cli.command {
        Commands::Serve {
            host,
            port,
            tui,
            db_path,
            inbound_auth_enabled,
            inbound_auth_token,
            api_rate_limit_max_requests,
            api_rate_limit_window_seconds,
            models_rate_limit_max_requests,
            models_rate_limit_window_seconds,
            chat_rate_limit_max_requests,
            chat_rate_limit_window_seconds,
            chat_idempotency_replay_window_seconds,
        } => {
            let config_path = discover_default_config_path_from_env();
            let config = build_serve_config(
                ServeOverrides {
                    host,
                    port,
                    enable_tui: tui,
                    db_path,
                    inbound_auth_enabled,
                    inbound_auth_token,
                    api_rate_limit_max_requests,
                    api_rate_limit_window_seconds,
                    models_rate_limit_max_requests,
                    models_rate_limit_window_seconds,
                    chat_rate_limit_max_requests,
                    chat_rate_limit_window_seconds,
                    chat_idempotency_replay_window_seconds,
                },
                config_path.as_deref(),
                &env,
            )?;
            rook::server::run(config).await?;
        }
        Commands::Tui => {
            launch_tui_with_runner(
                DEFAULT_ROOK_DB_PATH,
                "http://127.0.0.1:4141".to_string(),
                tui_runner,
            )
            .await?;
        }
        Commands::Doctor => {
            let config_path = discover_default_config_path_from_env();
            let report = doctor::run_with_config_path(config_path.as_deref(), env).await;
            output(doctor::render_report(&report))?;
            doctor::ensure_success(&report)?;
        }
        Commands::Config {
            action: ConfigCommands::Export,
        } => {
            let config_path = discover_default_config_path_from_env();
            let config = build_export_config_from_path(config_path.as_deref(), env)?;
            output(render_config_export(&config)?)?;
        }
    }

    Ok(())
}

fn build_export_config_from_path(
    file_path: Option<&std::path::Path>,
    env: &std::collections::HashMap<String, String>,
) -> Result<RookConfig> {
    Ok(RookConfig::from_sources_with_path(file_path, env)?)
}

fn build_serve_config(
    input: ServeOverrides,
    file_path: Option<&std::path::Path>,
    env: &std::collections::HashMap<String, String>,
) -> Result<ServerConfig> {
    let mut config = RookConfig::from_sources_with_path(file_path, env)?;

    if let Some(host) = input.host {
        config.host = host;
    }
    if let Some(port) = input.port {
        config.port = port;
    }
    if input.enable_tui {
        config.enable_tui = true;
    }
    if let Some(db_path) = input.db_path {
        config.db_path = db_path.into();
    }
    if input.inbound_auth_enabled {
        config.inbound_auth.enabled = true;
    }
    if let Some(inbound_auth_token) = input.inbound_auth_token {
        config.inbound_auth.bearer_token = Some(inbound_auth_token);
    }
    if let Some(max_requests) = input.api_rate_limit_max_requests {
        config.rate_limits.api.max_requests = max_requests;
    }
    if let Some(window_seconds) = input.api_rate_limit_window_seconds {
        config.rate_limits.api.window_seconds = window_seconds;
    }
    if let Some(max_requests) = input.models_rate_limit_max_requests {
        config.rate_limits.v1_models.max_requests = max_requests;
    }
    if let Some(window_seconds) = input.models_rate_limit_window_seconds {
        config.rate_limits.v1_models.window_seconds = window_seconds;
    }
    if let Some(max_requests) = input.chat_rate_limit_max_requests {
        config.rate_limits.v1_chat_completions.max_requests = max_requests;
    }
    if let Some(window_seconds) = input.chat_rate_limit_window_seconds {
        config.rate_limits.v1_chat_completions.window_seconds = window_seconds;
    }
    if let Some(replay_window_seconds) = input.chat_idempotency_replay_window_seconds {
        config.idempotency.chat_completions.replay_window_seconds = replay_window_seconds;
    }

    config.validate()?;
    Ok(config.to_server_config())
}

fn render_config_export(config: &RookConfig) -> Result<String> {
    Ok(serde_json::to_string_pretty(&RookConfigExportView::from_config(config))?)
}

async fn launch_tui_with_runner<F, Fut>(db_path: &str, dashboard_url: String, tui_runner: F) -> Result<()>
where
    F: Fn(RookRegistry, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), rook::domain::RookError>>,
{
    let registry = RookRegistry::open(db_path).await?;
    tui_runner(registry, dashboard_url).await?;
    Ok(())
}

struct ServeOverrides {
    host: Option<String>,
    port: Option<u16>,
    enable_tui: bool,
    db_path: Option<String>,
    inbound_auth_enabled: bool,
    inbound_auth_token: Option<String>,
    api_rate_limit_max_requests: Option<u32>,
    api_rate_limit_window_seconds: Option<u64>,
    models_rate_limit_max_requests: Option<u32>,
    models_rate_limit_window_seconds: Option<u64>,
    chat_rate_limit_max_requests: Option<u32>,
    chat_rate_limit_window_seconds: Option<u64>,
    chat_idempotency_replay_window_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn capture_output() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn render_config_export_outputs_redacted_json() {
        let output = render_config_export(&rook::config::RookConfig {
            inbound_auth: rook::config::InboundAuthConfig {
                enabled: true,
                bearer_token: Some("super-secret-token".to_string()),
            },
            ..Default::default()
        })
        .expect("config export should serialize");

        assert!(output.contains("\"host\": \"127.0.0.1\""));
        assert!(output.contains("\"bearer_token\": \"[redacted]\""));
        assert!(!output.contains("super-secret-token"));
    }

    #[test]
    fn build_export_config_from_path_uses_file_then_env_precedence() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "0.0.0.0"
            port = 6464
            db_path = "/file/rook.db"
            "#,
        )
        .expect("config file should be written");

        let env = std::collections::HashMap::from([
            ("ROOK_PORT".to_string(), "7575".to_string()),
            ("ROOK_DB_PATH".to_string(), "/env/rook.db".to_string()),
        ]);

        let config = build_export_config_from_path(Some(config_path.as_path()), &env)
            .expect("effective config should assemble from path");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 7575);
        assert_eq!(config.db_path, std::path::PathBuf::from("/env/rook.db"));
    }

    #[test]
    fn serve_cli_parses_inbound_auth_flags() {
        let cli = Cli::try_parse_from([
            "rook",
            "serve",
            "--host",
            "0.0.0.0",
            "--port",
            "9999",
            "--db-path",
            "./rook.db",
            "--inbound-auth-enabled",
            "--inbound-auth-token",
            "rook-inbound-secret",
            "--api-rate-limit-max-requests",
            "61",
            "--api-rate-limit-window-seconds",
            "30",
            "--models-rate-limit-max-requests",
            "121",
            "--models-rate-limit-window-seconds",
            "31",
            "--chat-rate-limit-max-requests",
            "32",
            "--chat-rate-limit-window-seconds",
            "33",
            "--chat-idempotency-replay-window-seconds",
            "3600",
        ])
        .unwrap();

        match cli.command {
            Commands::Serve {
                host,
                port,
                tui,
                db_path,
                inbound_auth_enabled,
                inbound_auth_token,
                api_rate_limit_max_requests,
                api_rate_limit_window_seconds,
                models_rate_limit_max_requests,
                models_rate_limit_window_seconds,
                chat_rate_limit_max_requests,
                chat_rate_limit_window_seconds,
                chat_idempotency_replay_window_seconds,
            } => {
                assert_eq!(host, Some("0.0.0.0".to_string()));
                assert_eq!(port, Some(9999));
                assert!(!tui);
                assert_eq!(db_path, Some("./rook.db".to_string()));
                assert!(inbound_auth_enabled);
                assert_eq!(inbound_auth_token, Some("rook-inbound-secret".to_string()));
                assert_eq!(api_rate_limit_max_requests, Some(61));
                assert_eq!(api_rate_limit_window_seconds, Some(30));
                assert_eq!(models_rate_limit_max_requests, Some(121));
                assert_eq!(models_rate_limit_window_seconds, Some(31));
                assert_eq!(chat_rate_limit_max_requests, Some(32));
                assert_eq!(chat_rate_limit_window_seconds, Some(33));
                assert_eq!(chat_idempotency_replay_window_seconds, Some(3600));
            }
            _ => panic!("expected serve command"),
        }
    }

    #[test]
    fn serve_cli_defaults_to_loopback_first_bind_posture() {
        let cli = Cli::try_parse_from(["rook", "serve"]).unwrap();

        match cli.command {
            Commands::Serve {
                host,
                port,
                tui,
                db_path,
                inbound_auth_enabled,
                inbound_auth_token,
                ..
            } => {
                assert_eq!(host, None);
                assert_eq!(port, None);
                assert!(!tui);
                assert_eq!(db_path, None);
                assert!(!inbound_auth_enabled);
                assert_eq!(inbound_auth_token, None);
            }
            _ => panic!("expected serve command"),
        }
    }

    #[test]
    fn build_serve_config_uses_cli_over_shared_config_inputs() {
        let env = std::collections::HashMap::from([("ROOK_PORT".to_string(), "7171".to_string())]);

        let config = build_serve_config(
            ServeOverrides {
                host: Some("0.0.0.0".to_string()),
                port: Some(8181),
                enable_tui: true,
                db_path: Some("/cli/rook.db".to_string()),
                inbound_auth_enabled: true,
                inbound_auth_token: Some("cli-token".to_string()),
                api_rate_limit_max_requests: Some(88),
                api_rate_limit_window_seconds: Some(44),
                models_rate_limit_max_requests: Some(99),
                models_rate_limit_window_seconds: Some(55),
                chat_rate_limit_max_requests: Some(77),
                chat_rate_limit_window_seconds: Some(66),
                chat_idempotency_replay_window_seconds: Some(7200),
            },
            None,
            &env,
        )
        .expect("serve config should assemble");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8181);
        assert_eq!(config.enable_tui, true);
        assert_eq!(config.db_path.as_deref(), Some("/cli/rook.db"));
        assert_eq!(config.inbound_auth.bearer_token.as_deref(), Some("cli-token"));
        assert_eq!(config.rate_limits.api.max_requests, 88);
        assert_eq!(config.rate_limits.v1_models.max_requests, 99);
        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 77);
        assert_eq!(config.idempotency.chat_completions.replay_window_seconds, 7200);
    }

    #[test]
    fn build_serve_config_from_path_uses_file_env_then_cli_precedence() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "1.1.1.1"
            port = 6464
            db_path = "/file/rook.db"
            "#,
        )
        .expect("config file should be written");

        let env = std::collections::HashMap::from([("ROOK_PORT".to_string(), "7171".to_string())]);

        let config = build_serve_config(
            ServeOverrides {
                host: Some("0.0.0.0".to_string()),
                port: Some(8181),
                enable_tui: false,
                db_path: Some("/cli/rook.db".to_string()),
                inbound_auth_enabled: false,
                inbound_auth_token: None,
                api_rate_limit_max_requests: None,
                api_rate_limit_window_seconds: None,
                models_rate_limit_max_requests: None,
                models_rate_limit_window_seconds: None,
                chat_rate_limit_max_requests: None,
                chat_rate_limit_window_seconds: None,
                chat_idempotency_replay_window_seconds: None,
            },
            Some(config_path.as_path()),
            &env,
        )
        .expect("serve config should assemble from file path");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8181);
        assert_eq!(config.db_path.as_deref(), Some("/cli/rook.db"));
    }

    #[test]
    fn build_serve_config_preserves_file_and_env_when_cli_omits_flags() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "2.2.2.2"
            port = 6464
            db_path = "/file/rook.db"
            [rate_limits.api]
            max_requests = 71
            window_seconds = 41
            "#,
        )
        .expect("config file should be written");

        let env = std::collections::HashMap::from([
            ("ROOK_PORT".to_string(), "7171".to_string()),
            ("ROOK_DB_PATH".to_string(), "/env/rook.db".to_string()),
        ]);

        let config = build_serve_config(
            ServeOverrides {
                host: None,
                port: None,
                enable_tui: false,
                db_path: None,
                inbound_auth_enabled: false,
                inbound_auth_token: None,
                api_rate_limit_max_requests: None,
                api_rate_limit_window_seconds: None,
                models_rate_limit_max_requests: None,
                models_rate_limit_window_seconds: None,
                chat_rate_limit_max_requests: None,
                chat_rate_limit_window_seconds: None,
                chat_idempotency_replay_window_seconds: None,
            },
            Some(config_path.as_path()),
            &env,
        )
        .expect("serve config should preserve file/env values");

        assert_eq!(config.host, "2.2.2.2");
        assert_eq!(config.port, 7171);
        assert_eq!(config.db_path.as_deref(), Some("/env/rook.db"));
        assert_eq!(config.rate_limits.api.max_requests, 71);
        assert_eq!(config.rate_limits.api.window_seconds, 41);
    }

    #[tokio::test]
    async fn doctor_command_outputs_pass_report_for_valid_config() {
        let lines = capture_output();
        let output = Arc::clone(&lines);

        let env = std::collections::HashMap::new();

        run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor,
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            move |line| {
                output.lock().expect("output lock should work").push(line);
                Ok(())
            },
            &env,
        )
        .await
        .expect("doctor command should succeed for valid config");

        let joined = lines.lock().expect("output lock should work").join("\n");
        assert!(joined.contains("rook doctor: pass"));
        assert!(joined.contains("summary: total=4, pass=4, warn=0, fail=0"));
        assert!(joined.contains("- config: pass"));
        assert!(joined.contains("- assets: pass"));
        assert!(joined.contains("- inbound_auth: pass"));
        assert!(joined.contains("- database: pass"));
    }

    #[tokio::test]
    async fn doctor_command_returns_error_on_invalid_effective_config() {
        let env = std::collections::HashMap::from([(
            "ROOK_PORT".to_string(),
            "not-a-port".to_string(),
        )]);

        let result = run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor,
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            |_line| Ok(()),
            &env,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn doctor_command_returns_error_when_database_is_unusable() {
        let env = std::collections::HashMap::from([(
            "ROOK_DB_PATH".to_string(),
            "/dev/null/rook.db".to_string(),
        )]);

        let result = run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor,
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            |_line| Ok(()),
            &env,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn doctor_command_does_not_leak_inbound_auth_token() {
        let lines = capture_output();
        let output = Arc::clone(&lines);
        let env = std::collections::HashMap::from([
            ("ROOK_INBOUND_AUTH_ENABLED".to_string(), "true".to_string()),
            (
                "ROOK_INBOUND_AUTH_TOKEN".to_string(),
                "super-secret-token".to_string(),
            ),
        ]);

        run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor,
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            move |line| {
                output.lock().expect("output lock should work").push(line);
                Ok(())
            },
            &env,
        )
        .await
        .expect("doctor command should succeed when auth config is valid");

        let joined = lines.lock().expect("output lock should work").join("\n");
        assert!(joined.contains("- inbound_auth: pass"));
        assert!(!joined.contains("super-secret-token"));
    }

    #[tokio::test]
    async fn config_export_command_outputs_redacted_json() {
        let lines = capture_output();
        let output = Arc::clone(&lines);

        run_cli_with_tui_runner_and_output(
            Cli {
                command: Commands::Config {
                    action: ConfigCommands::Export,
                },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            move |line| {
                output.lock().expect("output lock should work").push(line);
                Ok(())
            },
        )
        .await
        .expect("config export command should succeed");

        let joined = lines.lock().expect("output lock should work").join("\n");
        assert!(joined.contains("\"host\":"));
        assert!(joined.contains("\"bearer_token\":"));
    }

    #[tokio::test]
    async fn config_export_command_returns_error_on_invalid_effective_config() {
        let env = std::collections::HashMap::from([(
            "ROOK_PORT".to_string(),
            "not-a-port".to_string(),
        )]);

        let result = run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Config {
                    action: ConfigCommands::Export,
                },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            |_line| Ok(()),
            &env,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn tui_command_launches_real_runner_with_effective_db_path() {
        let captured = Arc::new(Mutex::new(None::<String>));
        let captured_for_runner = captured.clone();

        run_cli_with_tui_runner_and_output(
            Cli {
                command: Commands::Tui,
            },
            move |_registry, dashboard_url| {
                let captured_for_runner = captured_for_runner.clone();
                async move {
                    *captured_for_runner.lock().unwrap() = Some(format!(
                        "{}|{}",
                        DEFAULT_ROOK_DB_PATH, dashboard_url
                    ));
                    Ok(())
                }
            },
            |_line| Ok(()),
        )
        .await
        .unwrap();

        assert_eq!(
            captured.lock().unwrap().as_deref(),
            Some("./rook.db|http://127.0.0.1:4141")
        );
    }
}
