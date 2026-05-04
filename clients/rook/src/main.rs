//! Rook — local-first AI provider gateway.
//!
//! Entry point. Parses subcommands and dispatches to the appropriate module.

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use rook::admin::types::{
    UsageAggregateView, UsageGroupView, UsageSummaryPeriod, UsageSummaryView,
    UsageSummaryWindowView,
};
use rook::config::{
    discover_default_config_path, load_effective_config, CliRookConfigOverlay, LoadRookConfigInput,
    PartialChatCompletionsIdempotencyConfig, PartialIdempotencyConfig, PartialInboundAuthConfig,
    PartialRateLimitConfig, PartialSurfaceRateLimitPolicy, PartialUpstreamResilienceConfig,
    RookConfig, RookConfigExportView,
};
use rook::db::usage::{UsageAggregate, UsageGroupAggregate, UsageSummaryQuery};
use rook::doctor;
use rook::registry::RookRegistry;
use rook::server::ServerConfig;
use rook::services::usage::UsageService as _;

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

        /// Enable the operator TUI alongside the HTTP server (additive-only override; cannot disable config/env)
        #[arg(long, action = ArgAction::SetTrue)]
        tui: bool,

        /// Path to the on-disk SQLite database file
        #[arg(long)]
        db_path: Option<String>,

        /// Enable inbound bearer auth for `/api/*` and `/v1/*` (additive-only override; cannot disable config/env)
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

        /// Maximum attempts for buffered upstream chat completion requests
        #[arg(long)]
        upstream_max_buffered_attempts: Option<usize>,

        /// Cooldown seconds applied to an account after upstream failure
        #[arg(long)]
        upstream_failure_cooldown_seconds: Option<u64>,

        /// Backoff in milliseconds between buffered upstream retry attempts
        #[arg(long)]
        upstream_retry_backoff_milliseconds: Option<u64>,

        /// Maximum concurrent upstream requests allowed by the gateway
        #[arg(long)]
        upstream_max_concurrent_requests: Option<usize>,
    },
    /// Launch the operator TUI
    Tui,
    /// Run diagnostics and check configuration
    Doctor {
        /// Render machine-readable JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// Usage accounting reports
    Usage {
        #[command(subcommand)]
        action: UsageCommands,
    },
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

#[derive(Subcommand)]
enum UsageCommands {
    /// Print a persisted usage summary
    Report {
        /// Reporting window to summarize
        #[arg(long, default_value = "day", value_parser = parse_usage_summary_period)]
        period: UsageSummaryPeriod,

        /// Maximum number of groups to include per breakdown
        #[arg(long)]
        limit: Option<usize>,

        /// Output format
        #[arg(long, value_enum, default_value_t = UsageReportFormat::Json)]
        format: UsageReportFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum UsageReportFormat {
    Json,
}

impl std::fmt::Display for UsageReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => f.write_str("json"),
        }
    }
}

