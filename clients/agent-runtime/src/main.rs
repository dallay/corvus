#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::large_stack_arrays,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unnecessary_wraps,
    dead_code
)]

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

mod agent;
mod approval;
mod auth;
mod bootstrap;
mod capabilities;
mod channels;
mod composer;
mod config;
mod cost;
mod cron;
mod daemon;
mod doctor;
mod gateway;
mod hardware;
mod health;
mod heartbeat;
mod identity;
mod integrations;
mod memory;
mod migration;
mod observability;
mod onboard;
mod peripherals;
mod pre_execution;
mod providers;
mod runtime;
mod search;
mod security;
mod service;
mod skillforge;
mod skills;
#[cfg(test)]
mod test_support;
mod tools;
mod transcription;
mod tunnel;
mod update;
mod util;

use config::Config;

// Re-export so binary modules can use crate::...Commands from the library crate.
pub use corvus::{HardwareCommands, PeripheralCommands, ServiceCommands, ServiceLingerMode};

/// `Corvus` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "corvus")]
#[command(author = "acosta")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,

        /// Memory backend (sqlite, lucid, markdown, none) - used in quick mode (default: sqlite)
        #[arg(long)]
        memory: Option<String>,
    },

    /// Start the AI agent loop (or compose from manifest)
    Agent {
        /// Agent subcommand (build, run, new) - if omitted, starts interactive agent loop
        #[command(subcommand)]
        agent_subcommand: Option<AgentCompositionCommands>,

        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai, openai-codex)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,

        /// Attach a peripheral (board:path, e.g. nucleo-f401re:/dev/ttyACM0)
        #[arg(long)]
        peripheral: Vec<String>,

        /// Allow exactly one over-budget request for this CLI session
        #[arg(long)]
        override_budget: bool,
    },

    /// Run a code-specialist session (inspect, plan, edit, verify, report)
    Code {
        /// Task description or instruction for the code session
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,

        /// Allow exactly one over-budget request for this CLI session
        #[arg(long)]
        override_budget: bool,
    },

    /// Start the gateway server (webhooks, websockets)
    Gateway {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// Start long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    Daemon {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    Service {
        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    Doctor,

    /// Show system status (full details)
    Status,

    /// Configure and manage scheduled tasks
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage provider model catalogs
    Models {
        #[command(subcommand)]
        model_command: ModelCommands,
    },

    /// List supported AI providers
    Providers,

    /// Manage channels (telegram, discord, slack)
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    Skills {
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Manage provider subscription authentication profiles
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },

    /// Discover and introspect USB hardware
    Hardware {
        #[command(subcommand)]
        hardware_command: corvus::HardwareCommands,
    },

    /// Manage hardware peripherals (STM32, RPi GPIO, etc.)
    Peripheral {
        #[command(subcommand)]
        peripheral_command: corvus::PeripheralCommands,
    },

    /// Manage runtime updates
    Update {
        #[command(subcommand)]
        update_command: UpdateCommands,
    },

    /// Inspect and manage runtime cost state
    Cost {
        #[command(subcommand)]
        cost_command: CostCommands,
    },
}

/// Agent composition subcommands (from Phase 4)
#[derive(Subcommand, Debug)]
enum AgentCompositionCommands {
    /// Build an agent from a manifest
    Build {
        /// Path to agent manifest TOML file
        #[arg(long)]
        manifest: std::path::PathBuf,

        /// Output directory for compiled agent
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },

    /// Run an agent directly from a manifest (boot-time composition)
    Run {
        /// Path to agent manifest TOML file
        #[arg(long)]
        manifest: std::path::PathBuf,
    },

