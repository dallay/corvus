//! Rook — local-first AI provider gateway.
//!
//! Entry point. Parses subcommands and dispatches to the appropriate module.

use anyhow::Result;
use clap::{Parser, Subcommand};
use rook::config::{
    ChatCompletionsIdempotencyConfig, IdempotencyConfig, InboundAuthConfig, RateLimitConfig,
    TransportConfig,
};
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
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// TCP port to listen on
        #[arg(long, default_value_t = 4141)]
        port: u16,

        /// Enable the operator TUI alongside the HTTP server
        #[arg(long, default_value_t = false)]
        tui: bool,

        /// Path to the on-disk SQLite database file
        #[arg(long)]
        db_path: Option<String>,

        /// Enable inbound bearer auth for `/api/*` and `/v1/*`
        #[arg(long, default_value_t = false)]
        inbound_auth_enabled: bool,

        /// Static inbound bearer token for protected HTTP entrypoints
        #[arg(long)]
        inbound_auth_token: Option<String>,

        /// Global max requests allowed per window for `/api/*`
        #[arg(long, default_value_t = 60)]
        api_rate_limit_max_requests: u32,

        /// Window size in seconds for `/api/*` global rate limiting
        #[arg(long, default_value_t = 60)]
        api_rate_limit_window_seconds: u64,

        /// Global max requests allowed per window for `GET /v1/models`
        #[arg(long, default_value_t = 120)]
        models_rate_limit_max_requests: u32,

        /// Window size in seconds for `GET /v1/models` global rate limiting
        #[arg(long, default_value_t = 60)]
        models_rate_limit_window_seconds: u64,

        /// Global max requests allowed per window for `POST /v1/chat/completions`
        #[arg(long, default_value_t = 30)]
        chat_rate_limit_max_requests: u32,

        /// Window size in seconds for `POST /v1/chat/completions` global rate limiting
        #[arg(long, default_value_t = 60)]
        chat_rate_limit_window_seconds: u64,

        /// Replay window in seconds for keyed `POST /v1/chat/completions` idempotency
        #[arg(long, default_value_t = 86_400)]
        chat_idempotency_replay_window_seconds: u64,
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
    run_cli_with_tui_runner(cli, |registry, dashboard_url| async move {
        rook::tui::run_standalone(registry, dashboard_url).await
    })
    .await
}

async fn run_cli_with_tui_runner<F, Fut>(cli: Cli, tui_runner: F) -> Result<()>
where
    F: Fn(RookRegistry, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), rook::domain::RookError>>,
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
            let config = build_server_config(ServerConfigInput {
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
            });
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
            println!("rook doctor: not yet implemented");
            std::process::exit(1);
        }
        Commands::Config {
            action: ConfigCommands::Export,
        } => {
            println!("rook config export: not yet implemented");
            std::process::exit(1);
        }
    }

    Ok(())
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

struct ServerConfigInput {
    host: String,
    port: u16,
    enable_tui: bool,
    db_path: Option<String>,
    inbound_auth_enabled: bool,
    inbound_auth_token: Option<String>,
    api_rate_limit_max_requests: u32,
    api_rate_limit_window_seconds: u64,
    models_rate_limit_max_requests: u32,
    models_rate_limit_window_seconds: u64,
    chat_rate_limit_max_requests: u32,
    chat_rate_limit_window_seconds: u64,
    chat_idempotency_replay_window_seconds: u64,
}

fn build_server_config(input: ServerConfigInput) -> ServerConfig {
    ServerConfig {
        host: input.host,
        port: input.port,
        enable_tui: input.enable_tui,
        db_path: input.db_path,
        inbound_auth: InboundAuthConfig {
            enabled: input.inbound_auth_enabled,
            bearer_token: input.inbound_auth_token,
        },
        transport: TransportConfig::default(),
        rate_limits: RateLimitConfig {
            api: rook::config::SurfaceRateLimitPolicy {
                max_requests: input.api_rate_limit_max_requests,
                window_seconds: input.api_rate_limit_window_seconds,
            },
            v1_models: rook::config::SurfaceRateLimitPolicy {
                max_requests: input.models_rate_limit_max_requests,
                window_seconds: input.models_rate_limit_window_seconds,
            },
            v1_chat_completions: rook::config::SurfaceRateLimitPolicy {
                max_requests: input.chat_rate_limit_max_requests,
                window_seconds: input.chat_rate_limit_window_seconds,
            },
        },
        idempotency: IdempotencyConfig {
            chat_completions: ChatCompletionsIdempotencyConfig {
                enabled: true,
                replay_window_seconds: input.chat_idempotency_replay_window_seconds,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

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
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 9999);
                assert!(!tui);
                assert_eq!(db_path, Some("./rook.db".to_string()));
                assert!(inbound_auth_enabled);
                assert_eq!(inbound_auth_token, Some("rook-inbound-secret".to_string()));
                assert_eq!(api_rate_limit_max_requests, 61);
                assert_eq!(api_rate_limit_window_seconds, 30);
                assert_eq!(models_rate_limit_max_requests, 121);
                assert_eq!(models_rate_limit_window_seconds, 31);
                assert_eq!(chat_rate_limit_max_requests, 32);
                assert_eq!(chat_rate_limit_window_seconds, 33);
                assert_eq!(chat_idempotency_replay_window_seconds, 3600);
            }
            _ => panic!("expected serve command"),
        }
    }

    #[test]
    fn build_server_config_keeps_inbound_auth_separate() {
        let config = build_server_config(ServerConfigInput {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
            db_path: None,
            inbound_auth_enabled: true,
            inbound_auth_token: Some("rook-inbound-secret".to_string()),
            api_rate_limit_max_requests: 61,
            api_rate_limit_window_seconds: 30,
            models_rate_limit_max_requests: 121,
            models_rate_limit_window_seconds: 31,
            chat_rate_limit_max_requests: 32,
            chat_rate_limit_window_seconds: 33,
            chat_idempotency_replay_window_seconds: 3600,
        });

        assert!(config.inbound_auth.enabled);
        assert_eq!(
            config.inbound_auth.bearer_token,
            Some("rook-inbound-secret".to_string())
        );
        assert_eq!(config.transport, TransportConfig::default());
        assert_eq!(config.rate_limits.api.max_requests, 61);
        assert_eq!(config.rate_limits.api.window_seconds, 30);
        assert_eq!(config.rate_limits.v1_models.max_requests, 121);
        assert_eq!(config.rate_limits.v1_models.window_seconds, 31);
        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 32);
        assert_eq!(config.rate_limits.v1_chat_completions.window_seconds, 33);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            3600
        );
    }

    #[tokio::test]
    async fn tui_command_launches_real_runner_with_effective_db_path() {
        let captured = Arc::new(Mutex::new(None::<String>));
        let captured_for_runner = captured.clone();

        run_cli_with_tui_runner(
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
        )
        .await
        .unwrap();

        assert_eq!(
            captured.lock().unwrap().as_deref(),
            Some("./rook.db|http://127.0.0.1:4141")
        );
    }
}