fn parse_usage_summary_period(value: &str) -> std::result::Result<UsageSummaryPeriod, String> {
    match value {
        "hour" => Ok(UsageSummaryPeriod::Hour),
        "day" => Ok(UsageSummaryPeriod::Day),
        "month" => Ok(UsageSummaryPeriod::Month),
        other => Err(format!(
            "invalid period '{other}', expected one of: hour, day, month"
        )),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    run_cli(cli).await
}

async fn run_cli(cli: Cli) -> Result<()> {
    run_cli_with_tui_runner_and_output(
        cli,
        |registry, dashboard_url| async move {
            rook::tui::run_standalone(registry, dashboard_url).await
        },
        |line| {
            println!("{line}");
            Ok(())
        },
    )
    .await
}

async fn run_cli_with_tui_runner_and_output<F, Fut, O>(
    cli: Cli,
    tui_runner: F,
    output: O,
) -> Result<()>
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
            upstream_max_buffered_attempts,
            upstream_failure_cooldown_seconds,
            upstream_retry_backoff_milliseconds,
            upstream_max_concurrent_requests,
        } => {
            let config_path = discover_default_config_path(env);
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
                    upstream_max_buffered_attempts,
                    upstream_failure_cooldown_seconds,
                    upstream_retry_backoff_milliseconds,
                    upstream_max_concurrent_requests,
                },
                config_path.as_deref(),
                env,
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
        Commands::Doctor { json } => {
            let config_path = discover_default_config_path(env);
            let report = doctor::run_with_config_path(config_path.as_deref(), env).await;
            if json {
                output(doctor::render_json_report(&report)?)?;
            } else {
                output(doctor::render_report(&report))?;
            }
            doctor::ensure_success(&report)?;
        }
        Commands::Usage {
            action:
                UsageCommands::Report {
                    period,
                    limit,
                    format,
                },
        } => {
            let config_path = discover_default_config_path(env);
            let config = build_export_config_from_path(
                config_path.as_deref(),
                env,
                CliRookConfigOverlay::default(),
            )?;
            let report = build_usage_report(&config, period, limit).await?;
            output(render_usage_report(&report, format)?)?;
        }
        Commands::Config {
            action: ConfigCommands::Export,
        } => {
            let config_path = discover_default_config_path(env);
            let config = build_export_config_from_path(
                config_path.as_deref(),
                env,
                CliRookConfigOverlay::default(),
            )?;
            output(render_config_export(&config)?)?;
        }
    }

    Ok(())
}

fn build_export_config_from_path(
    file_path: Option<&std::path::Path>,
    env: &std::collections::HashMap<String, String>,
    cli: CliRookConfigOverlay,
) -> Result<RookConfig> {
    Ok(load_effective_config(LoadRookConfigInput {
        file_path,
        env,
        cli: Some(cli),
    })?)
}

fn build_serve_config(
    input: ServeOverrides,
    file_path: Option<&std::path::Path>,
    env: &std::collections::HashMap<String, String>,
) -> Result<ServerConfig> {
    let config = load_effective_config(LoadRookConfigInput {
        file_path,
        env,
        cli: Some(input.into_cli_overlay()),
    })?;

    Ok(config.to_server_config())
}

fn render_config_export(config: &RookConfig) -> Result<String> {
    Ok(serde_json::to_string_pretty(
        &RookConfigExportView::from_config(config),
    )?)
}

fn render_usage_report(view: &UsageSummaryView, format: UsageReportFormat) -> Result<String> {
    match format {
        UsageReportFormat::Json => Ok(serde_json::to_string_pretty(view)?),
    }
}

async fn build_usage_report(
    config: &RookConfig,
    period: UsageSummaryPeriod,
    limit: Option<usize>,
) -> Result<UsageSummaryView> {
    let registry = RookRegistry::open(config.db_path.to_string_lossy().as_ref()).await?;
    let now = chrono::Utc::now();
    let since = usage_window_start(period.clone(), now);
    let limit = limit.unwrap_or(10).clamp(1, 100);
    let summary = registry
        .usage()
        .summary(UsageSummaryQuery {
            since,
            until: now,
            limit,
        })
        .await?;

    Ok(UsageSummaryView {
        available: true,
        window: UsageSummaryWindowView {
            period,
            since,
            until: now,
        },
        totals: usage_aggregate_view(summary.totals),
        by_model: usage_group_views(summary.by_model),
        by_vendor: usage_group_views(summary.by_vendor),
        by_outcome: usage_group_views(summary.by_outcome),
    })
}

fn usage_window_start(
    period: UsageSummaryPeriod,
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    match period {
        UsageSummaryPeriod::Hour => now - chrono::Duration::hours(1),
        UsageSummaryPeriod::Day => now - chrono::Duration::days(1),
        UsageSummaryPeriod::Month => now - chrono::Duration::days(30),
    }
}

fn usage_aggregate_view(aggregate: UsageAggregate) -> UsageAggregateView {
    UsageAggregateView {
        requests: aggregate.requests,
        successful_requests: aggregate.successful_requests,
        failed_requests: aggregate.failed_requests,
        streaming_requests: aggregate.streaming_requests,
        prompt_tokens: aggregate.prompt_tokens,
        completion_tokens: aggregate.completion_tokens,
        total_tokens: aggregate.total_tokens,
        known_token_requests: aggregate.known_token_requests,
        estimated_cost_usd: aggregate.estimated_cost_usd,
    }
}