    /// Create a new agent from a template
    New {
        /// Template name (e.g., chat-bot, support-bot)
        #[arg(long)]
        template: String,

        /// Agent name
        #[arg(long)]
        name: String,

        /// Output directory (optional)
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum CostCommands {
    /// Show the current cost summary
    Summary,
    /// Show aggregated cost history
    History {
        /// Aggregation period
        #[arg(long, value_enum, default_value_t = CostHistoryPeriod::Day)]
        period: CostHistoryPeriod,

        /// Number of buckets to include
        #[arg(long, default_value_t = 30)]
        window: usize,
    },
    /// Reset tracked costs for a specific scope
    Reset {
        /// Reset scope
        #[arg(long, value_enum, default_value_t = CostResetScopeArg::Day)]
        scope: CostResetScopeArg,

        /// Optional reason recorded in cost audit history
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CostHistoryPeriod {
    Session,
    Day,
    Month,
}

impl From<CostHistoryPeriod> for cost::UsagePeriod {
    fn from(value: CostHistoryPeriod) -> Self {
        match value {
            CostHistoryPeriod::Session => Self::Session,
            CostHistoryPeriod::Day => Self::Day,
            CostHistoryPeriod::Month => Self::Month,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CostResetScopeArg {
    Session,
    Day,
    Month,
}

impl From<CostResetScopeArg> for cost::CostResetScope {
    fn from(value: CostResetScopeArg) -> Self {
        match value {
            CostResetScopeArg::Session => Self::Session,
            CostResetScopeArg::Day => Self::Day,
            CostResetScopeArg::Month => Self::Month,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CliSessionSurface {
    Agent,
    Code,
}

impl CliSessionSurface {
    fn label(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Code => "code",
        }
    }

    fn override_actor(self) -> &'static str {
        match self {
            Self::Agent => "cli-agent",
            Self::Code => "cli-code",
        }
    }
}

#[derive(Subcommand, Debug)]
enum UpdateCommands {
    /// Show update status and effective policy
    Status,
    /// Force an update check
    Check,
    /// Run update install transaction
    Install,
    /// Enable auto-install policy
    AutoEnable,
    /// Disable auto-install policy
    AutoDisable,
    /// Show update audit history
    History,
    /// Confirm a nonce issued by channel update flow
    Confirm {
        /// One-time update confirmation nonce
        nonce: String,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login with OpenAI Codex OAuth
    Login {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Use OAuth device-code flow
        #[arg(long)]
        device_code: bool,
    },
    /// Complete OAuth by pasting redirect URL or auth code
    PasteRedirect {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Full redirect URL or raw OAuth code
        #[arg(long)]
        input: Option<String>,
    },
    /// Paste setup token / auth token (for Anthropic subscription auth)
    PasteToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Token value (if omitted, read interactively)
        #[arg(long)]
        token: Option<String>,
        /// Auth kind override (`authorization` or `api-key`)
        #[arg(long)]
        auth_kind: Option<String>,
    },
    /// Alias for `paste-token` (interactive by default)
    SetupToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Refresh OpenAI Codex access token using refresh token
    Refresh {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name or profile id
        #[arg(long)]
        profile: Option<String>,
    },
    /// Remove auth profile
    Logout {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Set active profile for a provider
    Use {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name or full profile id
        #[arg(long)]
        profile: String,
    },
    /// List auth profiles
    List,
    /// Show auth status with active profile and token expiry info
    Status,
}

#[derive(Subcommand, Debug)]
enum MigrateCommands {
    /// Import memory from an `OpenClaw` workspace into this `Corvus` workspace
    Openclaw {
        /// Optional path to `OpenClaw` workspace (defaults to ~/.openclaw/workspace)
        #[arg(long)]
        source: Option<std::path::PathBuf>,

        /// Validate and preview migration without writing any data
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CronCommands {
    /// List all scheduled tasks
    List,
    /// Add a new scheduled task
    Add {
        /// Cron expression
        expression: String,
        /// Optional IANA timezone (e.g. America/Los_Angeles)
        #[arg(long)]
        tz: Option<String>,
        /// Command to run
        command: String,
    },
    /// Add a one-shot scheduled task at an RFC3339 timestamp
    AddAt {
        /// One-shot timestamp in RFC3339 format
        at: String,
        /// Command to run
        command: String,
    },
    /// Add a fixed-interval scheduled task
    AddEvery {
        /// Interval in milliseconds
        every_ms: u64,
        /// Command to run
        command: String,
    },
    /// Add a one-shot delayed task (e.g. "30m", "2h", "1d")
    Once {
        /// Delay duration
        delay: String,
        /// Command to run
        command: String,
    },
    /// Remove a scheduled task
    Remove {
        /// Task ID
        id: String,
    },
    /// Pause a scheduled task
    Pause {
        /// Task ID
        id: String,
    },
    /// Resume a paused task
    Resume {
        /// Task ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ModelCommands {
    /// Refresh and cache provider models
    Refresh {
        /// Provider name (defaults to configured default provider)
        #[arg(long)]
        provider: Option<String>,

        /// Force live refresh and ignore fresh cache
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ChannelCommands {
    /// List configured channels
    List,
    /// Start all configured channels (Telegram, Discord, Slack)
    Start,
    /// Run health checks for configured channels
    Doctor,
    /// Add a new channel
    Add {
        /// Channel type
        channel_type: String,
        /// Configuration JSON
        config: String,
    },
    /// Remove a channel
    Remove {
        /// Channel name
        name: String,
    },
    /// Bind a Telegram identity (username or numeric user ID) into allowlist
    BindTelegram {
        /// Telegram identity to allow (username without '@' or numeric user ID)
        identity: String,
    },
}

#[derive(Subcommand, Debug)]
enum SkillCommands {
    /// List installed skills
    List {
        /// Show all available official skills from the catalog
        #[arg(long)]
        catalog: bool,
    },
    /// Install a skill from a GitHub URL, local path, or catalog name
    Install {
        /// GitHub URL or local path
        source: String,
        /// Acknowledge trust for third-party skills with tools
        #[arg(long)]
        trust: bool,
    },
    /// Remove an installed skill
    Remove {
        /// Skill name
        name: String,
    },
    /// Search the official skills catalog
    Search {
        /// Search query
        query: String,
    },
    /// Update installed skills
    Update {
        /// Skill name to update (updates all if omitted)
        name: Option<String>,
    },
    /// Discover third-party skills from external sources
    Discover {
        /// Optional search query for discovery
        query: Option<String>,
    },
    /// Lockfile maintenance commands
    Lock {
        #[command(subcommand)]
        cmd: LockCommands,
    },
}

#[derive(Subcommand, Debug)]
enum LockCommands {
    /// Repair the skills lockfile
    Repair,
}

#[derive(Subcommand, Debug)]
enum IntegrationCommands {
    /// Show details about a specific integration
    Info {
        /// Integration name
        name: String,
    },
}

async fn collect_unified_loop_result(
    prompt: &str,
) -> crate::agent::unified_entrypoint::CanonicalOutcome {
    let session = std::env::var("CORVUS_SESSION_ID").unwrap_or_else(|_| "cli-session".to_string());
    let is_preview = std::env::var("CORVUS_UNIFIED_LOOP_PREVIEW").as_deref() == Ok("1");

    if is_preview {
        let mut loop_config = crate::agent::unified_loop::LoopConfig::default();
        let mut step_duration = Duration::from_millis(1);
        let mut tool_calls = 1usize;
        if prompt.contains("timeout") {
            loop_config.timeout = Duration::from_millis(1);
            step_duration = Duration::from_millis(2);
            tool_calls = 2;
        }
        if prompt.contains("needs-approval") {
            loop_config.approval_required_tool = Some("tool-1".to_string());
        }

        let mut preview = crate::agent::unified_entrypoint::execute_with_retry_backoff(
            session.clone(),
            prompt,
            &loop_config,
            crate::agent::unified_entrypoint::UnifiedExecutionConfig {
                tool_calls,
                step_duration,
                max_retries: 1,
                backoff_millis: 25,
                enable_test_triggers: cfg!(test),
            },
        )
        .await;

        if preview.events.iter().any(|event| {
            matches!(
                event,
                crate::agent::unified_loop::LoopEvent::ApprovalRequired(_)
            )
        }) && !preview.events.iter().any(|event| {
            matches!(
                event,
                crate::agent::unified_loop::LoopEvent::Error(message)
                    if message.contains("approval denied")
            )
        }) {
            preview
                .events
                .push(crate::agent::unified_loop::LoopEvent::Error(
                    "approval denied".to_string(),
                ));
        }

        return crate::agent::unified_entrypoint::CanonicalOutcome {
            session_id: preview.session_id,
            events: preview.events,
            approval_required: None,
            timeout_aborted: false,
            fallback_response: if preview.used_fallback {
                Some("fallback response: temporary tool/runtime issue".to_string())
            } else {
                None
            },
        };
    }

    let mut outcome = crate::pre_execution::evaluate(session, prompt).await;

    if outcome.approval_required.is_some()
        && !outcome.events.iter().any(|event| {
            matches!(
                event,
                crate::agent::unified_loop::LoopEvent::Error(message)
                    if message.contains("approval denied")
            )
        })
    {
        outcome
            .events
            .push(crate::agent::unified_loop::LoopEvent::Error(
                "approval denied".to_string(),
            ));
    }

    outcome
}

async fn collect_unified_loop_events(prompt: &str) -> Vec<crate::agent::unified_loop::LoopEvent> {
    collect_unified_loop_result(prompt).await.events
}

fn loop_event_kind(event: &crate::agent::unified_loop::LoopEvent) -> &'static str {
    match event {
        crate::agent::unified_loop::LoopEvent::Start => "start",
        crate::agent::unified_loop::LoopEvent::LLMProgress(_) => "llm_progress",
        crate::agent::unified_loop::LoopEvent::ToolDispatchStarted(_) => "tool_dispatch_started",
        crate::agent::unified_loop::LoopEvent::ToolDispatchCompleted(_) => {
            "tool_dispatch_completed"
        }
        crate::agent::unified_loop::LoopEvent::CompactionTriggered => "compaction_triggered",
        crate::agent::unified_loop::LoopEvent::ApprovalRequired(_) => "approval_required",
        crate::agent::unified_loop::LoopEvent::Complete(_) => "complete",
        crate::agent::unified_loop::LoopEvent::Error(_) => "error",
    }
}

fn init_logging() {
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

async fn maybe_handle_onboard_command(command: &Commands) -> Result<bool> {
    let Commands::Onboard {
        interactive,
        channels_only,
        api_key,
        provider,
        memory,
    } = command
    else {
        return Ok(false);
    };

    let interactive = *interactive;
    let channels_only = *channels_only;
    let api_key = api_key.clone();
    let provider = provider.clone();
    let memory = memory.clone();

    if interactive && channels_only {
        bail!("Use either --interactive or --channels-only, not both");
    }
    if channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
        bail!("--channels-only does not accept --api-key, --provider, or --memory");
    }

    let config = tokio::task::spawn_blocking(move || {
        if channels_only {
            onboard::run_channels_repair_wizard()
        } else if interactive {
            onboard::run_wizard()
        } else {
            onboard::run_quick_setup(api_key.as_deref(), provider.as_deref(), memory.as_deref())
        }
    })
    .await??;

    if maybe_run_onboard_autostart_reaper(&config) {
        channels::start_channels(config).await?;
    }

    Ok(true)
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    // This prevents the error: "could not automatically determine the process-level CryptoProvider"
    // when both aws-lc-rs and ring features are available (or neither is explicitly selected).
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    let cli = Cli::parse();

    // Initialize logging - respects RUST_LOG env var, defaults to INFO
    init_logging();

    // Onboard runs quick setup by default, or the interactive wizard with --interactive.
    // The onboard wizard uses reqwest::blocking internally, which creates its own
    // Tokio runtime. To avoid "Cannot drop a runtime in a context where blocking is
    // not allowed", we run the wizard on a blocking thread via spawn_blocking.
    if maybe_handle_onboard_command(&cli.command).await? {
        return Ok(());
    }

    // All other commands need config loaded first
    let mut config = Config::load_or_init()?;
    config.apply_env_overrides();

    handle_cli_command(cli.command, config).await
}

#[allow(clippy::large_futures)]
async fn handle_cli_command(command: Commands, config: Config) -> Result<()> {
    match command {
        Commands::Onboard { .. } => anyhow::bail!("Onboard command should not reach dispatch"),
        Commands::Agent {
            agent_subcommand,
            message,
            provider,
            model,
            temperature,
            peripheral,
            override_budget,
        } => {
            // Handle agent composition subcommands (Phase 4)
            if let Some(subcommand) = agent_subcommand {
                handle_agent_composition_command(subcommand).await
            } else {
                // Legacy behavior: interactive agent loop
                dispatch_agent_command(
                    config,
                    message,
                    provider,
                    model,
                    temperature,
                    peripheral,
                    override_budget,
                )
                .await
            }
        }
        Commands::Code {
            message,
            provider,
            model,
            temperature,
            override_budget,
        } => {
            dispatch_code_command(
                config,
                message,
                provider,
                model,
                temperature,
                override_budget,
            )
            .await
        }

        Commands::Gateway { port, host } => handle_gateway_command(config, port, host).await,

        Commands::Daemon { port, host } => handle_daemon_command(config, port, host).await,

        Commands::Status => handle_status_command(config).await,

        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        Commands::Models { model_command } => match model_command {
            ModelCommands::Refresh { provider, force } => {
                onboard::run_models_refresh(&config, provider.as_deref(), force)
            }
        },

        Commands::Providers => {
            handle_providers_command(config);
            Ok(())
        }

        Commands::Service { service_command } => service::handle_command(&service_command, &config),

        Commands::Doctor => doctor::run(&config),

        Commands::Channel { channel_command } => match channel_command {
            ChannelCommands::Start => handle_channel_start_command(config).await,
            ChannelCommands::Doctor => channels::doctor_channels(config).await,
            other => channels::handle_command(other, &config),
        },

        Commands::Integrations {
            integration_command,
        } => integrations::handle_command(integration_command, &config),

        Commands::Skills { skill_command } => {
            skills::handle_command(skill_command, &config.workspace_dir, &config.skills)
        }

        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command, &config).await
        }

        Commands::Auth { auth_command } => handle_auth_command(auth_command, &config).await,

        Commands::Hardware { hardware_command } => {
            hardware::handle_command(hardware_command.clone(), &config)
        }

        Commands::Peripheral { peripheral_command } => {
            peripherals::handle_command(peripheral_command.clone(), &config)
        }

        Commands::Update { update_command } => handle_update_command(config, update_command).await,

        Commands::Cost { cost_command } => handle_cost_command(config, cost_command),
    }
}

async fn dispatch_agent_command(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
    peripheral: Vec<String>,
    override_budget: bool,
) -> Result<()> {
    Box::pin(handle_agent_command(
        config,
        message,
        provider,
        model,
        temperature,
        peripheral,
        override_budget,
    ))
    .await
}

async fn dispatch_code_command(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
    override_budget: bool,
) -> Result<()> {
    Box::pin(handle_code_command(
        config,
        message,
        provider,
        model,
        temperature,
        override_budget,
    ))
    .await
}

fn handle_cost_command(config: Config, command: CostCommands) -> Result<()> {
    let service = cost_service_for_config(&config)?;

    match command {
        CostCommands::Summary => {
            let summary = service.current_summary(chrono::Utc::now())?;
            println!("{}", render_cost_summary(&summary, &config.cost));
            Ok(())
        }
        CostCommands::History { period, window } => {
            let history = service.history_window(period.into(), window, chrono::Utc::now())?;
            println!("{}", render_cost_history(&history));
            Ok(())
        }
        CostCommands::Reset { scope, reason } => {
            let result = perform_cost_reset(&config, scope.into(), reason)?;
            println!("{}", render_cost_reset(&result));
            Ok(())
        }
    }
}

fn cost_service_for_config(config: &Config) -> Result<cost::CostService> {
    let tracker = cost::CostTracker::new(config.cost.clone(), &config.workspace_dir)?;
    Ok(cost::CostService::new(Arc::new(tracker)))
}

fn perform_cost_reset(
    config: &Config,
    scope: cost::CostResetScope,
    reason: Option<String>,
) -> Result<cost::CostResetResult> {
    let service = cost_service_for_config(config)?;
    service.reset(
        cost::CostResetRequest {
            scope,
            actor: "cli".to_string(),
            reason,
        },
        chrono::Utc::now(),
    )
}

fn render_cost_summary(
    summary: &cost::CostGovernanceSummary,
    config: &crate::config::CostConfig,
) -> String {
    let session_percent = scope_percent(&summary.scope_statuses, cost::UsagePeriod::Session);
    let daily_percent = scope_percent(&summary.scope_statuses, cost::UsagePeriod::Day);
    let monthly_percent = scope_percent(&summary.scope_statuses, cost::UsagePeriod::Month);
    let active_period = summary.active_period.map(period_label).unwrap_or("none");
    let active_scope = summary
        .scope_statuses
        .iter()
        .max_by(|left, right| {
            budget_state_rank(left.state)
                .cmp(&budget_state_rank(right.state))
                .then_with(|| left.percent_used.total_cmp(&right.percent_used))
        })
        .map(|status| period_label(status.period))
        .unwrap_or("none");

    [
        format!("session_id={}", summary.session_id),
        format!("budget_state={}", budget_state_label(summary.budget_state)),
        format!("active_period={active_period}"),
        format!("active_scope={active_scope}"),
        format!("session_cost_usd={:.4}", summary.usage.session_cost_usd),
        format!("daily_cost_usd={:.4}", summary.usage.daily_cost_usd),
        format!("monthly_cost_usd={:.4}", summary.usage.monthly_cost_usd),
        format!("request_count={}", summary.usage.request_count),
        format!("total_tokens={}", summary.usage.total_tokens),
        format!("percent_used_session={session_percent:.2}"),
        format!("percent_used_daily={daily_percent:.2}"),
        format!("percent_used_monthly={monthly_percent:.2}"),
        format!("cost_enabled={}", config.enabled),
        format!("session_limit_usd={:.4}", config.session_limit_usd),
        format!("daily_limit_usd={:.4}", config.daily_limit_usd),
        format!("monthly_limit_usd={:.4}", config.monthly_limit_usd),
        format!("warn_at_percent={}", config.warn_at_percent),
        format!("allow_override={}", config.allow_override),
    ]
    .join("\n")
}

fn render_cost_history(history: &cost::CostHistory) -> String {
    let mut lines = vec![
        format!("period={}", period_label(history.period)),
        format!("points={}", history.points.len()),
        format!("total_cost_usd={:.4}", history.totals.cost_usd),
        format!("total_tokens={}", history.totals.tokens),
        format!("total_requests={}", history.totals.requests),
    ];

    for point in &history.points {
        lines.push(format!(
            "bucket={} cost_usd={:.4} tokens={} requests={}",
            point.bucket, point.cost_usd, point.tokens, point.requests
        ));
    }

    lines.join("\n")
}

fn render_cost_reset(result: &cost::CostResetResult) -> String {
    [
        format!("scope={}", reset_scope_label(result.scope)),
        format!("removed_cost_usd={:.4}", result.removed_cost_usd),
        format!("removed_requests={}", result.removed_requests),
        format!("effective_at={}", result.effective_at.to_rfc3339()),
    ]
    .join("\n")
}

fn apply_cli_budget_override(
    agent: &crate::agent::Agent,
    surface: CliSessionSurface,
) -> Result<()> {
    let override_record =
        agent.apply_next_request_budget_override(surface.override_actor(), None)?;
    println!(
        "budget_override=applied\nsurface={}\nscope=next_request\noverride_id={}",
        surface.label(),
        override_record.id
    );
    Ok(())
}

fn print_cli_session_summary(
    summary: Option<cost::CostGovernanceSummary>,
    surface: CliSessionSurface,
) {
    if let Some(summary) = summary {
        println!("{}", render_cli_session_summary(&summary, surface));
    }
}

fn render_cli_session_summary(
    summary: &cost::CostGovernanceSummary,
    surface: CliSessionSurface,
) -> String {
    let active_period = summary.active_period.map(period_label).unwrap_or("none");

    [
        "session_summary=true".to_string(),
        format!("surface={}", surface.label()),
        format!("session_id={}", summary.session_id),
        format!("budget_state={}", budget_state_label(summary.budget_state)),
        format!("active_period={active_period}"),
        format!("session_cost_usd={:.4}", summary.usage.session_cost_usd),
        format!("request_count={}", summary.usage.request_count),
        format!("total_tokens={}", summary.usage.total_tokens),
    ]
    .join("\n")
}

fn scope_percent(scope_statuses: &[cost::BudgetScopeStatus], period: cost::UsagePeriod) -> f64 {
    scope_statuses
        .iter()
        .find(|status| status.period == period)
        .map_or(0.0, |status| status.percent_used)
}

fn budget_state_label(state: cost::BudgetState) -> &'static str {
    match state {
        cost::BudgetState::Allowed => "allowed",
        cost::BudgetState::Warning => "warning",
        cost::BudgetState::Exceeded => "exceeded",
    }
}

fn budget_state_rank(state: cost::BudgetState) -> u8 {
    match state {
        cost::BudgetState::Allowed => 0,
        cost::BudgetState::Warning => 1,
        cost::BudgetState::Exceeded => 2,
    }
}

fn period_label(period: cost::UsagePeriod) -> &'static str {
    match period {
        cost::UsagePeriod::Session => "session",
        cost::UsagePeriod::Day => "day",
        cost::UsagePeriod::Month => "month",
        cost::UsagePeriod::Mission => "mission",
    }
}

fn reset_scope_label(scope: cost::CostResetScope) -> &'static str {
    match scope {
        cost::CostResetScope::Session => "session",
        cost::CostResetScope::Day => "day",
        cost::CostResetScope::Month => "month",
    }
}

fn print_update_status(view: &update::UpdateStatusView) {
    println!("current_version={}", view.current_version);
    println!(
        "latest_version={}",
        view.latest_version.as_deref().unwrap_or("unknown")
    );
    println!("update_available={}", view.update_available);
    println!("effective_install_method={}", view.effective_install_method);
    println!("install_method_source={}", view.install_method_source);
    println!(
        "last_check_at_unix={}",
        view.last_check_at_unix
            .map_or_else(|| "unknown".to_string(), |value| value.to_string())
    );
    println!(
        "last_check_outcome={}",
        view.last_check_outcome.as_deref().unwrap_or("unknown")
    );
    println!(
        "policy.auto_install_enabled={}",
        view.policy.auto_install_enabled
    );
    println!(
        "policy.channel_visibility_enabled={}",
        view.policy.channel_visibility_enabled
    );
    println!(
        "policy.cli_startup_notice_enabled={}",
        view.policy.cli_startup_notice_enabled
    );
    println!("policy.restart_policy={}", view.policy.restart_policy);
}

async fn handle_update_command(mut config: Config, command: UpdateCommands) -> Result<()> {
    match command {
        UpdateCommands::Status => {
            let view = update::get_update_status(&config, env!("CARGO_PKG_VERSION"))?;
            print_update_status(&view);
            Ok(())
        }
        UpdateCommands::Check => {
            let view = update::run_update_check(&config, env!("CARGO_PKG_VERSION")).await?;
            print_update_status(&view);
            if view.last_check_outcome.as_deref() == Some("success") {
                Ok(())
            } else {
                anyhow::bail!("update check failed")
            }
        }
        UpdateCommands::Install => {
            let (outcome, message) =
                update::run_update_install(&config, env!("CARGO_PKG_VERSION"))?;
            println!("{message}");
            match outcome {
                update::InstallCommandOutcome::Success => Ok(()),
                update::InstallCommandOutcome::NoUpdate => anyhow::bail!("no update available"),
                update::InstallCommandOutcome::Blocked => anyhow::bail!("install blocked"),
                update::InstallCommandOutcome::Busy => anyhow::bail!("install busy"),
                update::InstallCommandOutcome::Failed => anyhow::bail!("install failed"),
            }
        }
        UpdateCommands::AutoEnable => {
            update::set_auto_update_policy(&mut config, true)?;
            println!("auto_install_enabled=true");
            let view = update::get_update_status(&config, env!("CARGO_PKG_VERSION"))?;
            println!(
                "policy.auto_install_enabled={}",
                view.policy.auto_install_enabled
            );
            Ok(())
        }
        UpdateCommands::AutoDisable => {
            update::set_auto_update_policy(&mut config, false)?;
            println!("auto_install_enabled=false");
            let view = update::get_update_status(&config, env!("CARGO_PKG_VERSION"))?;
            println!(
                "policy.auto_install_enabled={}",
                view.policy.auto_install_enabled
            );
            Ok(())
        }
        UpdateCommands::History => {
            let events = update::read_update_history(&config)?;
            for event in events {
                println!(
                    "{} {} {} {}",
                    event.timestamp_unix, event.action, event.outcome, event.effective_method
                );
            }
            Ok(())
        }
        UpdateCommands::Confirm { nonce } => {
            let (outcome, message) = update::run_update_confirm(&config, &nonce).await?;
            println!("{message}");
            match outcome {
                update::ConfirmCommandOutcome::Success => Ok(()),
                update::ConfirmCommandOutcome::InvalidNonce => {
                    anyhow::bail!("invalid confirmation nonce")
                }
                update::ConfirmCommandOutcome::Failed => {
                    anyhow::bail!("confirmation install failed")
                }
            }
        }
    }
}

fn handle_providers_command(config: Config) {
    let providers = providers::list_providers();
    let current = config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .trim()
        .to_ascii_lowercase();
    println!("Supported providers ({} total):\n", providers.len());
    println!("  ID (use in config)  DESCRIPTION");
    println!("  ─────────────────── ───────────");
    for p in &providers {
        let is_active = p.name.eq_ignore_ascii_case(&current)
            || p.aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&current));
        let marker = if is_active { " (active)" } else { "" };
        let local_tag = if p.local { " [local]" } else { "" };
        let aliases = if p.aliases.is_empty() {
            String::new()
        } else {
            format!("  (aliases: {})", p.aliases.join(", "))
        };
        println!(
            "  {:<19} {}{}{}{}",
            p.name, p.display_name, local_tag, marker, aliases
        );
    }
    println!("\n  custom:<URL>   Any OpenAI-compatible endpoint");
    println!("  anthropic-custom:<URL>  Any Anthropic-compatible endpoint");
}

/// Handle agent composition subcommands (Phase 4: corvus agent build/run/new)
async fn handle_agent_composition_command(command: AgentCompositionCommands) -> Result<()> {
    use crate::composer::handle_composer_command;

    match command {
        AgentCompositionCommands::Build { manifest, output } => {
            handle_composer_command(crate::composer::ComposerCommands::Build { manifest, output })
                .await
        }
        AgentCompositionCommands::Run { manifest } => {
            handle_composer_command(crate::composer::ComposerCommands::Run { manifest }).await
        }
        AgentCompositionCommands::New {
            template,
            name,
            output,
        } => {
            handle_composer_command(crate::composer::ComposerCommands::New {
                template,
                name,
                output,
            })
            .await
        }
    }
}

async fn handle_agent_command(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
    peripheral: Vec<String>,
    override_budget: bool,
) -> Result<()> {
    maybe_print_update_notice_bounded(&config).await;

    let canonical_prompt = message
        .clone()
        .unwrap_or_else(|| "interactive-session".to_string());
    let canonical = collect_unified_loop_result(&canonical_prompt).await;

    if std::env::var("CORVUS_UNIFIED_LOOP_PREVIEW").as_deref() == Ok("1") {
        print_unified_loop_preview(&canonical);
        if std::env::var("CORVUS_UNIFIED_LOOP_ONLY").as_deref() == Ok("1") {
            return Ok(());
        }
    }

    if let Some(blocking) = crate::pre_execution::classify_blocking(&canonical) {
        match blocking {
            crate::pre_execution::BlockingOutcome::ApprovalRequired { tool } => {
                return Err(anyhow!(
                    "[session:{}] approval required for `{tool}`; request blocked",
                    canonical.session_id
                ));
            }
            crate::pre_execution::BlockingOutcome::TimeoutAborted => {
                return Err(anyhow!(
                    "[session:{}] request aborted due to timeout semantics",
                    canonical.session_id
                ));
            }
            crate::pre_execution::BlockingOutcome::Fallback { response } => {
                return Err(anyhow!(
                    "[session:{}] fallback activated: {response}",
                    canonical.session_id
                ));
            }
        }
    }

    if std::env::var("CORVUS_UNIFIED_CANONICAL_ONLY").as_deref() == Ok("1") {
        print_canonical_only(&canonical);
        return Ok(());
    }

    let mut effective_config = config;
    if let Some(p) = provider {
        effective_config.default_provider = Some(p);
    }
    if let Some(m) = model {
        effective_config.default_model = Some(m);
    }
    effective_config.default_temperature = temperature;

    if !peripheral.is_empty() {
        anyhow::bail!(
            "peripheral overrides are not currently supported; found {} override(s): {:?}",
            peripheral.len(),
            peripheral
        );
    }

    let provider_name = effective_config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .to_string();
    let model_name = effective_config
        .default_model
        .as_deref()
        .unwrap_or("anthropic/claude-sonnet-4-20250514")
        .to_string();
    let mut agent = crate::agent::Agent::from_config(&effective_config)?;
    let session_start = Instant::now();

    if override_budget {
        apply_cli_budget_override(&agent, CliSessionSurface::Agent)?;
    }

    agent.record_agent_start_event(&provider_name, &model_name);

    let run_result = if let Some(msg) = message {
        let response = agent.run_single(&msg).await;
        if let Ok(response) = &response {
            println!("{response}");
        }
        response.map(|_| ())
    } else {
        agent.run_interactive().await
    };

    let summary_result = agent.session_cost_summary(chrono::Utc::now());
    agent.record_agent_end_event(&provider_name, &model_name, session_start.elapsed());
    match summary_result {
        Ok(summary) => print_cli_session_summary(summary, CliSessionSurface::Agent),
        Err(error) => tracing::warn!("Failed to load agent session cost summary: {error}"),
    }

    run_result
}

async fn handle_code_command(
    config: Config,
    message: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
    override_budget: bool,
) -> Result<()> {
    let config = apply_code_session_config(config, provider, model, temperature);
    info!("Starting code-specialist session (profile=code)");
    let provider_name = config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .to_string();
    let model_name = config
        .default_model
        .as_deref()
        .unwrap_or("anthropic/claude-sonnet-4-20250514")
        .to_string();
    let mut agent = crate::agent::Agent::code_from_config(&config)?;
    let session_start = Instant::now();

    if override_budget {
        apply_cli_budget_override(&agent, CliSessionSurface::Code)?;
    }

    agent.record_agent_start_event(&provider_name, &model_name);

    let run_result = if let Some(msg) = message {
        let response = agent.run_single(&msg).await;
        if let Ok(response) = &response {
            println!("{response}");
        }
        response.map(|_| ())
    } else {
        agent.run_interactive().await
    };

    let summary_result = agent.session_cost_summary(chrono::Utc::now());
    agent.record_agent_end_event(&provider_name, &model_name, session_start.elapsed());
    match summary_result {
        Ok(summary) => print_cli_session_summary(summary, CliSessionSurface::Code),
        Err(error) => tracing::warn!("Failed to load code session cost summary: {error}"),
    }

    run_result
}

fn apply_code_session_config(
    mut config: Config,
    provider: Option<String>,
    model: Option<String>,
    temperature: f64,
) -> Config {
    if let Some(p) = provider {
        config.default_provider = Some(p);
    }
    if let Some(m) = model {
        config.default_model = Some(m);
    }
    config.default_temperature = temperature;
    config.agent.profile = "code".to_string();
    config.agent.code_session.enabled = true;
    config
}

fn print_unified_loop_preview(canonical: &crate::agent::unified_entrypoint::CanonicalOutcome) {
    println!("loop_session={}", canonical.session_id);
    for event in &canonical.events {
        let event_kind = loop_event_kind(event);
        println!("loop_event={event_kind}");
        info!(
            session_id = %canonical.session_id,
            event_kind,
            "Unified loop preview event"
        );
    }
}

fn print_canonical_only(canonical: &crate::agent::unified_entrypoint::CanonicalOutcome) {
    println!("loop_session={}", canonical.session_id);
    for event in &canonical.events {
        let event_kind = loop_event_kind(event);
        println!("loop_event={event_kind}");
    }
}

async fn handle_gateway_command(
    config: Config,
    port: Option<u16>,
    host: Option<String>,
) -> Result<()> {
    run_startup_staged_image_reaper(&config);
    let port = port.unwrap_or(config.gateway.port);
    let host = host.unwrap_or_else(|| config.gateway.host.clone());
    if port == 0 {
        info!("🚀 Starting Corvus Gateway on {host} (random port)");
    } else {
        info!("🚀 Starting Corvus Gateway on {host}:{port}");
    }
    gateway::run_gateway(&host, port, config).await
}

async fn handle_daemon_command(
    config: Config,
    port: Option<u16>,
    host: Option<String>,
) -> Result<()> {
    run_startup_staged_image_reaper(&config);
    let update_config = config.clone();
    tokio::spawn(async move {
        update::maybe_print_update_notice(&update_config).await;
    });
    let port = port.unwrap_or(config.gateway.port);
    let host = host.unwrap_or_else(|| config.gateway.host.clone());
    if port == 0 {
        info!("🧠 Starting Corvus Daemon on {host} (random port)");
    } else {
        info!("🧠 Starting Corvus Daemon on {host}:{port}");
    }
    daemon::run(config, host, port).await
}

async fn handle_channel_start_command(config: Config) -> Result<()> {
    run_startup_staged_image_reaper(&config);
    channels::start_channels(config).await
}

fn startup_staged_image_reaper_threshold(config: &Config) -> Duration {
    Duration::from_secs(
        config
            .multimodal
            .effective_staged_image_reaper_threshold_minutes()
            * 60,
    )
}

fn run_startup_staged_image_reaper(config: &Config) {
    let report =
        channels::media::reap_startup_staged_images(startup_staged_image_reaper_threshold(config));
    info!(
        cleaned_files = report.deleted_files,
        matched_files = report.matched_files,
        scanned_entries = report.scanned_entries,
        "startup staged image reaper completed"
    );
}

fn should_autostart_onboard_channels() -> bool {
    std::env::var("CORVUS_AUTOSTART_CHANNELS").as_deref() == Ok("1")
}

fn maybe_run_onboard_autostart_reaper(config: &Config) -> bool {
    if should_autostart_onboard_channels() {
        run_startup_staged_image_reaper(config);
        true
    } else {
        false
    }
}

fn command_uses_startup_staged_image_reaper(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Gateway { .. }
            | Commands::Daemon { .. }
            | Commands::Onboard { .. }
            | Commands::Channel {
                channel_command: ChannelCommands::Start,
            }
    )
}

fn dashboard_resume_status_lines() -> [&'static str; 4] {
    [
        "Start gateway: corvus gateway",
        "Start dashboard UI (from repository root): make dashboard-dev",
        "Open http://localhost:4324 and pair via /pair",
        "Need command help: corvus --help",
    ]
}

async fn handle_status_command(config: Config) -> Result<()> {
    maybe_print_update_notice_bounded(&config).await;
    println!("🦀 Corvus Status");
    println!();
    println!("Version:     {}", env!("CARGO_PKG_VERSION"));
    println!("Workspace:   {}", config.workspace_dir.display());
    println!("Config:      {}", config.config_path.display());
    println!();
    println!(
        "🤖 Provider:      {}",
        config.default_provider.as_deref().unwrap_or("openrouter")
    );
    println!(
        "   Model:         {}",
        config.default_model.as_deref().unwrap_or("(default)")
    );
    println!("📊 Observability:  {}", config.observability.backend);
    println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
    println!("⚙️  Runtime:       {}", config.runtime.kind);
    println!(
        "💓 Heartbeat:      {}",
        if config.heartbeat.enabled {
            format!("every {}min", config.heartbeat.interval_minutes)
        } else {
            "disabled".into()
        }
    );
    println!(
        "🧠 Memory:         {} (auto-save: {})",
        config.memory.backend,
        if config.memory.auto_save { "on" } else { "off" }
    );

    println!();
    println!("Security:");
    println!("  Workspace only:    {}", config.autonomy.workspace_only);
    println!(
        "  Allowed commands:  {}",
        config.autonomy.allowed_commands.join(", ")
    );
    println!(
        "  Max actions/hour:  {}",
        config.autonomy.max_actions_per_hour
    );
    if let Some(message) = config.autonomy.action_rate_deprecation_warning() {
        println!("  Deprecation:       {message}");
    }
    println!();
    println!("Channels:");
    println!("  CLI:      ✅ always");
    for (name, configured) in [
        ("Telegram", config.channels_config.telegram.is_some()),
        ("Discord", config.channels_config.discord.is_some()),
        ("Slack", config.channels_config.slack.is_some()),
        ("Webhook", config.channels_config.webhook.is_some()),
    ] {
        println!(
            "  {name:9} {}",
            if configured {
                "✅ configured"
            } else {
                "❌ not configured"
            }
        );
    }
    println!();
    println!("Peripherals:");
    println!(
        "  Enabled:   {}",
        if config.peripherals.enabled {
            "yes"
        } else {
            "no"
        }
    );
    println!("  Boards:    {}", config.peripherals.boards.len());

    println!();
    println!("Web dashboard (resume anytime):");
    for line in dashboard_resume_status_lines() {
        println!("  - {line}");
    }

    Ok(())
}

async fn maybe_print_update_notice_bounded(config: &Config) {
    if tokio::time::timeout(
        Duration::from_millis(500),
        update::maybe_print_update_notice(config),
    )
    .await
    .is_err()
    {
        tracing::debug!("Update notice check timed out after 500ms");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingOpenAiLogin {
    profile: String,
    code_verifier: String,
    state: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingOpenAiLoginFile {
    profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encrypted_code_verifier: Option<String>,
    state: String,
    created_at: String,
}

fn pending_openai_login_path(config: &Config) -> std::path::PathBuf {
    auth::state_dir_from_config(config).join("auth-openai-pending.json")
}

fn pending_openai_secret_store(config: &Config) -> security::secrets::SecretStore {
    security::secrets::SecretStore::new(
        &auth::state_dir_from_config(config),
        config.secrets.encrypt,
    )
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

fn save_pending_openai_login(config: &Config, pending: &PendingOpenAiLogin) -> Result<()> {
    let path = pending_openai_login_path(config);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let secret_store = pending_openai_secret_store(config);
    let encrypted_code_verifier = secret_store.encrypt(&pending.code_verifier)?;
    let persisted = PendingOpenAiLoginFile {
        profile: pending.profile.clone(),
        code_verifier: None,
        encrypted_code_verifier: Some(encrypted_code_verifier),
        state: pending.state.clone(),
        created_at: pending.created_at.clone(),
    };
    let tmp = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let json = serde_json::to_vec_pretty(&persisted)?;
    std::fs::write(&tmp, json)?;
    set_owner_only_permissions(&tmp)?;
    std::fs::rename(&tmp, &path).or_else(|err| {
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            std::fs::remove_file(&path).ok();
            std::fs::rename(&tmp, &path)
        } else {
            Err(err)
        }
    })?;
    set_owner_only_permissions(&path)?;
    Ok(())
}

fn load_pending_openai_login(config: &Config) -> Result<Option<PendingOpenAiLogin>> {
    let path = pending_openai_login_path(config);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let persisted: PendingOpenAiLoginFile = serde_json::from_slice(&bytes)?;
    let secret_store = pending_openai_secret_store(config);
    let code_verifier = if let Some(encrypted) = persisted.encrypted_code_verifier {
        secret_store.decrypt(&encrypted)?
    } else if let Some(plaintext) = persisted.code_verifier {
        plaintext
    } else {
        bail!("Pending OpenAI login is missing code verifier");
    };
    Ok(Some(PendingOpenAiLogin {
        profile: persisted.profile,
        code_verifier,
        state: persisted.state,
        created_at: persisted.created_at,
    }))
}

fn clear_pending_openai_login(config: &Config) {
    let path = pending_openai_login_path(config);
    if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
        let _ = file.set_len(0);
        let _ = file.sync_all();
    }
    let _ = std::fs::remove_file(path);
}

fn read_auth_input(prompt: &str) -> Result<String> {
    let input = Password::new()
        .with_prompt(prompt)
        .allow_empty_password(false)
        .interact()?;
    Ok(input.trim().to_string())
}

fn read_plain_input(prompt: &str) -> Result<String> {
    let input: String = Input::new().with_prompt(prompt).interact_text()?;
    Ok(input.trim().to_string())
}

fn extract_openai_account_id_for_profile(access_token: &str) -> Option<String> {
    let account_id = auth::openai_oauth::extract_account_id_from_jwt(access_token);
    if account_id.is_none() {
        warn!(
            "Could not extract OpenAI account id from OAuth access token; \
             requests may fail until re-authentication."
        );
    }
    account_id
}

fn format_expiry(profile: &auth::profiles::AuthProfile) -> String {
    match profile
        .token_set
        .as_ref()
        .and_then(|token_set| token_set.expires_at)
    {
        Some(ts) => {
            let now = chrono::Utc::now();
            if ts <= now {
                format!("expired at {}", ts.to_rfc3339())
            } else {
                let mins = (ts - now).num_minutes();
                format!("expires in {mins}m ({})", ts.to_rfc3339())
            }
        }
        None => "n/a".to_string(),
    }
}

async fn handle_auth_command(auth_command: AuthCommands, config: &Config) -> Result<()> {
    let auth_service = auth::AuthService::from_config(config);

    match auth_command {
        AuthCommands::Login {
            provider,
            profile,
            device_code,
        } => handle_login(&auth_service, config, &provider, &profile, device_code).await,

        AuthCommands::PasteRedirect {
            provider,
            profile,
            input,
        } => handle_paste_redirect(&auth_service, config, &provider, &profile, input).await,

        AuthCommands::PasteToken {
            provider,
            profile,
            token,
            auth_kind,
        } => handle_paste_token(&auth_service, &provider, &profile, token, auth_kind),

        AuthCommands::SetupToken { provider, profile } => {
            handle_setup_token(&auth_service, &provider, &profile)
        }

        AuthCommands::Refresh { provider, profile } => {
            handle_refresh(&auth_service, &provider, profile.as_deref()).await
        }

        AuthCommands::Logout { provider, profile } => {
            handle_logout(&auth_service, &provider, &profile)
        }

        AuthCommands::Use { provider, profile } => handle_use(&auth_service, &provider, &profile),

        AuthCommands::List => handle_list(&auth_service),

        AuthCommands::Status => handle_status(&auth_service),
    }
}

async fn handle_login(
    auth_service: &auth::AuthService,
    config: &Config,
    provider: &str,
    profile: &str,
    device_code: bool,
) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    if provider != "openai-codex" {
        bail!("`auth login` currently supports only --provider openai-codex");
    }

    let client = reqwest::Client::new();

    if device_code {
        match auth::openai_oauth::start_device_code_flow(&client).await {
            Ok(device) => {
                return handle_device_code_login(auth_service, config, profile, &client, &device)
                    .await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Device-code flow failed: {e}"));
            }
        }
    }

    handle_browser_flow_login(auth_service, config, profile, &client).await
}

async fn handle_device_code_login(
    auth_service: &auth::AuthService,
    config: &Config,
    profile: &str,
    client: &reqwest::Client,
    device: &auth::openai_oauth::DeviceCodeStart,
) -> Result<()> {
    println!("OpenAI device-code login started.");
    println!("Visit: {}", device.verification_uri);
    println!("Code:  {}", device.user_code);
    if let Some(uri_complete) = &device.verification_uri_complete {
        println!("Fast link: {uri_complete}");
    }
    if let Some(message) = &device.message {
        println!("{message}");
    }

    let token_set = auth::openai_oauth::poll_device_code_tokens(client, device).await?;
    let account_id = extract_openai_account_id_for_profile(&token_set.access_token);

    let saved = auth_service.store_openai_tokens(profile, token_set, account_id, true)?;
    clear_pending_openai_login(config);

    println!("Saved profile {}", saved.id);
    println!("Active profile for openai-codex: {}", saved.id);
    Ok(())
}

async fn handle_browser_flow_login(
    auth_service: &auth::AuthService,
    config: &Config,
    profile: &str,
    client: &reqwest::Client,
) -> Result<()> {
    let pkce = auth::openai_oauth::generate_pkce_state();
    let pending = PendingOpenAiLogin {
        profile: profile.to_string(),
        code_verifier: pkce.code_verifier.clone(),
        state: pkce.state.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    save_pending_openai_login(config, &pending)?;

    let authorize_url =
        auth::openai_oauth::build_authorize_url(&pkce, auth::openai_oauth::OPENAI_LOOPBACK_PORT);
    println!("Open this URL in your browser and authorize access:");
    println!("{authorize_url}");
    println!();
    println!(
        "Waiting for callback at {} ...",
        auth::openai_oauth::openai_oauth_redirect_uri(auth::openai_oauth::OPENAI_LOOPBACK_PORT)
    );

    let code = match auth::openai_oauth::receive_loopback_code(
        &pkce.state,
        std::time::Duration::from_secs(180),
        auth::openai_oauth::OPENAI_LOOPBACK_PORT,
    )
    .await
    {
        Ok(code) => code,
        Err(e) => {
            return Err(anyhow::anyhow!("Callback capture failed: {}", e));
        }
    };

    let token_set = auth::openai_oauth::exchange_code_for_tokens(
        client,
        &code,
        &pkce,
        auth::openai_oauth::OPENAI_LOOPBACK_PORT,
    )
    .await?;
    let account_id = extract_openai_account_id_for_profile(&token_set.access_token);

    let saved = auth_service.store_openai_tokens(profile, token_set, account_id, true)?;
    clear_pending_openai_login(config);

    println!("Saved profile {}", saved.id);
    println!("Active profile for openai-codex: {}", saved.id);
    Ok(())
}

async fn handle_paste_redirect(
    auth_service: &auth::AuthService,
    config: &Config,
    provider: &str,
    profile: &str,
    input: Option<String>,
) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    if provider != "openai-codex" {
        bail!("`auth paste-redirect` currently supports only --provider openai-codex");
    }

    let pending = load_pending_openai_login(config)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No pending OpenAI login found. Run `corvus auth login --provider openai-codex` first."
        )
    })?;

    if pending.profile != profile {
        bail!(
            "Pending login profile mismatch: pending={}, requested={}",
            pending.profile,
            profile
        );
    }

    let redirect_input = match input {
        Some(value) => value,
        None => read_plain_input("Paste redirect URL or OAuth code")?,
    };

    let code = auth::openai_oauth::parse_code_from_redirect(&redirect_input, Some(&pending.state))?;

    let pkce = auth::openai_oauth::PkceState {
        code_verifier: pending.code_verifier.clone(),
        code_challenge: String::new(),
        state: pending.state.clone(),
    };

    let client = reqwest::Client::new();
    let token_set = auth::openai_oauth::exchange_code_for_tokens(
        &client,
        &code,
        &pkce,
        auth::openai_oauth::OPENAI_LOOPBACK_PORT,
    )
    .await?;
    let account_id = extract_openai_account_id_for_profile(&token_set.access_token);

    let saved = auth_service.store_openai_tokens(profile, token_set, account_id, true)?;
    clear_pending_openai_login(config);

    println!("Saved profile {}", saved.id);
    println!("Active profile for openai-codex: {}", saved.id);
    Ok(())
}

fn handle_paste_token(
    auth_service: &auth::AuthService,
    provider: &str,
    profile: &str,
    token: Option<String>,
    auth_kind: Option<String>,
) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    let token = match token {
        Some(token) => token.trim().to_string(),
        None => read_auth_input("Paste token")?,
    };
    if token.is_empty() {
        bail!("Token cannot be empty");
    }

    let kind = auth::anthropic_token::detect_auth_kind(&token, auth_kind.as_deref());
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "auth_kind".to_string(),
        kind.as_metadata_value().to_string(),
    );

    let saved = auth_service.store_provider_token(&provider, profile, &token, metadata, true)?;
    println!("Saved profile {}", saved.id);
    println!("Active profile for {provider}: {}", saved.id);
    Ok(())
}

fn handle_setup_token(
    auth_service: &auth::AuthService,
    provider: &str,
    profile: &str,
) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    let token = read_auth_input("Paste token")?;
    if token.is_empty() {
        bail!("Token cannot be empty");
    }