fn usage_group_views(groups: Vec<UsageGroupAggregate>) -> Vec<UsageGroupView> {
    groups
        .into_iter()
        .map(|group| UsageGroupView {
            key: group.key,
            aggregate: usage_aggregate_view(group.aggregate),
        })
        .collect()
}

async fn launch_tui_with_runner<F, Fut>(
    db_path: &str,
    dashboard_url: String,
    tui_runner: F,
) -> Result<()>
where
    F: Fn(RookRegistry, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), rook::domain::RookError>>,
{
    let registry = RookRegistry::open(db_path).await?;
    tui_runner(registry, dashboard_url).await?;
    Ok(())
}

#[derive(Clone)]
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
    upstream_max_buffered_attempts: Option<usize>,
    upstream_failure_cooldown_seconds: Option<u64>,
    upstream_retry_backoff_milliseconds: Option<u64>,
    upstream_max_concurrent_requests: Option<usize>,
}

impl ServeOverrides {
    fn into_cli_overlay(self) -> CliRookConfigOverlay {
        let upstream_resilience = PartialUpstreamResilienceConfig {
            max_buffered_attempts: self.upstream_max_buffered_attempts,
            failure_cooldown_seconds: self.upstream_failure_cooldown_seconds,
            retry_backoff_milliseconds: self.upstream_retry_backoff_milliseconds,
            max_concurrent_upstream_requests: self.upstream_max_concurrent_requests,
        };
        let upstream_resilience = option_if(
            upstream_resilience.max_buffered_attempts.is_some()
                || upstream_resilience.failure_cooldown_seconds.is_some()
                || upstream_resilience.retry_backoff_milliseconds.is_some()
                || upstream_resilience
                    .max_concurrent_upstream_requests
                    .is_some(),
            upstream_resilience,
        );

        CliRookConfigOverlay {
            host: self.host,
            port: self.port,
            enable_tui: self.enable_tui.then_some(true),
            db_path: self.db_path.map(Into::into),
            inbound_auth: option_if(
                self.inbound_auth_enabled || self.inbound_auth_token.is_some(),
                PartialInboundAuthConfig {
                    enabled: self.inbound_auth_enabled.then_some(true),
                    bearer_token: self.inbound_auth_token,
                },
            ),
            transport: None,
            rate_limits: option_if(
                self.api_rate_limit_max_requests.is_some()
                    || self.api_rate_limit_window_seconds.is_some()
                    || self.models_rate_limit_max_requests.is_some()
                    || self.models_rate_limit_window_seconds.is_some()
                    || self.chat_rate_limit_max_requests.is_some()
                    || self.chat_rate_limit_window_seconds.is_some(),
                PartialRateLimitConfig {
                    api: option_if(
                        self.api_rate_limit_max_requests.is_some()
                            || self.api_rate_limit_window_seconds.is_some(),
                        PartialSurfaceRateLimitPolicy {
                            max_requests: self.api_rate_limit_max_requests,
                            window_seconds: self.api_rate_limit_window_seconds,
                        },
                    ),
                    v1_models: option_if(
                        self.models_rate_limit_max_requests.is_some()
                            || self.models_rate_limit_window_seconds.is_some(),
                        PartialSurfaceRateLimitPolicy {
                            max_requests: self.models_rate_limit_max_requests,
                            window_seconds: self.models_rate_limit_window_seconds,
                        },
                    ),
                    v1_chat_completions: option_if(
                        self.chat_rate_limit_max_requests.is_some()
                            || self.chat_rate_limit_window_seconds.is_some(),
                        PartialSurfaceRateLimitPolicy {
                            max_requests: self.chat_rate_limit_max_requests,
                            window_seconds: self.chat_rate_limit_window_seconds,
                        },
                    ),
                },
            ),
            idempotency: option_if(
                self.chat_idempotency_replay_window_seconds.is_some(),
                PartialIdempotencyConfig {
                    chat_completions: Some(PartialChatCompletionsIdempotencyConfig {
                        enabled: None,
                        replay_window_seconds: self.chat_idempotency_replay_window_seconds,
                    }),
                },
            ),
            upstream_resilience,
        }
    }
}