    let kind = auth::anthropic_token::detect_auth_kind(&token, Some("authorization"));
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "auth_kind".to_string(),
        kind.as_metadata_value().to_string(),
    );

    let saved = auth_service.store_provider_token(&provider, profile, &token, metadata, true)?;
    println!("Saved profile {}", saved.id);
    println!("Active profile for {provider}: {}", saved.id);
    Ok(())
}

async fn handle_refresh(
    auth_service: &auth::AuthService,
    provider: &str,
    profile: Option<&str>,
) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    if provider != "openai-codex" {
        bail!("`auth refresh` currently supports only --provider openai-codex");
    }

    match auth_service.get_valid_openai_access_token(profile).await? {
        Some(_) => {
            println!("OpenAI Codex token is valid (refresh completed if needed).");
            Ok(())
        }
        None => {
            bail!("No OpenAI Codex auth profile found. Run `corvus auth login --provider openai-codex`.")
        }
    }
}

fn handle_logout(auth_service: &auth::AuthService, provider: &str, profile: &str) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    let removed = auth_service.remove_profile(&provider, profile)?;
    if removed {
        println!("Removed auth profile {provider}:{profile}");
    } else {
        println!("Auth profile not found: {provider}:{profile}");
    }
    Ok(())
}

fn handle_use(auth_service: &auth::AuthService, provider: &str, profile: &str) -> Result<()> {
    let provider = auth::normalize_provider(provider)?;
    let active = auth_service.set_active_profile(&provider, profile)?;
    println!("Active profile for {provider}: {active}");
    Ok(())
}