fn option_if<T>(condition: bool, value: T) -> Option<T> {
    condition.then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};

    fn capture_output() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn usage_report_cli_parses_json_report_options() {
        let cli = Cli::try_parse_from([
            "rook", "usage", "report", "--period", "day", "--limit", "25", "--format", "json",
        ])
        .unwrap();

        match cli.command {
            Commands::Usage {
                action:
                    UsageCommands::Report {
                        period,
                        limit,
                        format,
                    },
            } => {
                assert_eq!(period, rook::admin::types::UsageSummaryPeriod::Day);
                assert_eq!(limit, Some(25));
                assert_eq!(format, UsageReportFormat::Json);
            }
            _ => panic!("expected usage report command"),
        }
    }

    #[test]
    fn doctor_cli_parses_json_flag() {
        let cli = Cli::try_parse_from(["rook", "doctor", "--json"]).unwrap();

        match cli.command {
            Commands::Doctor { json } => assert!(json),
            _ => panic!("expected doctor command"),
        }
    }

    #[test]
    fn serve_overrides_include_upstream_resilience_cli_values() {
        let overrides = ServeOverrides {
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
            upstream_max_buffered_attempts: Some(4),
            upstream_failure_cooldown_seconds: Some(90),
            upstream_retry_backoff_milliseconds: Some(50),
            upstream_max_concurrent_requests: Some(9),
        };

        let env = std::collections::HashMap::new();
        let config = build_serve_config(overrides, None, &env).unwrap();

        assert_eq!(config.upstream_resilience.max_buffered_attempts, 4);
        assert_eq!(config.upstream_resilience.failure_cooldown.as_secs(), 90);
        assert_eq!(config.upstream_resilience.retry_backoff.as_millis(), 50);
        assert_eq!(
            config.upstream_resilience.max_concurrent_upstream_requests,
            9
        );
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

    #[tokio::test]
    async fn build_usage_report_reads_configured_database_summary() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let db_path = temp_dir.path().join("usage.db");
        let registry = RookRegistry::open(db_path.to_str().unwrap())
            .await
            .expect("registry should open temp db");
        registry
            .usage()
            .record(rook::db::usage::StoredUsageEvent {
                id: "usage-cli-one".to_string(),
                occurred_at: Utc::now(),
                request_id: Some("req-usage-cli".to_string()),
                logical_model: "gpt-4o".to_string(),
                vendor: "open_ai".to_string(),
                account_id: Some("account-id".to_string()),
                account_label: "primary".to_string(),
                stream: false,
                outcome: "success".to_string(),
                status_code: 200,
                latency_ms: 42,
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                total_tokens: Some(30),
                cost_usd: None,
                currency: None,
                provider_request_id: None,
            })
            .await
            .expect("usage event should be recorded");
        drop(registry);

        let config = RookConfig {
            db_path: db_path.clone(),
            ..Default::default()
        };

        let report = build_usage_report(&config, UsageSummaryPeriod::Day, Some(10))
            .await
            .expect("usage report should load from configured db");

        assert!(report.available);
        assert_eq!(report.totals.requests, 1);
        assert_eq!(report.totals.successful_requests, 1);
        assert_eq!(report.totals.total_tokens, 30);
        assert_eq!(report.by_model[0].key, "gpt-4o");
        assert_eq!(report.by_vendor[0].key, "open_ai");
        assert_eq!(report.by_outcome[0].key, "success");
    }

    #[test]
    fn render_usage_report_outputs_admin_usage_json_shape() {
        let since = chrono::Utc::now() - chrono::Duration::days(1);
        let until = chrono::Utc::now();
        let view = rook::admin::types::UsageSummaryView {
            available: true,
            window: rook::admin::types::UsageSummaryWindowView {
                period: rook::admin::types::UsageSummaryPeriod::Day,
                since,
                until,
            },
            totals: rook::admin::types::UsageAggregateView {
                requests: 1,
                successful_requests: 1,
                failed_requests: 0,
                streaming_requests: 0,
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                known_token_requests: 1,
                estimated_cost_usd: None,
            },
            by_model: vec![rook::admin::types::UsageGroupView {
                key: "gpt-4o".to_string(),
                aggregate: rook::admin::types::UsageAggregateView {
                    requests: 1,
                    successful_requests: 1,
                    failed_requests: 0,
                    streaming_requests: 0,
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                    known_token_requests: 1,
                    estimated_cost_usd: None,
                },
            }],
            by_vendor: Vec::new(),
            by_outcome: Vec::new(),
        };

        let rendered = render_usage_report(&view, UsageReportFormat::Json)
            .expect("usage report should render as JSON");
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["available"], true);
        assert_eq!(json["window"]["period"], "day");
        assert_eq!(json["totals"]["requests"], 1);
        assert_eq!(json["by_model"][0]["key"], "gpt-4o");
        assert!(json.get("by_vendor").is_some());
        assert!(json.get("by_outcome").is_some());
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

        let config = build_export_config_from_path(
            Some(config_path.as_path()),
            &env,
            CliRookConfigOverlay::default(),
        )
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
                upstream_max_buffered_attempts,
                upstream_failure_cooldown_seconds,
                upstream_retry_backoff_milliseconds,
                upstream_max_concurrent_requests,
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
                assert_eq!(upstream_max_buffered_attempts, None);
                assert_eq!(upstream_failure_cooldown_seconds, None);
                assert_eq!(upstream_retry_backoff_milliseconds, None);
                assert_eq!(upstream_max_concurrent_requests, None);
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
    fn serve_and_config_export_share_effective_config_resolution() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("config.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "1.1.1.1"
            port = 6464
            db_path = "/file/rook.db"

            [inbound_auth]
            enabled = true
            bearer_token = "file-token"
            "#,
        )
        .expect("config file should be written");

        let env = std::collections::HashMap::from([
            ("HOME".to_string(), temp_dir.path().display().to_string()),
            ("ROOK_PORT".to_string(), "7474".to_string()),
            ("ROOK_DB_PATH".to_string(), "/env/rook.db".to_string()),
            (
                "ROOK_INBOUND_AUTH_TOKEN".to_string(),
                "env-token".to_string(),
            ),
        ]);
        let rook_dir = temp_dir.path().join(".config").join("rook");
        std::fs::create_dir_all(&rook_dir).expect("rook config dir should exist");
        std::fs::copy(&config_path, rook_dir.join("config.toml"))
            .expect("default config path should be populated");

        let serve_overrides = ServeOverrides {
            host: Some("3.3.3.3".to_string()),
            port: Some(8484),
            enable_tui: true,
            db_path: Some("/cli/rook.db".to_string()),
            inbound_auth_enabled: true,
            inbound_auth_token: Some("cli-token".to_string()),
            api_rate_limit_max_requests: None,
            api_rate_limit_window_seconds: None,
            models_rate_limit_max_requests: None,
            models_rate_limit_window_seconds: None,
            chat_rate_limit_max_requests: None,
            chat_rate_limit_window_seconds: None,
            chat_idempotency_replay_window_seconds: None,
            upstream_max_buffered_attempts: None,
            upstream_failure_cooldown_seconds: None,
            upstream_retry_backoff_milliseconds: None,
            upstream_max_concurrent_requests: None,
        };

        let serve = build_serve_config(serve_overrides.clone(), Some(config_path.as_path()), &env)
            .expect("serve config should resolve");

        let export = build_export_config_from_path(
            Some(config_path.as_path()),
            &env,
            serve_overrides.into_cli_overlay(),
        )
        .expect("export config should resolve");
        let rendered = render_config_export(&export).expect("export should render");

        assert_eq!(serve.host, export.host);
        assert_eq!(serve.port, export.port);
        assert_eq!(serve.db_path.as_deref(), Some("/cli/rook.db"));
        assert_eq!(export.db_path, std::path::PathBuf::from("/cli/rook.db"));
        assert_eq!(
            serve.inbound_auth.bearer_token.as_deref(),
            Some("cli-token")
        );
        assert_eq!(
            export.inbound_auth.bearer_token.as_deref(),
            Some("cli-token")
        );

        assert!(rendered.contains("\"host\": \"3.3.3.3\""));
        assert!(rendered.contains("\"port\": 8484"));
        assert!(rendered.contains("\"db_path\": \"/cli/rook.db\""));
        assert!(rendered.contains("\"bearer_token\": \"[redacted]\""));
        assert!(!rendered.contains("env-token"));
        assert!(!rendered.contains("cli-token"));
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
                upstream_max_buffered_attempts: None,
                upstream_failure_cooldown_seconds: None,
                upstream_retry_backoff_milliseconds: None,
                upstream_max_concurrent_requests: None,
            },
            None,
            &env,
        )
        .expect("serve config should assemble");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8181);
        assert!(config.enable_tui);
        assert_eq!(config.db_path.as_deref(), Some("/cli/rook.db"));
        assert_eq!(
            config.inbound_auth.bearer_token.as_deref(),
            Some("cli-token")
        );
        assert_eq!(config.rate_limits.api.max_requests, 88);
        assert_eq!(config.rate_limits.v1_models.max_requests, 99);
        assert_eq!(config.rate_limits.v1_chat_completions.max_requests, 77);
        assert_eq!(
            config.idempotency.chat_completions.replay_window_seconds,
            7200
        );
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
                upstream_max_buffered_attempts: None,
                upstream_failure_cooldown_seconds: None,
                upstream_retry_backoff_milliseconds: None,
                upstream_max_concurrent_requests: None,
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
                upstream_max_buffered_attempts: None,
                upstream_failure_cooldown_seconds: None,
                upstream_retry_backoff_milliseconds: None,
                upstream_max_concurrent_requests: None,
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
    async fn usage_report_command_outputs_json_without_leaking_config_secret() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let db_path = temp_dir.path().join("usage-command.db");
        let config_path = temp_dir.path().join("rook.toml");
        std::fs::write(
            &config_path,
            format!(
                r#"
                db_path = "{}"

                [inbound_auth]
                enabled = true
                bearer_token = "super-secret-usage-token"
                "#,
                db_path.display()
            ),
        )
        .expect("config file should be written");

        let registry = RookRegistry::open(db_path.to_str().unwrap())
            .await
            .expect("registry should open temp db");
        registry
            .usage()
            .record(rook::db::usage::StoredUsageEvent {
                id: "usage-command-one".to_string(),
                occurred_at: Utc::now(),
                request_id: Some("req-usage-command".to_string()),
                logical_model: "gpt-4o-mini".to_string(),
                vendor: "open_ai".to_string(),
                account_id: Some("account-id".to_string()),
                account_label: "primary".to_string(),
                stream: false,
                outcome: "success".to_string(),
                status_code: 200,
                latency_ms: 24,
                prompt_tokens: Some(3),
                completion_tokens: Some(4),
                total_tokens: Some(7),
                cost_usd: None,
                currency: None,
                provider_request_id: None,
            })
            .await
            .expect("usage event should be recorded");
        drop(registry);

        let lines = capture_output();
        let output = Arc::clone(&lines);
        let env = std::collections::HashMap::from([(
            "XDG_CONFIG_HOME".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        )]);
        let rook_config_dir = temp_dir.path().join("rook");
        std::fs::create_dir_all(&rook_config_dir).expect("rook config dir should exist");
        std::fs::copy(&config_path, rook_config_dir.join("config.toml"))
            .expect("default config should be installed");

        run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Usage {
                    action: UsageCommands::Report {
                        period: UsageSummaryPeriod::Day,
                        limit: Some(10),
                        format: UsageReportFormat::Json,
                    },
                },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            move |line| {
                output.lock().expect("output lock should work").push(line);
                Ok(())
            },
            &env,
        )
        .await
        .expect("usage report command should succeed");

        let rendered = lines.lock().expect("output lock should work").join("\n");
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["available"], true);
        assert_eq!(json["totals"]["requests"], 1);
        assert_eq!(json["totals"]["total_tokens"], 7);
        assert_eq!(json["by_model"][0]["key"], "gpt-4o-mini");
        assert!(!rendered.contains("super-secret-usage-token"));
    }

    #[tokio::test]
    async fn doctor_command_outputs_pass_report_for_valid_config() {
        let lines = capture_output();
        let output = Arc::clone(&lines);

        let env = std::collections::HashMap::new();

        run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor { json: false },
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
        assert!(joined.contains("- database: pass"));
        assert!(joined.contains("- assets: pass"));
        assert!(joined.contains("- inbound_auth: pass"));
        assert!(joined.contains("detail: effective bind target: 127.0.0.1:4141"));
    }

    #[tokio::test]
    async fn doctor_command_outputs_json_when_requested() {
        let lines = capture_output();
        let output = Arc::clone(&lines);
        let env = std::collections::HashMap::new();

        run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor { json: true },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            move |line| {
                output.lock().expect("output lock should work").push(line);
                Ok(())
            },
            &env,
        )
        .await
        .expect("doctor json command should succeed for valid config");

        let joined = lines.lock().expect("output lock should work").join("\n");
        let parsed: serde_json::Value = serde_json::from_str(&joined).unwrap();

        assert_eq!(parsed["status"], "pass");
        assert!(parsed["checks"].as_array().unwrap().len() >= 4);
        assert_eq!(parsed["checks"][0]["name"], "config");
    }

    #[tokio::test]
    async fn doctor_command_returns_error_on_invalid_effective_config() {
        let env =
            std::collections::HashMap::from([("ROOK_PORT".to_string(), "not-a-port".to_string())]);

        let result = run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor { json: false },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            |_line| Ok(()),
            &env,
        )
        .await;

        let error_text = result.expect_err("invalid config should fail").to_string();
        assert!(error_text.contains("ROOK_PORT"));
        assert!(error_text.contains("not-a-port"));
        assert!(error_text.contains("parse") || error_text.contains("port"));
    }

    #[tokio::test]
    async fn doctor_command_returns_error_when_database_is_unusable() {
        let env = std::collections::HashMap::from([(
            "ROOK_DB_PATH".to_string(),
            "/dev/null/rook.db".to_string(),
        )]);

        let result = run_cli_with_tui_runner_output_and_env(
            Cli {
                command: Commands::Doctor { json: false },
            },
            |_registry, _dashboard_url| async move { Ok(()) },
            |_line| Ok(()),
            &env,
        )
        .await;

        let error_text = result.expect_err("unusable db should fail").to_string();
        assert!(error_text.contains("database"));
        assert!(error_text.contains("startup"));
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
                command: Commands::Doctor { json: false },
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
        assert!(joined.contains("detail: inbound auth state: enabled with token configured"));
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
        let env =
            std::collections::HashMap::from([("ROOK_PORT".to_string(), "not-a-port".to_string())]);

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

        let error_text = result.expect_err("invalid config should fail").to_string();
        assert!(error_text.contains("ROOK_PORT"));
        assert!(error_text.contains("not-a-port"));
        assert!(error_text.contains("parse") || error_text.contains("port"));
    }

    #[test]
    fn build_serve_config_returns_error_on_invalid_effective_config() {
        let env = std::collections::HashMap::from([(
            "ROOK_INBOUND_AUTH_ENABLED".to_string(),
            "true".to_string(),
        )]);

        let result = build_serve_config(
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
                upstream_max_buffered_attempts: None,
                upstream_failure_cooldown_seconds: None,
                upstream_retry_backoff_milliseconds: None,
                upstream_max_concurrent_requests: None,
            },
            None,
            &env,
        );

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
                    *captured_for_runner.lock().unwrap() =
                        Some(format!("{}|{}", DEFAULT_ROOK_DB_PATH, dashboard_url));
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