fn handle_list(auth_service: &auth::AuthService) -> Result<()> {
    let data = auth_service.load_profiles()?;
    if data.profiles.is_empty() {
        println!("No auth profiles configured.");
        return Ok(());
    }

    for (id, profile) in &data.profiles {
        let active = data
            .active_profiles
            .get(&profile.provider)
            .is_some_and(|active_id| active_id == id);
        let marker = if active { "*" } else { " " };
        println!("{marker} {id}");
    }

    Ok(())
}

fn handle_status(auth_service: &auth::AuthService) -> Result<()> {
    let data = auth_service.load_profiles()?;
    if data.profiles.is_empty() {
        println!("No auth profiles configured.");
        return Ok(());
    }

    for (id, profile) in &data.profiles {
        let active = data
            .active_profiles
            .get(&profile.provider)
            .is_some_and(|active_id| active_id == id);
        let marker = if active { "*" } else { " " };
        println!(
            "{} {} kind={:?} account={} expires={}",
            marker,
            id,
            profile.kind,
            profile.account_id.as_deref().unwrap_or("unknown"),
            format_expiry(profile)
        );
    }

    println!();
    println!("Active profiles:");
    for (provider, active) in &data.active_profiles {
        println!("  {provider}: {active}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tracing_capture::capture_tracing_events;
    use async_trait::async_trait;
    use clap::CommandFactory;
    use clap::Parser;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: Mutex<()> = Mutex::new(());
        &LOCK
    }

    struct MainTestProvider;

    #[async_trait]
    impl crate::providers::Provider for MainTestProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            Ok("ok".to_string())
        }

        async fn chat(
            &self,
            _request: crate::providers::ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> Result<crate::providers::ChatResponse> {
            Ok(crate::providers::ChatResponse {
                text: Some("ok".to_string()),
                tool_calls: vec![],
            })
        }
    }

    struct MainTestTool;

    #[async_trait]
    impl crate::tools::Tool for MainTestTool {
        fn name(&self) -> &str {
            "noop"
        }

        fn description(&self) -> &str {
            "noop"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<crate::tools::ToolResult> {
            Ok(crate::tools::ToolResult {
                success: true,
                output: "ok".to_string(),
                error: None,
                structured: None,
            })
        }
    }

    fn build_test_agent(
        cost_config: crate::config::CostConfig,
        tracker: Option<Arc<crate::cost::CostTracker>>,
        workspace_dir: &std::path::Path,
    ) -> crate::agent::Agent {
        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let memory =
            Arc::from(crate::memory::create_memory(&memory_cfg, workspace_dir, None).unwrap());
        let observer = Arc::new(crate::observability::NoopObserver {});

        crate::agent::Agent::builder()
            .provider(Box::new(MainTestProvider))
            .tools(vec![Box::new(MainTestTool)])
            .memory(memory)
            .observer(observer)
            .tool_dispatcher(Box::new(crate::agent::dispatcher::XmlToolDispatcher))
            .workspace_dir(workspace_dir.to_path_buf())
            .cost_tracker(tracker)
            .cost_config(cost_config)
            .build()
            .unwrap()
    }

    #[test]
    fn cli_definition_has_no_flag_conflicts() {
        Cli::command().debug_assert();
    }

    #[test]
    fn format_expiry_shows_expired() {
        use crate::auth::profiles::{AuthProfile, AuthProfileKind, TokenSet};
        use chrono::{Duration, Utc};

        let past = Utc::now() - Duration::hours(1);
        let token_set = TokenSet {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(past),
            id_token: None,
            scope: None,
            token_type: None,
        };
        let profile = AuthProfile {
            id: "uuid".to_string(),
            provider: "openai-codex".to_string(),
            profile_name: "default".to_string(),
            kind: AuthProfileKind::OAuth,
            token_set: Some(token_set),
            account_id: None,
            workspace_id: None,
            token: None,
            metadata: std::collections::BTreeMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let formatted = format_expiry(&profile);
        assert!(formatted.contains("expired"));
    }

    #[test]
    fn format_expiry_shows_expires_in() {
        use crate::auth::profiles::{AuthProfile, AuthProfileKind, TokenSet};
        use chrono::{Duration, Utc};

        let future = Utc::now() + Duration::hours(2);
        let token_set = TokenSet {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(future),
            id_token: None,
            scope: None,
            token_type: None,
        };
        let profile = AuthProfile {
            id: "uuid".to_string(),
            provider: "openai-codex".to_string(),
            profile_name: "default".to_string(),
            kind: AuthProfileKind::OAuth,
            token_set: Some(token_set),
            account_id: None,
            workspace_id: None,
            token: None,
            metadata: std::collections::BTreeMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let formatted = format_expiry(&profile);
        assert!(formatted.contains("expires in"));
    }

    #[test]
    fn format_expiry_shows_na_when_no_token_set() {
        use crate::auth::profiles::{AuthProfile, AuthProfileKind};
        use chrono::Utc;

        let profile = AuthProfile {
            id: "uuid".to_string(),
            provider: "anthropic".to_string(),
            profile_name: "default".to_string(),
            kind: AuthProfileKind::Token,
            token_set: None,
            account_id: None,
            workspace_id: None,
            token: None,
            metadata: std::collections::BTreeMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let formatted = format_expiry(&profile);
        assert_eq!(formatted, "n/a");
    }

    #[cfg(unix)]
    #[test]
    fn set_owner_only_permissions_sets_0600() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::NamedTempFile;

        let tmp = NamedTempFile::new().unwrap();
        set_owner_only_permissions(tmp.path()).unwrap();

        let mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn pending_openai_login_serde_roundtrip() {
        let pending = PendingOpenAiLogin {
            profile: "default".to_string(),
            code_verifier: "test-verifier".to_string(),
            state: "test-state".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&pending).unwrap();
        let parsed: PendingOpenAiLogin = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.profile, "default");
        assert_eq!(parsed.code_verifier, "test-verifier");
        assert_eq!(parsed.state, "test-state");
    }

    #[test]
    fn pending_openai_login_file_supports_encrypted_verifier() {
        let file = PendingOpenAiLoginFile {
            profile: "default".to_string(),
            code_verifier: None,
            encrypted_code_verifier: Some("encrypted-data".to_string()),
            state: "test-state".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&file).unwrap();
        let parsed: PendingOpenAiLoginFile = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed.encrypted_code_verifier,
            Some("encrypted-data".to_string())
        );
        assert!(parsed.code_verifier.is_none());
    }

    #[test]
    fn save_and_load_pending_openai_login_roundtrips() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);
        let pending = PendingOpenAiLogin {
            profile: "default".to_string(),
            code_verifier: "verifier-secret".to_string(),
            state: "csrf-state".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        save_pending_openai_login(&config, &pending).unwrap();

        let path = pending_openai_login_path(&config);
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(persisted["profile"], "default");
        assert_eq!(persisted["state"], "csrf-state");
        assert!(persisted.get("encrypted_code_verifier").is_some());
        assert!(persisted.get("code_verifier").is_none());

        let loaded = load_pending_openai_login(&config).unwrap().unwrap();
        assert_eq!(loaded.profile, pending.profile);
        assert_eq!(loaded.code_verifier, pending.code_verifier);
        assert_eq!(loaded.state, pending.state);
        assert_eq!(loaded.created_at, pending.created_at);
    }

    #[test]
    fn save_pending_openai_login_overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);
        let path = pending_openai_login_path(&config);

        let pending1 = PendingOpenAiLogin {
            profile: "default".to_string(),
            code_verifier: "verifier-1".to_string(),
            state: "state-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        save_pending_openai_login(&config, &pending1).unwrap();
        assert!(path.exists());

        // Second save should overwrite without error (cross-platform)
        let pending2 = PendingOpenAiLogin {
            profile: "default".to_string(),
            code_verifier: "verifier-2".to_string(),
            state: "state-2".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
        };
        save_pending_openai_login(&config, &pending2).unwrap();
        assert!(path.exists());

        // Loaded value matches the latest saved data
        let loaded = load_pending_openai_login(&config).unwrap().unwrap();
        assert_eq!(loaded.code_verifier, "verifier-2");
        assert_eq!(loaded.state, "state-2");
        assert_eq!(loaded.created_at, "2024-01-02T00:00:00Z");
    }

    #[test]
    fn load_pending_openai_login_supports_legacy_plaintext_files() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);
        let path = pending_openai_login_path(&config);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let persisted = PendingOpenAiLoginFile {
            profile: "legacy".to_string(),
            code_verifier: Some("plain-verifier".to_string()),
            encrypted_code_verifier: None,
            state: "legacy-state".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
        };
        std::fs::write(&path, serde_json::to_vec(&persisted).unwrap()).unwrap();

        let loaded = load_pending_openai_login(&config).unwrap().unwrap();

        assert_eq!(loaded.profile, "legacy");
        assert_eq!(loaded.code_verifier, "plain-verifier");
        assert_eq!(loaded.state, "legacy-state");
        assert_eq!(loaded.created_at, "2024-01-02T00:00:00Z");
    }

    #[test]
    fn load_pending_openai_login_rejects_missing_verifier_fields() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);
        let path = pending_openai_login_path(&config);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let persisted = PendingOpenAiLoginFile {
            profile: "broken".to_string(),
            code_verifier: None,
            encrypted_code_verifier: None,
            state: "broken-state".to_string(),
            created_at: "2024-01-03T00:00:00Z".to_string(),
        };
        std::fs::write(&path, serde_json::to_vec(&persisted).unwrap()).unwrap();

        let error = load_pending_openai_login(&config).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Pending OpenAI login is missing code verifier"
        );
    }

    #[test]
    fn clear_pending_openai_login_removes_persisted_file() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);
        let pending = PendingOpenAiLogin {
            profile: "default".to_string(),
            code_verifier: "verifier-secret".to_string(),
            state: "csrf-state".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        save_pending_openai_login(&config, &pending).unwrap();

        let path = pending_openai_login_path(&config);
        assert!(path.exists());

        clear_pending_openai_login(&config);

        assert!(!path.exists());
        assert!(load_pending_openai_login(&config).unwrap().is_none());
    }

    #[tokio::test]
    async fn unified_loop_collection_returns_lifecycle_events() {
        let events = collect_unified_loop_events("hello").await;
        assert!(events
            .iter()
            .any(|event| matches!(event, crate::agent::unified_loop::LoopEvent::Start)));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                crate::agent::unified_loop::LoopEvent::Complete(message) if message == "done"
            )
        }));
    }

    #[test]
    fn dashboard_resume_status_lines_include_help_and_secure_pairing_path() {
        let lines = dashboard_resume_status_lines();
        let combined = lines.join("\n").to_ascii_lowercase();

        assert!(combined.contains("corvus gateway"));
        assert!(combined.contains("make dashboard-dev"));
        assert!(combined.contains("localhost:4324"));
        assert!(combined.contains("/pair"));
        assert!(combined.contains("corvus --help"));
        assert!(!combined.contains("bearer "));
        assert!(!combined.contains("authorization:"));
        assert!(!combined.contains("/web/admin/"));
    }

    #[test]
    fn startup_staged_image_reaper_uses_default_and_override_thresholds() {
        let mut config = Config::default();
        assert_eq!(
            startup_staged_image_reaper_threshold(&config),
            Duration::from_secs(
                channels::media::DEFAULT_STAGED_IMAGE_REAPER_THRESHOLD_MINUTES * 60,
            )
        );

        config.multimodal.staged_image_reaper_threshold_minutes = Some(90);
        assert_eq!(
            startup_staged_image_reaper_threshold(&config),
            Duration::from_secs(90 * 60)
        );
    }

    #[test]
    fn startup_staged_image_reaper_routes_only_required_command_paths() {
        let gateway = Commands::Gateway {
            port: None,
            host: None,
        };
        let daemon = Commands::Daemon {
            port: None,
            host: None,
        };
        let channel_start = Commands::Channel {
            channel_command: ChannelCommands::Start,
        };
        let channel_doctor = Commands::Channel {
            channel_command: ChannelCommands::Doctor,
        };
        let onboard = Commands::Onboard {
            interactive: false,
            channels_only: false,
            api_key: None,
            provider: None,
            memory: None,
        };

        assert!(command_uses_startup_staged_image_reaper(&gateway));
        assert!(command_uses_startup_staged_image_reaper(&daemon));
        assert!(command_uses_startup_staged_image_reaper(&channel_start));
        assert!(command_uses_startup_staged_image_reaper(&onboard));
        assert!(!command_uses_startup_staged_image_reaper(&channel_doctor));
        assert!(!command_uses_startup_staged_image_reaper(&Commands::Status));
    }

    #[test]
    fn onboard_autostart_reaper_runs_only_when_env_guard_is_enabled() {
        let _env_lock = env_lock().lock().unwrap();
        let _autostart = EnvVarGuard::set("CORVUS_AUTOSTART_CHANNELS", "1");
        let tmp = TempDir::new().unwrap();
        let _tmpdir = EnvVarGuard::set("TMPDIR", tmp.path());
        let config = Config::default();

        let (autostarted, events) =
            capture_tracing_events(|| maybe_run_onboard_autostart_reaper(&config));

        assert!(autostarted);
        let reaper_log = events.iter().find(|event| {
            event
                .field("message")
                .is_some_and(|message| message.contains("startup staged image reaper completed"))
        });
        assert!(reaper_log.is_some());
    }

    #[test]
    fn onboard_autostart_reaper_stays_idle_without_env_guard() {
        let _env_lock = env_lock().lock().unwrap();
        let _autostart = EnvVarGuard::remove("CORVUS_AUTOSTART_CHANNELS");
        let config = Config::default();

        let (autostarted, events) =
            capture_tracing_events(|| maybe_run_onboard_autostart_reaper(&config));

        assert!(!autostarted);
        assert!(events.is_empty());
    }

    #[test]
    fn startup_staged_image_reaper_logs_cleaned_file_count() {
        let _env_lock = env_lock().lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let _tmpdir = EnvVarGuard::set("TMPDIR", tmp.path());
        let config = Config::default();

        let ((), events) = capture_tracing_events(|| run_startup_staged_image_reaper(&config));

        let reaper_log = events
            .iter()
            .find(|event| {
                event.field("message").is_some_and(|message| {
                    message.contains("startup staged image reaper completed")
                })
            })
            .unwrap();
        assert_eq!(reaper_log.field("cleaned_files"), Some("0"));
        assert_eq!(reaper_log.field("matched_files"), Some("0"));
        assert_eq!(reaper_log.field("scanned_entries"), Some("0"));
    }

    #[test]
    fn update_command_contract_parses_status_check_install() {
        let status = Cli::try_parse_from(["corvus", "update", "status"]).unwrap();
        assert!(matches!(
            status.command,
            Commands::Update {
                update_command: UpdateCommands::Status
            }
        ));

        let check = Cli::try_parse_from(["corvus", "update", "check"]).unwrap();
        assert!(matches!(
            check.command,
            Commands::Update {
                update_command: UpdateCommands::Check
            }
        ));

        let install = Cli::try_parse_from(["corvus", "update", "install"]).unwrap();
        assert!(matches!(
            install.command,
            Commands::Update {
                update_command: UpdateCommands::Install
            }
        ));
    }

    #[test]
    fn update_command_contract_parses_policy_toggles_and_history() {
        let auto_enable = Cli::try_parse_from(["corvus", "update", "auto-enable"]).unwrap();
        assert!(matches!(
            auto_enable.command,
            Commands::Update {
                update_command: UpdateCommands::AutoEnable
            }
        ));

        let auto_disable = Cli::try_parse_from(["corvus", "update", "auto-disable"]).unwrap();
        assert!(matches!(
            auto_disable.command,
            Commands::Update {
                update_command: UpdateCommands::AutoDisable
            }
        ));

        let history = Cli::try_parse_from(["corvus", "update", "history"]).unwrap();
        assert!(matches!(
            history.command,
            Commands::Update {
                update_command: UpdateCommands::History
            }
        ));

        let confirm = Cli::try_parse_from(["corvus", "update", "confirm", "abc123"]).unwrap();
        assert!(matches!(
            confirm.command,
            Commands::Update {
                update_command: UpdateCommands::Confirm { .. }
            }
        ));
    }

    #[tokio::test]
    async fn onboard_rejects_interactive_with_channels_only() {
        let cli =
            Cli::try_parse_from(["corvus", "onboard", "--interactive", "--channels-only"]).unwrap();

        let error = maybe_handle_onboard_command(&cli.command)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Use either --interactive or --channels-only, not both"
        );
    }

    #[test]
    fn apply_code_session_config_sets_code_profile_and_overrides() {
        let tmp = TempDir::new().unwrap();
        let config = crate::test_support::test_config(&tmp);

        let updated = apply_code_session_config(
            config,
            Some("openrouter".to_string()),
            Some("model-x".to_string()),
            0.25,
        );

        assert_eq!(updated.default_provider.as_deref(), Some("openrouter"));
        assert_eq!(updated.default_model.as_deref(), Some("model-x"));
        assert_eq!(updated.default_temperature, 0.25);
        assert_eq!(updated.agent.profile, "code");
        assert!(updated.agent.code_session.enabled);
    }

    #[tokio::test]
    async fn onboard_rejects_channels_only_with_quick_setup_flags() {
        for args in [
            vec![
                "corvus",
                "onboard",
                "--channels-only",
                "--api-key",
                "secret",
            ],
            vec![
                "corvus",
                "onboard",
                "--channels-only",
                "--provider",
                "openai",
            ],
            vec!["corvus", "onboard", "--channels-only", "--memory", "sqlite"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            let error = maybe_handle_onboard_command(&cli.command)
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                "--channels-only does not accept --api-key, --provider, or --memory"
            );
        }
    }

    #[test]
    fn code_command_is_distinct_from_agent_command() {
        // Structural test: both Commands::Agent and Commands::Code variants must exist
        // and be distinct. This test ensures the CLI contract is explicit.
        // It compiles only if Commands has a Code variant.
        let _ = |_: Commands| {};
        // Verify Code variant parses via CLI
        let cli = Cli::try_parse_from(["corvus", "code", "--message", "hello"]).unwrap();
        assert!(matches!(cli.command, Commands::Code { .. }));
        // Verify Agent variant is NOT Code
        let agent_cli = Cli::try_parse_from(["corvus", "agent", "--message", "hello"]).unwrap();
        assert!(matches!(agent_cli.command, Commands::Agent { .. }));
    }

    #[test]
    fn agent_and_code_commands_parse_override_budget_flag() {
        let code_cli =
            Cli::try_parse_from(["corvus", "code", "--message", "hello", "--override-budget"])
                .unwrap();
        assert!(matches!(
            code_cli.command,
            Commands::Code {
                override_budget: true,
                ..
            }
        ));

        let agent_cli =
            Cli::try_parse_from(["corvus", "agent", "--message", "hello", "--override-budget"])
                .unwrap();
        assert!(matches!(
            agent_cli.command,
            Commands::Agent {
                override_budget: true,
                ..
            }
        ));
    }

    #[test]
    fn cost_command_contract_parses_summary_history_and_reset() {
        let summary = Cli::try_parse_from(["corvus", "cost", "summary"]).unwrap();
        assert!(matches!(
            summary.command,
            Commands::Cost {
                cost_command: CostCommands::Summary
            }
        ));

        let history = Cli::try_parse_from([
            "corvus", "cost", "history", "--period", "month", "--window", "12",
        ])
        .unwrap();
        assert!(matches!(
            history.command,
            Commands::Cost {
                cost_command: CostCommands::History {
                    period: CostHistoryPeriod::Month,
                    window: 12,
                }
            }
        ));

        let reset = Cli::try_parse_from([
            "corvus", "cost", "reset", "--scope", "day", "--reason", "cleanup",
        ])
        .unwrap();
        assert!(matches!(
            reset.command,
            Commands::Cost {
                cost_command: CostCommands::Reset {
                    scope: CostResetScopeArg::Day,
                    reason: Some(_),
                }
            }
        ));
    }

    #[test]
    fn render_cost_summary_reports_budget_state_and_usage() {
        let tmp = TempDir::new().unwrap();
        let mut config = crate::test_support::test_config(&tmp);
        config.cost.enabled = true;
        config.cost.session_limit_usd = 4.0;

        let tracker = Arc::new(
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap(),
        );
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 3.3;
        tracker.record_usage(usage).unwrap();

        let service = crate::cost::CostService::new(tracker);
        let summary = service.current_summary(chrono::Utc::now()).unwrap();
        let rendered = render_cost_summary(&summary, &config.cost);

        assert!(rendered.contains("budget_state=warning"));
        assert!(rendered.contains("active_period=session"));
        assert!(rendered.contains("percent_used_session="));
        assert!(rendered.contains("session_limit_usd="));
        assert!(rendered.contains("daily_cost_usd="));
        assert!(rendered.contains("monthly_limit_usd="));
    }

    #[test]
    fn cli_override_application_registers_next_request_override() {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: true,
            allow_override: true,
            ..crate::config::CostConfig::default()
        };
        let tracker =
            Arc::new(crate::cost::CostTracker::new(cost_config.clone(), tmp.path()).unwrap());
        let agent = build_test_agent(cost_config, Some(tracker.clone()), tmp.path());

        apply_cli_budget_override(&agent, CliSessionSurface::Agent).unwrap();

        let active_override = tracker
            .active_override(chrono::Utc::now())
            .unwrap()
            .unwrap();
        assert_eq!(
            active_override.scope,
            crate::cost::CostOverrideScope::NextRequest
        );
        assert_eq!(active_override.actor, "cli-agent");
        assert_eq!(active_override.remaining_uses, 1);
    }

    #[test]
    fn cli_override_application_writes_audit_and_allows_next_blocked_request_once() {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: true,
            allow_override: true,
            daily_limit_usd: 1.0,
            monthly_limit_usd: 10.0,
            ..crate::config::CostConfig::default()
        };
        let tracker =
            Arc::new(crate::cost::CostTracker::new(cost_config.clone(), tmp.path()).unwrap());
        let agent = build_test_agent(cost_config, Some(tracker.clone()), tmp.path());
        let service = crate::cost::CostService::new(tracker.clone());

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        apply_cli_budget_override(&agent, CliSessionSurface::Agent).unwrap();

        let first = service
            .evaluate_request(0.1, None, chrono::Utc::now())
            .unwrap();
        assert!(matches!(
            first,
            crate::cost::BudgetEvaluation::Proceed {
                override_applied: Some(_),
                ..
            }
        ));

        let second = service
            .evaluate_request(0.1, None, chrono::Utc::now())
            .unwrap();
        assert!(matches!(
            second,
            crate::cost::BudgetEvaluation::Blocked { .. }
        ));

        let audit = service.audit_trail(10).unwrap();
        assert!(audit.iter().any(|event| {
            event.kind == crate::cost::CostAuditKind::OverrideGranted
                && event.actor.as_deref() == Some("[REDACTED]")
                && event.override_scope == Some(crate::cost::CostOverrideScope::NextRequest)
        }));
        assert!(audit.iter().any(|event| {
            event.kind == crate::cost::CostAuditKind::OverrideConsumed
                && event.actor.as_deref() == Some("[REDACTED]")
        }));
    }

    #[test]
    fn cli_override_application_fails_when_cost_tracking_disabled() {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: false,
            allow_override: true,
            ..crate::config::CostConfig::default()
        };
        let agent = build_test_agent(cost_config, None, tmp.path());

        let error = apply_cli_budget_override(&agent, CliSessionSurface::Code).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Cost tracking is disabled for this session"
        );
    }

    #[test]
    fn cli_override_application_fails_when_override_policy_disabled() {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: true,
            allow_override: false,
            ..crate::config::CostConfig::default()
        };
        let tracker =
            Arc::new(crate::cost::CostTracker::new(cost_config.clone(), tmp.path()).unwrap());
        let agent = build_test_agent(cost_config, Some(tracker), tmp.path());

        let error = apply_cli_budget_override(&agent, CliSessionSurface::Code).unwrap_err();
        assert_eq!(error.to_string(), "Cost overrides are disabled by policy");
    }

    #[test]
    fn render_cli_session_summary_reports_exit_state() {
        let summary = crate::cost::CostGovernanceSummary {
            session_id: "session-123".to_string(),
            usage: crate::cost::types::CostSummary {
                session_cost_usd: 1.75,
                daily_cost_usd: 1.75,
                monthly_cost_usd: 1.75,
                total_tokens: 2048,
                request_count: 3,
                by_model: std::collections::HashMap::new(),
            },
            budget_state: crate::cost::BudgetState::Warning,
            active_period: Some(crate::cost::UsagePeriod::Day),
            scope_statuses: vec![],
            active_override: None,
        };

        let rendered = render_cli_session_summary(&summary, CliSessionSurface::Code);

        assert!(rendered.contains("session_summary=true"));
        assert!(rendered.contains("surface=code"));
        assert!(rendered.contains("session_id=session-123"));
        assert!(rendered.contains("budget_state=warning"));
        assert!(rendered.contains("active_period=day"));
        assert!(rendered.contains("session_cost_usd=1.7500"));
        assert!(rendered.contains("request_count=3"));
        assert!(rendered.contains("total_tokens=2048"));
    }

    #[test]
    fn perform_cost_reset_clears_requested_scope() {
        let tmp = TempDir::new().unwrap();
        let mut config = crate::test_support::test_config(&tmp);
        config.cost.enabled = true;

        let tracker =
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap();
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.25;
        tracker.record_usage(usage).unwrap();

        let result = perform_cost_reset(
            &config,
            crate::cost::CostResetScope::Day,
            Some("test".to_string()),
        )
        .unwrap();
        assert_eq!(result.scope, crate::cost::CostResetScope::Day);
        assert_eq!(result.removed_requests, 1);
    }
}
