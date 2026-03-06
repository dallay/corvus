use crate::config::schema::{DingTalkConfig, IrcConfig, QQConfig, StreamMode, WhatsAppConfig};
use crate::config::{
    AutonomyConfig, BrowserConfig, ChannelsConfig, ComposioConfig, Config, DiscordConfig,
    HeartbeatConfig, IMessageConfig, MatrixConfig, McpConfig, MemoryConfig, ObservabilityConfig,
    RuntimeConfig, SecretsConfig, SlackConfig, SurrealMemoryConfig, TelegramConfig, WebhookConfig,
};
use crate::hardware::{self, HardwareConfig};
use crate::memory::{
    default_memory_backend_key, memory_backend_profile, selectable_memory_backends,
};
use crate::providers::{
    canonical_china_provider_name, is_glm_alias, is_glm_cn_alias, is_minimax_alias,
    is_moonshot_alias, is_qianfan_alias, is_qwen_alias, is_zai_alias, is_zai_cn_alias,
};
use anyhow::{bail, Context, Result};
use console::style;
use dialoguer::{Confirm, Input, Password, Select};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

// ── Project context collected during wizard ──────────────────────

/// User-provided personalization baked into workspace MD files.
#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    pub user_name: String,
    pub timezone: String,
    pub agent_name: String,
    pub communication_style: String,
}

// ── Banner ───────────────────────────────────────────────────────

const BANNER: &str = r"
    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡

      ██████╗ ██████╗ ██████╗ ██╗   ██╗██╗   ██╗███████╗
     ██╔════╝██╔═══██╗██╔══██╗██║   ██║██║   ██║██╔════╝
     ██║     ██║   ██║██████╔╝██║   ██║██║   ██║███████╗
     ██║     ██║   ██║██╔══██╗╚██╗ ██╔╝██║   ██║╚════██║
     ╚██████╗╚██████╔╝██║  ██║ ╚████╔╝ ╚██████╔╝███████║
      ╚═════╝ ╚═════╝ ╚═╝  ╚═╝  ╚═══╝   ╚═════╝ ╚══════╝

    Zero overhead. Zero compromise. 100% Rust. 100% Agnostic.

    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡
";

const LIVE_MODEL_MAX_OPTIONS: usize = 120;
const MODEL_PREVIEW_LIMIT: usize = 20;
const MODEL_CACHE_FILE: &str = "models_cache.json";
const MODEL_CACHE_TTL_SECS: u64 = 12 * 60 * 60;
const CUSTOM_MODEL_SENTINEL: &str = "__custom_model__";
const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const COPILOT_DEFAULT_API: &str = "https://api.githubcopilot.com";

const COPILOT_DISCOVERY_HEADERS: [(&str, &str); 4] = [
    ("Editor-Version", "vscode/1.85.1"),
    ("Editor-Plugin-Version", "copilot/1.155.0"),
    ("User-Agent", "GithubCopilot/1.155.0"),
    ("Accept", "application/json"),
];

const DASHBOARD_ACTIVATION_PROMPT: &str = "Activate web dashboard now? (optional)";
const DASHBOARD_GATEWAY_URL: &str = "http://127.0.0.1:3000";
const DASHBOARD_UI_URL: &str = "http://localhost:4324";
const DASHBOARD_DIAG_REQUEST_TIMEOUT_MS: u64 = 500;
const DASHBOARD_DIAG_RETRY_DELAY_MS: u64 = 150;
const DASHBOARD_DIAG_MAX_ATTEMPTS: usize = 2;
const DASHBOARD_DIAG_MAX_BUDGET_MS: u64 = 1_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardActivationDecision {
    Accept,
    Decline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardActivationStatus {
    GatewayNotRunning,
    GatewayRunningPairingRequired,
    GatewayRunningAlreadyPaired,
    DashboardUiUnavailable,
    UnknownLocalFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserOpenResult {
    Opened,
    Unsupported,
    FailedNonFatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DashboardActivationResult {
    status: DashboardActivationStatus,
    browser_open_result: BrowserOpenResult,
}

impl DashboardActivationStatus {
    fn code_and_label(self) -> &'static str {
        match self {
            Self::GatewayNotRunning => "DASH-001 GatewayNotRunning",
            Self::GatewayRunningPairingRequired => "DASH-002 GatewayRunningPairingRequired",
            Self::GatewayRunningAlreadyPaired => "DASH-003 GatewayRunningAlreadyPaired",
            Self::DashboardUiUnavailable => "DASH-004 DashboardUiUnavailable",
            Self::UnknownLocalFailure => "DASH-999 UnknownLocalFailure",
        }
    }

    fn cause(self) -> &'static str {
        match self {
            Self::GatewayNotRunning => {
                "Local gateway health probe did not succeed within bounded retries."
            }
            Self::GatewayRunningPairingRequired => {
                "Local gateway health probe succeeded; continue in secure /pair flow."
            }
            Self::GatewayRunningAlreadyPaired => {
                "Gateway reported available and already paired for this environment."
            }
            Self::DashboardUiUnavailable => {
                "Dashboard UI could not be opened from this runtime environment."
            }
            Self::UnknownLocalFailure => {
                "Local diagnostics encountered an unexpected error; use safe manual commands."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayProbeOutcome {
    HealthyPaired,
    HealthyUnpaired,
    Unreachable,
    ProbeError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayProbeAttempt {
    HealthyPaired,
    HealthyUnpaired,
    RetryableFailure,
    FatalError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardingPostSummaryStage {
    PrintSummary,
    OfferChannelLaunch,
    OfferDashboardActivation,
}

fn onboarding_post_summary_stage_order(
    has_channels: bool,
    has_api_key: bool,
) -> Vec<OnboardingPostSummaryStage> {
    let mut stages = vec![OnboardingPostSummaryStage::PrintSummary];

    if has_channels && has_api_key {
        stages.push(OnboardingPostSummaryStage::OfferChannelLaunch);
    }

    stages.push(OnboardingPostSummaryStage::OfferDashboardActivation);
    stages
}

fn prompt_dashboard_activation_decision() -> Result<DashboardActivationDecision> {
    let activate = Confirm::new()
        .with_prompt(format!(
            "  {} {}",
            style("🌐").cyan(),
            DASHBOARD_ACTIVATION_PROMPT
        ))
        .default(false)
        .interact()?;

    Ok(if activate {
        DashboardActivationDecision::Accept
    } else {
        DashboardActivationDecision::Decline
    })
}

fn map_gateway_probe_to_status(outcome: GatewayProbeOutcome) -> DashboardActivationStatus {
    match outcome {
        GatewayProbeOutcome::HealthyPaired => {
            DashboardActivationStatus::GatewayRunningAlreadyPaired
        }
        GatewayProbeOutcome::HealthyUnpaired => {
            DashboardActivationStatus::GatewayRunningPairingRequired
        }
        GatewayProbeOutcome::Unreachable => DashboardActivationStatus::GatewayNotRunning,
        GatewayProbeOutcome::ProbeError => DashboardActivationStatus::UnknownLocalFailure,
    }
}

fn dashboard_diagnosis_budget_ms() -> u64 {
    let request_budget = DASHBOARD_DIAG_REQUEST_TIMEOUT_MS * DASHBOARD_DIAG_MAX_ATTEMPTS as u64;
    let retry_budget = DASHBOARD_DIAG_RETRY_DELAY_MS * (DASHBOARD_DIAG_MAX_ATTEMPTS as u64 - 1);
    request_budget + retry_budget
}

fn run_gateway_probe_attempts<F, S>(mut attempt: F, mut sleep: S) -> GatewayProbeOutcome
where
    F: FnMut() -> GatewayProbeAttempt,
    S: FnMut(Duration),
{
    for idx in 0..DASHBOARD_DIAG_MAX_ATTEMPTS {
        match attempt() {
            GatewayProbeAttempt::HealthyPaired => return GatewayProbeOutcome::HealthyPaired,
            GatewayProbeAttempt::HealthyUnpaired => return GatewayProbeOutcome::HealthyUnpaired,
            GatewayProbeAttempt::FatalError => return GatewayProbeOutcome::ProbeError,
            GatewayProbeAttempt::RetryableFailure if idx + 1 < DASHBOARD_DIAG_MAX_ATTEMPTS => {
                sleep(Duration::from_millis(DASHBOARD_DIAG_RETRY_DELAY_MS));
            }
            GatewayProbeAttempt::RetryableFailure => return GatewayProbeOutcome::Unreachable,
        }
    }

    GatewayProbeOutcome::Unreachable
}

fn gateway_probe_attempt(
    client: &reqwest::blocking::Client,
    health_url: &str,
) -> GatewayProbeAttempt {
    let response = match client
        .get(health_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
    {
        Ok(response) => response,
        Err(_) => return GatewayProbeAttempt::RetryableFailure,
    };

    let payload = match response.json::<Value>() {
        Ok(payload) => payload,
        Err(_) => return GatewayProbeAttempt::FatalError,
    };

    match payload.get("paired").and_then(Value::as_bool) {
        Some(true) => GatewayProbeAttempt::HealthyPaired,
        Some(false) => GatewayProbeAttempt::HealthyUnpaired,
        None => GatewayProbeAttempt::FatalError,
    }
}

fn probe_gateway_health_bounded() -> GatewayProbeOutcome {
    if dashboard_diagnosis_budget_ms() > DASHBOARD_DIAG_MAX_BUDGET_MS {
        return GatewayProbeOutcome::ProbeError;
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(DASHBOARD_DIAG_REQUEST_TIMEOUT_MS))
        .build()
    {
        Ok(client) => client,
        Err(_) => return GatewayProbeOutcome::ProbeError,
    };

    let health_url = format!("{DASHBOARD_GATEWAY_URL}/health");

    run_gateway_probe_attempts(
        || gateway_probe_attempt(&client, &health_url),
        std::thread::sleep,
    )
}

fn diagnose_dashboard_activation(
    probe_result: GatewayProbeOutcome,
    browser_open_result: BrowserOpenResult,
) -> DashboardActivationStatus {
    match probe_result {
        GatewayProbeOutcome::Unreachable => DashboardActivationStatus::GatewayNotRunning,
        GatewayProbeOutcome::ProbeError => DashboardActivationStatus::UnknownLocalFailure,
        GatewayProbeOutcome::HealthyPaired | GatewayProbeOutcome::HealthyUnpaired => {
            if matches!(
                browser_open_result,
                BrowserOpenResult::Unsupported | BrowserOpenResult::FailedNonFatal
            ) {
                DashboardActivationStatus::DashboardUiUnavailable
            } else {
                map_gateway_probe_to_status(probe_result)
            }
        }
    }
}

fn attempt_open_dashboard_ui() -> BrowserOpenResult {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(DASHBOARD_UI_URL);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", DASHBOARD_UI_URL]);
        command
    } else if cfg!(target_os = "linux") {
        let mut command = Command::new("xdg-open");
        command.arg(DASHBOARD_UI_URL);
        command
    } else {
        return BrowserOpenResult::Unsupported;
    };

    match command.status() {
        Ok(status) if status.success() => BrowserOpenResult::Opened,
        Ok(_) => BrowserOpenResult::FailedNonFatal,
        Err(err) if err.kind() == ErrorKind::NotFound => BrowserOpenResult::Unsupported,
        Err(_) => BrowserOpenResult::FailedNonFatal,
    }
}

fn run_dashboard_activation() -> DashboardActivationResult {
    let browser_open_result = attempt_open_dashboard_ui();
    let gateway_probe = probe_gateway_health_bounded();
    let status = diagnose_dashboard_activation(gateway_probe, browser_open_result);

    DashboardActivationResult {
        status,
        browser_open_result,
    }
}

fn dashboard_resume_later_output_lines() -> Vec<String> {
    vec![
        "1. Check status: corvus status".to_string(),
        "2. Start gateway: corvus gateway".to_string(),
        "3. Start dashboard UI: make dashboard-dev".to_string(),
        format!("4. Open {DASHBOARD_UI_URL} and complete secure pairing at /pair."),
        "5. Need command help: corvus --help".to_string(),
    ]
}

fn dashboard_status_fallback_lines(status: DashboardActivationStatus) -> Vec<String> {
    match status {
        DashboardActivationStatus::GatewayNotRunning => vec![
            "- Start gateway: corvus gateway".to_string(),
            "- Start dashboard UI (if needed): make dashboard-dev".to_string(),
            format!("- Open dashboard: {DASHBOARD_UI_URL}"),
        ],
        DashboardActivationStatus::GatewayRunningPairingRequired => vec![
            format!("- Open dashboard: {DASHBOARD_UI_URL}"),
            "- Complete secure pairing using the /pair flow.".to_string(),
        ],
        DashboardActivationStatus::GatewayRunningAlreadyPaired => {
            vec![
                format!("- Open dashboard: {DASHBOARD_UI_URL}"),
                "- Continue to dashboard configuration view.".to_string(),
            ]
        }
        DashboardActivationStatus::DashboardUiUnavailable => vec![
            "- Start dashboard UI: make dashboard-dev".to_string(),
            format!("- Then open: {DASHBOARD_UI_URL}"),
        ],
        DashboardActivationStatus::UnknownLocalFailure => vec![
            "- Check status: corvus status".to_string(),
            "- Run diagnostics: corvus doctor".to_string(),
            "- Start gateway: corvus gateway".to_string(),
            "- Start dashboard UI: make dashboard-dev".to_string(),
        ],
    }
}

#[derive(Debug, Clone)]
struct DashboardActivationOutput {
    guide_lines: Vec<String>,
    detail_lines: Vec<String>,
}

fn browser_open_result_line(browser_open_result: BrowserOpenResult) -> String {
    match browser_open_result {
        BrowserOpenResult::Opened => format!("Browser open: launched {DASHBOARD_UI_URL}."),
        BrowserOpenResult::Unsupported => {
            format!("Browser open: unsupported in this runtime; open {DASHBOARD_UI_URL} manually.")
        }
        BrowserOpenResult::FailedNonFatal => {
            format!("Browser open: failed non-fatally; open {DASHBOARD_UI_URL} manually.")
        }
    }
}

fn render_dashboard_activation_output(
    decision: DashboardActivationDecision,
    status: DashboardActivationStatus,
    browser_open_result: BrowserOpenResult,
) -> DashboardActivationOutput {
    if decision == DashboardActivationDecision::Decline {
        return DashboardActivationOutput {
            guide_lines: Vec::new(),
            detail_lines: dashboard_resume_later_output_lines(),
        };
    }

    let guide_lines = vec![
        format!("1. Verify gateway is reachable at {DASHBOARD_GATEWAY_URL}."),
        format!("2. Open dashboard UI at {DASHBOARD_UI_URL}."),
        "3. Complete secure pairing via /pair when prompted.".to_string(),
        "4. If dashboard UI is not running, start it with: make dashboard-dev".to_string(),
    ];

    let mut detail_lines = vec![browser_open_result_line(browser_open_result)];
    detail_lines.push(format!(
        "Dashboard activation status: {}",
        status.code_and_label()
    ));
    detail_lines.push(format!("Cause: {}", status.cause()));
    detail_lines.push("Fallback commands:".to_string());
    detail_lines.extend(dashboard_status_fallback_lines(status));
    detail_lines.push("Resume later:".to_string());
    detail_lines.extend(dashboard_resume_later_output_lines());

    DashboardActivationOutput {
        guide_lines,
        detail_lines,
    }
}

fn dashboard_activation_output_lines_for_result(
    decision: DashboardActivationDecision,
    activation_result: DashboardActivationResult,
) -> Vec<String> {
    let output = render_dashboard_activation_output(
        decision,
        activation_result.status,
        activation_result.browser_open_result,
    );

    if output.guide_lines.is_empty() {
        return output.detail_lines;
    }

    let mut lines = output.guide_lines;
    lines.extend(output.detail_lines);
    lines
}

fn dashboard_activation_output_lines(decision: DashboardActivationDecision) -> Vec<String> {
    match decision {
        DashboardActivationDecision::Decline => dashboard_resume_later_output_lines(),
        DashboardActivationDecision::Accept => {
            let activation_result = run_dashboard_activation();
            dashboard_activation_output_lines_for_result(decision, activation_result)
        }
    }
}

fn print_dashboard_activation_output(decision: DashboardActivationDecision) {
    let lines = dashboard_activation_output_lines(decision);
    if lines.is_empty() {
        return;
    }

    println!();
    let heading = match decision {
        DashboardActivationDecision::Accept => "Web dashboard activation (optional):",
        DashboardActivationDecision::Decline => "Resume web dashboard later:",
    };

    println!("  {}", style(heading).white().bold());
    for line in lines {
        println!("    {line}");
    }
    println!();
}

#[derive(Debug, Deserialize)]
struct CopilotApiEndpoints {
    api: Option<String>,
}

#[derive(Deserialize)]
struct CopilotApiKeyInfo {
    token: String,
    #[serde(default)]
    endpoints: Option<CopilotApiEndpoints>,
}

// ── Main wizard entry point ──────────────────────────────────────

pub fn run_wizard() -> Result<Config> {
    println!("{}", style(BANNER).cyan().bold());

    println!(
        "  {}",
        style("Welcome to Corvus — the fastest, smallest AI assistant.")
            .white()
            .bold()
    );
    println!(
        "  {}",
        style("This wizard will configure your agent in under 60 seconds.").dim()
    );
    println!();

    print_step(1, 9, "Workspace Setup");
    let (workspace_dir, config_path) = setup_workspace()?;

    print_step(2, 9, "AI Provider & API Key");
    let (provider, api_key, model, provider_api_url) = setup_provider(&workspace_dir)?;

    print_step(3, 9, "Channels (How You Talk to Corvus)");
    let channels_config = setup_channels()?;

    print_step(4, 9, "Tunnel (Expose to Internet)");
    let tunnel_config = setup_tunnel()?;

    print_step(5, 9, "Tool Mode & Security");
    let (composio_config, secrets_config) = setup_tool_mode()?;

    print_step(6, 9, "Hardware (Physical World)");
    let hardware_config = setup_hardware()?;

    print_step(7, 9, "Memory Configuration");
    let memory_config = setup_memory()?;

    print_step(8, 9, "Project Context (Personalize Your Agent)");
    let project_ctx = setup_project_context()?;

    print_step(9, 9, "Workspace Files");
    scaffold_workspace(&workspace_dir, &project_ctx)?;

    // ── Build config ──
    // Defaults: SQLite memory, supervised autonomy, workspace-scoped, native runtime
    let config = Config {
        workspace_dir: workspace_dir.clone(),
        config_path: config_path.clone(),
        api_key: if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        },
        api_url: provider_api_url,
        default_provider: Some(provider),
        default_model: Some(model),
        default_temperature: 0.7,
        observability: ObservabilityConfig::default(),
        autonomy: AutonomyConfig::default(),
        runtime: RuntimeConfig::default(),
        reliability: crate::config::ReliabilityConfig::default(),
        scheduler: crate::config::schema::SchedulerConfig::default(),
        agent: crate::config::schema::AgentConfig::default(),
        mission: crate::config::MissionConfig::default(),
        model_routes: Vec::new(),
        heartbeat: HeartbeatConfig::default(),
        cron: crate::config::CronConfig::default(),
        channels_config,
        updates: crate::config::UpdateConfig::default(),
        memory: memory_config, // User-selected memory backend
        tunnel: tunnel_config,
        gateway: crate::config::GatewayConfig::default(),
        composio: composio_config,
        secrets: secrets_config,
        browser: BrowserConfig::default(),
        http_request: crate::config::HttpRequestConfig::default(),
        web_search: crate::config::WebSearchConfig::default(),
        mcp: McpConfig::default(),
        identity: crate::config::IdentityConfig::default(),
        cost: crate::config::CostConfig::default(),
        peripherals: crate::config::PeripheralsConfig::default(),
        agents: std::collections::HashMap::new(),
        hardware: hardware_config,
        query_classification: crate::config::QueryClassificationConfig::default(),
    };

    println!(
        "  {} Security: {} | workspace-scoped",
        style("✓").green().bold(),
        style("Supervised").green()
    );
    println!(
        "  {} Memory: {} (auto-save: {})",
        style("✓").green().bold(),
        style(&config.memory.backend).green(),
        if config.memory.auto_save { "on" } else { "off" }
    );

    config.save()?;
    persist_workspace_selection(&config.config_path)?;

    // ── Final summary ────────────────────────────────────────────
    print_summary(&config);

    // ── Offer to launch channels immediately ─────────────────────
    let has_channels = config.channels_config.telegram.is_some()
        || config.channels_config.discord.is_some()
        || config.channels_config.slack.is_some()
        || config.channels_config.imessage.is_some()
        || config.channels_config.matrix.is_some()
        || config.channels_config.email.is_some()
        || config.channels_config.dingtalk.is_some()
        || config.channels_config.qq.is_some();

    let post_summary_stages =
        onboarding_post_summary_stage_order(has_channels, config.api_key.is_some());

    if post_summary_stages.contains(&OnboardingPostSummaryStage::OfferChannelLaunch) {
        let launch: bool = Confirm::new()
            .with_prompt(format!(
                "  {} Launch channels now? (connected channels → AI → reply)",
                style("🚀").cyan()
            ))
            .default(true)
            .interact()?;

        if launch {
            println!();
            println!(
                "  {} {}",
                style("⚡").cyan(),
                style("Starting channel server...").white().bold()
            );
            println!();
            // Signal to main.rs to call start_channels after wizard returns
            std::env::set_var("CORVUS_AUTOSTART_CHANNELS", "1");
        }
    }

    if post_summary_stages.contains(&OnboardingPostSummaryStage::OfferDashboardActivation) {
        let dashboard_activation = prompt_dashboard_activation_decision()?;
        print_dashboard_activation_output(dashboard_activation);
    }

    Ok(config)
}

/// Interactive repair flow: rerun channel setup only without redoing full onboarding.
pub fn run_channels_repair_wizard() -> Result<Config> {
    println!("{}", style(BANNER).cyan().bold());
    println!(
        "  {}",
        style("Channels Repair — update channel tokens and allowlists only")
            .white()
            .bold()
    );
    println!();

    let mut config = Config::load_or_init()?;

    print_step(1, 1, "Channels (How You Talk to Corvus)");
    config.channels_config = setup_channels()?;
    config.save()?;
    persist_workspace_selection(&config.config_path)?;

    println!();
    println!(
        "  {} Channel config saved: {}",
        style("✓").green().bold(),
        style(config.config_path.display()).green()
    );

    let has_channels = config.channels_config.telegram.is_some()
        || config.channels_config.discord.is_some()
        || config.channels_config.slack.is_some()
        || config.channels_config.imessage.is_some()
        || config.channels_config.matrix.is_some()
        || config.channels_config.email.is_some()
        || config.channels_config.dingtalk.is_some()
        || config.channels_config.qq.is_some();

    if has_channels && config.api_key.is_some() {
        let launch: bool = Confirm::new()
            .with_prompt(format!(
                "  {} Launch channels now? (connected channels → AI → reply)",
                style("🚀").cyan()
            ))
            .default(true)
            .interact()?;

        if launch {
            println!();
            println!(
                "  {} {}",
                style("⚡").cyan(),
                style("Starting channel server...").white().bold()
            );
            println!();
            // Signal to main.rs to call start_channels after wizard returns
            std::env::set_var("CORVUS_AUTOSTART_CHANNELS", "1");
        }
    }

    Ok(config)
}

// ── Quick setup (zero prompts) ───────────────────────────────────

/// Non-interactive setup: generates a sensible default config instantly.
/// Use `corvus onboard` or
/// `corvus onboard --api-key sk-... --provider openrouter --memory sqlite|lucid|surreal-graphs`.
/// Use `corvus onboard --interactive` for the full wizard.
fn backend_key_from_choice(choice: usize) -> &'static str {
    selectable_memory_backends()
        .get(choice)
        .map_or(default_memory_backend_key(), |backend| backend.key)
}

fn memory_config_defaults_for_backend(backend: &str) -> MemoryConfig {
    let profile = memory_backend_profile(backend);

    MemoryConfig {
        backend: backend.to_string(),
        auto_save: profile.auto_save_default,
        hygiene_enabled: profile.uses_sqlite_hygiene,
        archive_after_days: if profile.uses_sqlite_hygiene { 7 } else { 0 },
        purge_after_days: if profile.uses_sqlite_hygiene { 30 } else { 0 },
        conversation_retention_days: 30,
        embedding_provider: "none".to_string(),
        embedding_model: "text-embedding-3-small".to_string(),
        embedding_dimensions: 1536,
        vector_weight: 0.7,
        keyword_weight: 0.3,
        min_relevance_score: 0.4,
        embedding_cache_size: if profile.uses_sqlite_hygiene {
            10000
        } else {
            0
        },
        chunk_max_tokens: 512,
        response_cache_enabled: false,
        response_cache_ttl_minutes: 60,
        response_cache_max_entries: 5_000,
        snapshot_enabled: false,
        snapshot_on_hygiene: false,
        auto_hydrate: true,
        sqlite_open_timeout_secs: None,
        surreal: SurrealMemoryConfig::default(),
    }
}

#[allow(clippy::too_many_lines)]
pub fn run_quick_setup(
    credential_override: Option<&str>,
    provider: Option<&str>,
    memory_backend: Option<&str>,
) -> Result<Config> {
    println!("{}", style(BANNER).cyan().bold());
    println!(
        "  {}",
        style("Quick Setup — generating config with sensible defaults...")
            .white()
            .bold()
    );
    println!();

    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let corvus_dir = home.join(".corvus");
    let workspace_dir = corvus_dir.join("workspace");
    let config_path = corvus_dir.join("config.toml");

    fs::create_dir_all(&workspace_dir).context("Failed to create workspace directory")?;

    let provider_name = provider.unwrap_or("openrouter").to_string();
    let model = default_model_for_provider(&provider_name);
    let memory_backend_name = memory_backend
        .unwrap_or(default_memory_backend_key())
        .to_string();
    #[cfg(not(feature = "memory-surreal"))]
    if memory_backend_name == "surreal" {
        bail!("memory backend 'surreal' requires a binary built with feature 'memory-surreal'");
    }

    // Create memory config based on backend choice
    let memory_config = memory_config_defaults_for_backend(&memory_backend_name);

    let config = Config {
        workspace_dir: workspace_dir.clone(),
        config_path: config_path.clone(),
        api_key: credential_override.map(String::from),
        api_url: None,
        default_provider: Some(provider_name.clone()),
        default_model: Some(model.clone()),
        default_temperature: 0.7,
        observability: ObservabilityConfig::default(),
        autonomy: AutonomyConfig::default(),
        runtime: RuntimeConfig::default(),
        reliability: crate::config::ReliabilityConfig::default(),
        scheduler: crate::config::schema::SchedulerConfig::default(),
        agent: crate::config::schema::AgentConfig::default(),
        mission: crate::config::MissionConfig::default(),
        model_routes: Vec::new(),
        heartbeat: HeartbeatConfig::default(),
        cron: crate::config::CronConfig::default(),
        channels_config: ChannelsConfig::default(),
        updates: crate::config::UpdateConfig::default(),
        memory: memory_config,
        tunnel: crate::config::TunnelConfig::default(),
        gateway: crate::config::GatewayConfig::default(),
        composio: ComposioConfig::default(),
        secrets: SecretsConfig::default(),
        browser: BrowserConfig::default(),
        http_request: crate::config::HttpRequestConfig::default(),
        web_search: crate::config::WebSearchConfig::default(),
        mcp: McpConfig::default(),
        identity: crate::config::IdentityConfig::default(),
        cost: crate::config::CostConfig::default(),
        peripherals: crate::config::PeripheralsConfig::default(),
        agents: std::collections::HashMap::new(),
        hardware: crate::config::HardwareConfig::default(),
        query_classification: crate::config::QueryClassificationConfig::default(),
    };

    config.save()?;
    persist_workspace_selection(&config.config_path)?;

    // Scaffold minimal workspace files
    let default_ctx = ProjectContext {
        user_name: std::env::var("USER").unwrap_or_else(|_| "User".into()),
        timezone: "UTC".into(),
        agent_name: "Corvus".into(),
        communication_style:
            "Be warm, natural, and clear. Use occasional relevant emojis (1-2 max) and avoid robotic phrasing."
                .into(),
    };
    scaffold_workspace(&workspace_dir, &default_ctx)?;

    println!(
        "  {} Workspace:  {}",
        style("✓").green().bold(),
        style(workspace_dir.display()).green()
    );
    println!(
        "  {} Provider:   {}",
        style("✓").green().bold(),
        style(&provider_name).green()
    );
    println!(
        "  {} Model:      {}",
        style("✓").green().bold(),
        style(&model).green()
    );
    println!(
        "  {} API Key:    {}",
        style("✓").green().bold(),
        if credential_override.is_some() {
            style("set").green()
        } else {
            style("not set (use --api-key or edit config.toml)").yellow()
        }
    );
    println!(
        "  {} Security:   {}",
        style("✓").green().bold(),
        style("Supervised (workspace-scoped)").green()
    );
    println!(
        "  {} Memory:     {} (auto-save: {})",
        style("✓").green().bold(),
        style(&config.memory.backend).green(),
        if config.memory.backend == "none" {
            "off"
        } else {
            "on"
        }
    );
    println!(
        "  {} Secrets:    {}",
        style("✓").green().bold(),
        style("encrypted").green()
    );
    println!(
        "  {} Gateway:    {}",
        style("✓").green().bold(),
        style("pairing required (127.0.0.1:8080)").green()
    );
    println!(
        "  {} Tunnel:     {}",
        style("✓").green().bold(),
        style("none (local only)").dim()
    );
    println!(
        "  {} Composio:   {}",
        style("✓").green().bold(),
        style("disabled (sovereign mode)").dim()
    );
    println!();
    println!(
        "  {} {}",
        style("Config saved:").white().bold(),
        style(config_path.display()).green()
    );
    println!();
    println!("  {}", style("Next steps:").white().bold());
    if credential_override.is_none() {
        println!("    1. Set your API key:  export OPENROUTER_API_KEY=\"sk-...\"");
        println!("    2. Or edit:           ~/.corvus/config.toml");
        println!("    3. Chat:              corvus agent -m \"Hello!\"");
        println!("    4. Gateway:           corvus gateway");
    } else {
        println!("    1. Chat:     corvus agent -m \"Hello!\"");
        println!("    2. Gateway:  corvus gateway");
        println!("    3. Status:   corvus status");
    }
    println!();

    Ok(config)
}

fn canonical_provider_name(provider_name: &str) -> &str {
    if let Some(canonical) = canonical_china_provider_name(provider_name) {
        return canonical;
    }

    match provider_name {
        "github-copilot" => "copilot",
        "grok" => "xai",
        "together" => "together-ai",
        "google" | "google-gemini" => "gemini",
        _ => provider_name,
    }
}

fn models_for_provider(canonical_provider: &str) -> Vec<(String, String)> {
    curated_models_for_provider(canonical_provider)
}
/// Pick a sensible default model for the given provider.
const MINIMAX_ONBOARD_MODELS: [(&str, &str); 5] = [
    ("MiniMax-M2.5", "MiniMax M2.5 (latest, recommended)"),
    ("MiniMax-M2.5-highspeed", "MiniMax M2.5 High-Speed (faster)"),
    ("MiniMax-M2.1", "MiniMax M2.1 (stable)"),
    ("MiniMax-M2.1-highspeed", "MiniMax M2.1 High-Speed (faster)"),
    ("MiniMax-M2", "MiniMax M2 (legacy)"),
];

fn default_model_for_provider(provider: &str) -> String {
    match canonical_provider_name(provider) {
        "anthropic" => "claude-sonnet-4-5-20250929".into(),
        "openai" => "gpt-5.2".into(),
        "glm" | "zai" => "glm-5".into(),
        "minimax" => "MiniMax-M2.5".into(),
        "qwen" => "qwen-plus".into(),
        "ollama" => "llama3.2".into(),
        "groq" => "llama-3.3-70b-versatile".into(),
        "deepseek" => "deepseek-chat".into(),
        "gemini" => "gemini-2.5-pro".into(),
        "copilot" => "gpt-4o".into(),
        _ => "anthropic/claude-sonnet-4.5".into(),
    }
}

fn curated_models_for_provider(provider_name: &str) -> Vec<(String, String)> {
    match canonical_provider_name(provider_name) {
        "openrouter" => vec![
            (
                "anthropic/claude-sonnet-4.5".to_string(),
                "Claude Sonnet 4.5 (balanced, recommended)".to_string(),
            ),
            (
                "openai/gpt-5.2".to_string(),
                "GPT-5.2 (latest flagship)".to_string(),
            ),
            (
                "openai/gpt-5-mini".to_string(),
                "GPT-5 mini (fast, cost-efficient)".to_string(),
            ),
            (
                "google/gemini-3-pro-preview".to_string(),
                "Gemini 3 Pro Preview (frontier reasoning)".to_string(),
            ),
            (
                "x-ai/grok-4.1-fast".to_string(),
                "Grok 4.1 Fast (reasoning + speed)".to_string(),
            ),
            (
                "deepseek/deepseek-v3.2".to_string(),
                "DeepSeek V3.2 (agentic + affordable)".to_string(),
            ),
            (
                "meta-llama/llama-4-maverick".to_string(),
                "Llama 4 Maverick (open model)".to_string(),
            ),
        ],
        "anthropic" => vec![
            (
                "claude-sonnet-4-5-20250929".to_string(),
                "Claude Sonnet 4.5 (balanced, recommended)".to_string(),
            ),
            (
                "claude-opus-4-6".to_string(),
                "Claude Opus 4.6 (best quality)".to_string(),
            ),
            (
                "claude-haiku-4-5-20251001".to_string(),
                "Claude Haiku 4.5 (fastest, cheapest)".to_string(),
            ),
        ],
        "openai" => vec![
            (
                "gpt-5.2".to_string(),
                "GPT-5.2 (latest coding/agentic flagship)".to_string(),
            ),
            (
                "gpt-5-mini".to_string(),
                "GPT-5 mini (faster, cheaper)".to_string(),
            ),
            (
                "gpt-5-nano".to_string(),
                "GPT-5 nano (lowest latency/cost)".to_string(),
            ),
            (
                "gpt-5.2-codex".to_string(),
                "GPT-5.2 Codex (agentic coding)".to_string(),
            ),
        ],
        "venice" => vec![
            (
                "llama-3.3-70b".to_string(),
                "Llama 3.3 70B (default, fast)".to_string(),
            ),
            (
                "claude-opus-45".to_string(),
                "Claude Opus 4.5 via Venice (strongest)".to_string(),
            ),
            (
                "llama-3.1-405b".to_string(),
                "Llama 3.1 405B (largest open source)".to_string(),
            ),
        ],
        "groq" => vec![
            (
                "llama-3.3-70b-versatile".to_string(),
                "Llama 3.3 70B (fast, recommended)".to_string(),
            ),
            (
                "openai/gpt-oss-120b".to_string(),
                "GPT-OSS 120B (strong open-weight)".to_string(),
            ),
            (
                "openai/gpt-oss-20b".to_string(),
                "GPT-OSS 20B (cost-efficient open-weight)".to_string(),
            ),
        ],
        "mistral" => vec![
            (
                "mistral-large-latest".to_string(),
                "Mistral Large (latest flagship)".to_string(),
            ),
            (
                "mistral-medium-latest".to_string(),
                "Mistral Medium (balanced)".to_string(),
            ),
            (
                "codestral-latest".to_string(),
                "Codestral (code-focused)".to_string(),
            ),
            (
                "devstral-latest".to_string(),
                "Devstral (software engineering specialist)".to_string(),
            ),
        ],
        "deepseek" => vec![
            (
                "deepseek-chat".to_string(),
                "DeepSeek Chat (mapped to V3.2 non-thinking)".to_string(),
            ),
            (
                "deepseek-reasoner".to_string(),
                "DeepSeek Reasoner (mapped to V3.2 thinking)".to_string(),
            ),
        ],
        "xai" => vec![
            (
                "grok-4-1-fast-reasoning".to_string(),
                "Grok 4.1 Fast Reasoning (recommended)".to_string(),
            ),
            (
                "grok-4-1-fast-non-reasoning".to_string(),
                "Grok 4.1 Fast Non-Reasoning (low latency)".to_string(),
            ),
            (
                "grok-code-fast-1".to_string(),
                "Grok Code Fast 1 (coding specialist)".to_string(),
            ),
            ("grok-4".to_string(), "Grok 4 (max quality)".to_string()),
        ],
        "perplexity" => vec![
            (
                "sonar-pro".to_string(),
                "Sonar Pro (flagship web-grounded model)".to_string(),
            ),
            (
                "sonar-reasoning-pro".to_string(),
                "Sonar Reasoning Pro (complex multi-step reasoning)".to_string(),
            ),
            (
                "sonar-deep-research".to_string(),
                "Sonar Deep Research (long-form research)".to_string(),
            ),
            ("sonar".to_string(), "Sonar (search, fast)".to_string()),
        ],
        "fireworks" => vec![
            (
                "accounts/fireworks/models/llama-v3p3-70b-instruct".to_string(),
                "Llama 3.3 70B".to_string(),
            ),
            (
                "accounts/fireworks/models/mixtral-8x22b-instruct".to_string(),
                "Mixtral 8x22B".to_string(),
            ),
        ],
        "together-ai" => vec![
            (
                "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
                "Llama 3.3 70B Instruct Turbo (recommended)".to_string(),
            ),
            (
                "moonshotai/Kimi-K2.5".to_string(),
                "Kimi K2.5 (reasoning + coding)".to_string(),
            ),
            (
                "deepseek-ai/DeepSeek-V3.1".to_string(),
                "DeepSeek V3.1 (strong value)".to_string(),
            ),
        ],
        "cohere" => vec![
            (
                "command-a-03-2025".to_string(),
                "Command A (flagship enterprise model)".to_string(),
            ),
            (
                "command-a-reasoning-08-2025".to_string(),
                "Command A Reasoning (agentic reasoning)".to_string(),
            ),
            (
                "command-r-08-2024".to_string(),
                "Command R (stable fast baseline)".to_string(),
            ),
        ],
        "moonshot" => vec![
            (
                "kimi-latest".to_string(),
                "Kimi Latest (rolling latest assistant model)".to_string(),
            ),
            (
                "kimi-k2-0905-preview".to_string(),
                "Kimi K2 0905 Preview (strong coding)".to_string(),
            ),
            (
                "kimi-thinking-preview".to_string(),
                "Kimi Thinking Preview (deep reasoning)".to_string(),
            ),
        ],
        "glm" | "zai" => vec![
            (
                "glm-4.7".to_string(),
                "GLM-4.7 (latest flagship)".to_string(),
            ),
            ("glm-5".to_string(), "GLM-5 (high reasoning)".to_string()),
            (
                "glm-4-plus".to_string(),
                "GLM-4 Plus (stable baseline)".to_string(),
            ),
        ],
        "minimax" => vec![
            (
                "MiniMax-M2.5".to_string(),
                "MiniMax M2.5 (latest flagship)".to_string(),
            ),
            (
                "MiniMax-M2.1".to_string(),
                "MiniMax M2.1 (strong coding/reasoning)".to_string(),
            ),
            (
                "MiniMax-M2.1-lightning".to_string(),
                "MiniMax M2.1 Lightning (fast)".to_string(),
            ),
        ],
        "qwen" => vec![
            (
                "qwen-max".to_string(),
                "Qwen Max (highest quality)".to_string(),
            ),
            (
                "qwen-plus".to_string(),
                "Qwen Plus (balanced default)".to_string(),
            ),
            (
                "qwen-turbo".to_string(),
                "Qwen Turbo (fast and cost-efficient)".to_string(),
            ),
        ],
        "ollama" => vec![
            (
                "llama3.2".to_string(),
                "Llama 3.2 (recommended local)".to_string(),
            ),
            ("mistral".to_string(), "Mistral 7B".to_string()),
            ("codellama".to_string(), "Code Llama".to_string()),
            ("phi3".to_string(), "Phi-3 (small, fast)".to_string()),
        ],
        "gemini" => vec![
            (
                "gemini-3-pro-preview".to_string(),
                "Gemini 3 Pro Preview (latest frontier reasoning)".to_string(),
            ),
            (
                "gemini-2.5-pro".to_string(),
                "Gemini 2.5 Pro (stable reasoning)".to_string(),
            ),
            (
                "gemini-2.5-flash".to_string(),
                "Gemini 2.5 Flash (best price/performance)".to_string(),
            ),
            (
                "gemini-2.5-flash-lite".to_string(),
                "Gemini 2.5 Flash-Lite (lowest cost)".to_string(),
            ),
        ],
        "copilot" => vec![
            (
                "gpt-4o".to_string(),
                "GPT-4o (stable default for Copilot)".to_string(),
            ),
            (
                "claude-3.5-sonnet".to_string(),
                "Claude 3.5 Sonnet (strong coding quality)".to_string(),
            ),
            (
                "o3-mini".to_string(),
                "o3-mini (fast reasoning fallback)".to_string(),
            ),
        ],
        _ => vec![("default".to_string(), "Default model".to_string())],
    }
}

fn supports_live_model_fetch(provider_name: &str) -> bool {
    matches!(
        canonical_provider_name(provider_name),
        "openrouter"
            | "openai"
            | "anthropic"
            | "groq"
            | "mistral"
            | "deepseek"
            | "xai"
            | "together-ai"
            | "gemini"
            | "ollama"
            | "copilot"
            | "astrai"
    )
}

fn build_model_fetch_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .context("failed to build model-fetch HTTP client")
}

fn normalize_model_ids(ids: Vec<String>) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for id in ids {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            unique.insert(trimmed.to_string());
        }
    }
    unique.into_iter().collect()
}

fn parse_openai_compatible_model_ids(payload: &Value) -> Vec<String> {
    let mut models = Vec::new();

    if let Some(data) = payload.get("data").and_then(Value::as_array) {
        for model in data {
            if let Some(id) = model.get("id").and_then(Value::as_str) {
                models.push(id.to_string());
            }
        }
    } else if let Some(data) = payload.as_array() {
        for model in data {
            if let Some(id) = model.get("id").and_then(Value::as_str) {
                models.push(id.to_string());
            }
        }
    }

    normalize_model_ids(models)
}

fn parse_gemini_model_ids(payload: &Value) -> Vec<String> {
    let Some(models) = payload.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    for model in models {
        let supports_generate_content = model
            .get("supportedGenerationMethods")
            .and_then(Value::as_array)
            .is_none_or(|methods| {
                methods
                    .iter()
                    .any(|method| method.as_str() == Some("generateContent"))
            });

        if !supports_generate_content {
            continue;
        }

        if let Some(name) = model.get("name").and_then(Value::as_str) {
            ids.push(name.trim_start_matches("models/").to_string());
        }
    }

    normalize_model_ids(ids)
}

fn parse_ollama_model_ids(payload: &Value) -> Vec<String> {
    let Some(models) = payload.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    for model in models {
        if let Some(name) = model.get("name").and_then(Value::as_str) {
            ids.push(name.to_string());
        }
    }

    normalize_model_ids(ids)
}

fn fetch_openai_compatible_models(endpoint: &str, api_key: Option<&str>) -> Result<Vec<String>> {
    let Some(api_key) = api_key else {
        return Ok(Vec::new());
    };

    let client = build_model_fetch_client()?;
    let payload: Value = client
        .get(endpoint)
        .bearer_auth(api_key)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .with_context(|| format!("model fetch failed: GET {endpoint}"))?
        .json()
        .context("failed to parse model list response")?;

    Ok(parse_openai_compatible_model_ids(&payload))
}

fn fetch_openrouter_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let client = build_model_fetch_client()?;
    let mut request = client.get("https://openrouter.ai/api/v1/models");
    if let Some(api_key) = api_key {
        request = request.bearer_auth(api_key);
    }

    let payload: Value = request
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET https://openrouter.ai/api/v1/models")?
        .json()
        .context("failed to parse OpenRouter model list response")?;

    Ok(parse_openai_compatible_model_ids(&payload))
}

fn fetch_anthropic_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let Some(api_key) = api_key else {
        return Ok(Vec::new());
    };

    let client = build_model_fetch_client()?;
    let mut request = client
        .get("https://api.anthropic.com/v1/models")
        .header("anthropic-version", "2023-06-01");

    if api_key.starts_with("sk-ant-oat01-") {
        request = request
            .header("Authorization", format!("Bearer {api_key}"))
            .header("anthropic-beta", "oauth-2025-04-20");
    } else {
        request = request.header("x-api-key", api_key);
    }

    let response = request
        .send()
        .context("model fetch failed: GET https://api.anthropic.com/v1/models")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        bail!("Anthropic model list request failed (HTTP {status}): {body}");
    }

    let payload: Value = response
        .json()
        .context("failed to parse Anthropic model list response")?;

    Ok(parse_openai_compatible_model_ids(&payload))
}

fn fetch_gemini_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let Some(api_key) = api_key else {
        return Ok(Vec::new());
    };

    let client = build_model_fetch_client()?;
    let payload: Value = client
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key), ("pageSize", "200")])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET Gemini models")?
        .json()
        .context("failed to parse Gemini model list response")?;

    Ok(parse_gemini_model_ids(&payload))
}

fn fetch_ollama_models() -> Result<Vec<String>> {
    let client = build_model_fetch_client()?;
    let payload: Value = client
        .get("http://localhost:11434/api/tags")
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET http://localhost:11434/api/tags")?
        .json()
        .context("failed to parse Ollama model list response")?;

    Ok(parse_ollama_model_ids(&payload))
}

fn resolve_live_model_api_key(provider_name: &str, api_key: &str) -> Option<String> {
    if !api_key.trim().is_empty() {
        return Some(api_key.trim().to_string());
    }

    std::env::var(provider_env_var(provider_name))
        .ok()
        .or_else(|| {
            // Anthropic also accepts OAuth setup-tokens via ANTHROPIC_OAUTH_TOKEN
            if provider_name == "anthropic" {
                std::env::var("ANTHROPIC_OAUTH_TOKEN").ok()
            } else if provider_name == "copilot" {
                // GitHub CLI commonly exposes GH_TOKEN
                std::env::var("GH_TOKEN").ok()
            } else {
                None
            }
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn fetch_copilot_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let github_token = api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("GitHub token is required for Copilot model discovery"))?;

    let client = build_model_fetch_client()?;

    let mut token_request = client
        .get(COPILOT_TOKEN_URL)
        .header("Authorization", format!("token {github_token}"));
    for (header, value) in COPILOT_DISCOVERY_HEADERS {
        token_request = token_request.header(header, value);
    }

    let api_key_info: CopilotApiKeyInfo = token_request
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET Copilot API token")?
        .json()
        .context("failed to parse Copilot API token response")?;

    let endpoint = api_key_info
        .endpoints
        .and_then(|endpoints| endpoints.api)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| COPILOT_DEFAULT_API.to_string());

    let mut model_request = client
        .get(format!("{}/models", endpoint.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key_info.token));
    for (header, value) in COPILOT_DISCOVERY_HEADERS {
        model_request = model_request.header(header, value);
    }

    let payload: Value = model_request
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET Copilot models")?
        .json()
        .context("failed to parse Copilot model list response")?;

    Ok(parse_openai_compatible_model_ids(&payload))
}

fn fetch_live_models_for_provider(provider_name: &str, api_key: &str) -> Result<Vec<String>> {
    let provider_name = canonical_provider_name(provider_name);
    let api_key = resolve_live_model_api_key(provider_name, api_key);

    let models = match provider_name {
        "openrouter" => fetch_openrouter_models(api_key.as_deref())?,
        "openai" => {
            fetch_openai_compatible_models("https://api.openai.com/v1/models", api_key.as_deref())?
        }
        "groq" => fetch_openai_compatible_models(
            "https://api.groq.com/openai/v1/models",
            api_key.as_deref(),
        )?,
        "mistral" => {
            fetch_openai_compatible_models("https://api.mistral.ai/v1/models", api_key.as_deref())?
        }
        "deepseek" => fetch_openai_compatible_models(
            "https://api.deepseek.com/v1/models",
            api_key.as_deref(),
        )?,
        "xai" => fetch_openai_compatible_models("https://api.x.ai/v1/models", api_key.as_deref())?,
        "together-ai" => fetch_openai_compatible_models(
            "https://api.together.xyz/v1/models",
            api_key.as_deref(),
        )?,
        "anthropic" => fetch_anthropic_models(api_key.as_deref())?,
        "gemini" => fetch_gemini_models(api_key.as_deref())?,
        "ollama" => {
            if api_key.as_deref().map_or(true, |k| k.trim().is_empty()) {
                // Key is None or empty, assume local Ollama
                fetch_ollama_models()?
            } else {
                // Key is present, assume Ollama Cloud and return hardcoded list
                vec![
                    "glm-5:cloud".to_string(),
                    "glm-4.7:cloud".to_string(),
                    "gpt-oss:cloud".to_string(),
                    "gemini-3-flash-preview:cloud".to_string(),
                    "qwen2.5-coder:1.5b".to_string(),
                    "qwen2.5-coder:3b".to_string(),
                    "qwen2.5:cloud".to_string(),
                    "minimax-m2.5:cloud".to_string(),
                    "deepseek-v3.1:cloud".to_string(),
                ]
            }
        }
        "astrai" => {
            fetch_openai_compatible_models("https://as-trai.com/v1/models", api_key.as_deref())?
        }
        "copilot" => fetch_copilot_models(api_key.as_deref())?,
        _ => Vec::new(),
    };

    Ok(models)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelCacheEntry {
    provider: String,
    fetched_at_unix: u64,
    models: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ModelCacheState {
    entries: Vec<ModelCacheEntry>,
}

#[derive(Debug, Clone)]
struct CachedModels {
    models: Vec<String>,
    age_secs: u64,
}

fn model_cache_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(MODEL_CACHE_FILE)
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn load_model_cache_state(workspace_dir: &Path) -> Result<ModelCacheState> {
    let path = model_cache_path(workspace_dir);
    if !path.exists() {
        return Ok(ModelCacheState::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read model cache at {}", path.display()))?;

    match serde_json::from_str::<ModelCacheState>(&raw) {
        Ok(state) => Ok(state),
        Err(_) => Ok(ModelCacheState::default()),
    }
}

fn save_model_cache_state(workspace_dir: &Path, state: &ModelCacheState) -> Result<()> {
    let path = model_cache_path(workspace_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create model cache directory {}",
                parent.display()
            )
        })?;
    }

    let json = serde_json::to_vec_pretty(state).context("failed to serialize model cache")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write model cache at {}", path.display()))?;

    Ok(())
}

fn cache_live_models_for_provider(
    workspace_dir: &Path,
    provider_name: &str,
    models: &[String],
) -> Result<()> {
    let normalized_models = normalize_model_ids(models.to_vec());
    if normalized_models.is_empty() {
        return Ok(());
    }

    let mut state = load_model_cache_state(workspace_dir)?;
    let now = now_unix_secs();

    if let Some(entry) = state
        .entries
        .iter_mut()
        .find(|entry| entry.provider == provider_name)
    {
        entry.fetched_at_unix = now;
        entry.models = normalized_models;
    } else {
        state.entries.push(ModelCacheEntry {
            provider: provider_name.to_string(),
            fetched_at_unix: now,
            models: normalized_models,
        });
    }

    save_model_cache_state(workspace_dir, &state)
}

fn load_cached_models_for_provider_internal(
    workspace_dir: &Path,
    provider_name: &str,
    ttl_secs: Option<u64>,
) -> Result<Option<CachedModels>> {
    let state = load_model_cache_state(workspace_dir)?;
    let now = now_unix_secs();

    let Some(entry) = state
        .entries
        .into_iter()
        .find(|entry| entry.provider == provider_name)
    else {
        return Ok(None);
    };

    if entry.models.is_empty() {
        return Ok(None);
    }

    let age_secs = now.saturating_sub(entry.fetched_at_unix);
    if ttl_secs.is_some_and(|ttl| age_secs > ttl) {
        return Ok(None);
    }

    Ok(Some(CachedModels {
        models: entry.models,
        age_secs,
    }))
}

fn load_cached_models_for_provider(
    workspace_dir: &Path,
    provider_name: &str,
    ttl_secs: u64,
) -> Result<Option<CachedModels>> {
    load_cached_models_for_provider_internal(workspace_dir, provider_name, Some(ttl_secs))
}

fn load_any_cached_models_for_provider(
    workspace_dir: &Path,
    provider_name: &str,
) -> Result<Option<CachedModels>> {
    load_cached_models_for_provider_internal(workspace_dir, provider_name, None)
}

fn humanize_age(age_secs: u64) -> String {
    if age_secs < 60 {
        format!("{age_secs}s")
    } else if age_secs < 60 * 60 {
        format!("{}m", age_secs / 60)
    } else {
        format!("{}h", age_secs / (60 * 60))
    }
}

fn build_model_options(model_ids: Vec<String>, source: &str) -> Vec<(String, String)> {
    model_ids
        .into_iter()
        .map(|model_id| {
            let label = format!("{model_id} ({source})");
            (model_id, label)
        })
        .collect()
}

fn load_fresh_cached_model_options(
    workspace_dir: &Path,
    provider_name: &str,
) -> Result<Option<Vec<(String, String)>>> {
    let Some(cached) =
        load_cached_models_for_provider(workspace_dir, provider_name, MODEL_CACHE_TTL_SECS)?
    else {
        return Ok(None);
    };

    let shown_count = cached.models.len().min(LIVE_MODEL_MAX_OPTIONS);
    print_bullet(&format!(
        "Found cached models ({shown_count}) updated {} ago.",
        humanize_age(cached.age_secs)
    ));

    Ok(Some(build_model_options(
        cached
            .models
            .into_iter()
            .take(LIVE_MODEL_MAX_OPTIONS)
            .collect(),
        "cached",
    )))
}

fn load_stale_cached_model_options(
    workspace_dir: &Path,
    provider_name: &str,
) -> Result<Option<Vec<(String, String)>>> {
    let Some(stale) = load_any_cached_models_for_provider(workspace_dir, provider_name)? else {
        return Ok(None);
    };

    print_bullet(&format!(
        "Loaded stale cache from {} ago.",
        humanize_age(stale.age_secs)
    ));

    Ok(Some(build_model_options(
        stale
            .models
            .into_iter()
            .take(LIVE_MODEL_MAX_OPTIONS)
            .collect(),
        "stale-cache",
    )))
}

fn handle_live_fetch_result(
    workspace_dir: &Path,
    provider_name: &str,
    fetch_result: Result<Vec<String>>,
    live_options: &mut Option<Vec<(String, String)>>,
) -> Result<()> {
    match fetch_result {
        Ok(live_model_ids) if !live_model_ids.is_empty() => {
            cache_live_models_for_provider(workspace_dir, provider_name, &live_model_ids)?;

            let fetched_count = live_model_ids.len();
            let shown_count = fetched_count.min(LIVE_MODEL_MAX_OPTIONS);
            let shown_models: Vec<String> = live_model_ids
                .into_iter()
                .take(LIVE_MODEL_MAX_OPTIONS)
                .collect();

            if shown_count < fetched_count {
                print_bullet(&format!(
                    "Fetched {fetched_count} models. Showing first {shown_count}."
                ));
            } else {
                print_bullet(&format!("Fetched {shown_count} live models."));
            }

            *live_options = Some(build_model_options(shown_models, "live"));
        }
        Ok(_) => {
            print_bullet("Provider returned no models; using curated list.");
        }
        Err(error) => {
            print_bullet("Live fetch failed; using cached/curated list.");
            tracing::debug!("Live model fetch error: {error}");

            if live_options.is_none() {
                *live_options = load_stale_cached_model_options(workspace_dir, provider_name)?;
            }
        }
    }

    Ok(())
}

fn fetch_live_model_options(
    workspace_dir: &Path,
    provider_name: &str,
    api_key: &str,
) -> Result<Option<Vec<(String, String)>>> {
    if !supports_live_model_fetch(provider_name) {
        return Ok(None);
    }

    let can_fetch_without_key = matches!(provider_name, "openrouter" | "ollama");
    let has_api_key = resolve_live_model_api_key(provider_name, api_key).is_some();

    if !can_fetch_without_key && !has_api_key {
        print_bullet("No API key detected, so using curated model list.");
        print_bullet("Tip: add an API key and rerun onboarding to fetch live models.");
        return Ok(None);
    }

    let mut live_options = load_fresh_cached_model_options(workspace_dir, provider_name)?;

    let should_fetch_now = Confirm::new()
        .with_prompt(if live_options.is_some() {
            "  Refresh models from provider now?"
        } else {
            "  Fetch latest models from provider now?"
        })
        .default(live_options.is_none())
        .interact()?;

    if should_fetch_now {
        handle_live_fetch_result(
            workspace_dir,
            provider_name,
            fetch_live_models_for_provider(provider_name, api_key),
            &mut live_options,
        )?;
    }

    Ok(live_options)
}

fn print_model_preview(models: &[String]) {
    for model in models.iter().take(MODEL_PREVIEW_LIMIT) {
        println!("  {} {model}", style("-"));
    }

    if models.len() > MODEL_PREVIEW_LIMIT {
        println!(
            "  {} ... and {} more",
            style("-"),
            models.len() - MODEL_PREVIEW_LIMIT
        );
    }
}

pub fn run_models_refresh(
    config: &Config,
    provider_override: Option<&str>,
    force: bool,
) -> Result<()> {
    let provider_name = provider_override
        .or(config.default_provider.as_deref())
        .unwrap_or("openrouter")
        .trim()
        .to_string();

    if provider_name.is_empty() {
        anyhow::bail!("Provider name cannot be empty");
    }

    if !supports_live_model_fetch(&provider_name) {
        anyhow::bail!("Provider '{provider_name}' does not support live model discovery yet");
    }

    if !force {
        if let Some(cached) = load_cached_models_for_provider(
            &config.workspace_dir,
            &provider_name,
            MODEL_CACHE_TTL_SECS,
        )? {
            println!(
                "Using cached model list for '{}' (updated {} ago):",
                provider_name,
                humanize_age(cached.age_secs)
            );
            print_model_preview(&cached.models);
            println!();
            println!(
                "Tip: run `corvus models refresh --force --provider {}` to fetch latest now.",
                provider_name
            );
            return Ok(());
        }
    }

    let api_key = config.api_key.clone().unwrap_or_default();

    match fetch_live_models_for_provider(&provider_name, &api_key) {
        Ok(models) if !models.is_empty() => {
            cache_live_models_for_provider(&config.workspace_dir, &provider_name, &models)?;
            println!(
                "Refreshed '{}' model cache with {} models.",
                provider_name,
                models.len()
            );
            print_model_preview(&models);
            Ok(())
        }
        Ok(_) => {
            if let Some(stale_cache) =
                load_any_cached_models_for_provider(&config.workspace_dir, &provider_name)?
            {
                println!(
                    "Provider returned no models; using stale cache (updated {} ago):",
                    humanize_age(stale_cache.age_secs)
                );
                print_model_preview(&stale_cache.models);
                return Ok(());
            }

            anyhow::bail!("Provider '{}' returned an empty model list", provider_name)
        }
        Err(error) => {
            if let Some(stale_cache) =
                load_any_cached_models_for_provider(&config.workspace_dir, &provider_name)?
            {
                println!(
                    "Live refresh failed ({}). Falling back to stale cache (updated {} ago):",
                    error,
                    humanize_age(stale_cache.age_secs)
                );
                print_model_preview(&stale_cache.models);
                return Ok(());
            }

            Err(error)
                .with_context(|| format!("failed to refresh models for provider '{provider_name}'"))
        }
    }
}

// ── Step helpers ─────────────────────────────────────────────────

fn print_step(current: u8, total: u8, title: &str) {
    println!();
    println!(
        "  {} {}",
        style(format!("[{current}/{total}]")).cyan().bold(),
        style(title).white().bold()
    );
    println!("  {}", style("─".repeat(50)).dim());
}

fn print_bullet(text: &str) {
    println!("  {} {}", style("›").cyan(), text);
}

fn persist_workspace_selection(config_path: &Path) -> Result<()> {
    let config_dir = config_path
        .parent()
        .context("Config path must have a parent directory")?;
    crate::config::schema::persist_active_workspace_config_dir(config_dir).with_context(|| {
        format!(
            "Failed to persist active workspace selection for {}",
            config_dir.display()
        )
    })
}

// ── Step 1: Workspace ────────────────────────────────────────────

fn setup_workspace() -> Result<(PathBuf, PathBuf)> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let default_dir = home.join(".corvus");

    print_bullet(&format!(
        "Default location: {}",
        style(default_dir.display()).green()
    ));

    let use_default = Confirm::new()
        .with_prompt("  Use default workspace location?")
        .default(true)
        .interact()?;

    let corvus_dir = if use_default {
        default_dir
    } else {
        let custom: String = Input::new()
            .with_prompt("  Enter workspace path")
            .interact_text()?;
        let expanded = shellexpand::tilde(&custom).to_string();
        PathBuf::from(expanded)
    };

    let workspace_dir = corvus_dir.join("workspace");
    let config_path = corvus_dir.join("config.toml");

    fs::create_dir_all(&workspace_dir).context("Failed to create workspace directory")?;

    println!(
        "  {} Workspace: {}",
        style("✓").green().bold(),
        style(workspace_dir.display()).green()
    );

    Ok((workspace_dir, config_path))
}

// ── Provider Tier Helpers ───────────────────────────────────────────

fn tier_options() -> Vec<&'static str> {
    vec![
        "⭐ Recommended (OpenRouter, Venice, Anthropic, OpenAI, Gemini, GitHub Copilot)",
        "⚡ Fast inference (Groq, Fireworks, Together AI, NVIDIA NIM)",
        "🌐 Gateway / proxy (Vercel AI, Cloudflare AI, Amazon Bedrock)",
        "🔬 Specialized (Moonshot/Kimi, GLM/Zhipu, MiniMax, Qwen/DashScope, Qianfan, Z.AI, Synthetic, OpenCode Zen, Cohere)",
        "🏠 Local / private (Ollama — no API key needed)",
        "🔧 Custom — bring your own OpenAI-compatible API",
    ]
}

fn providers_for_tier(tier_idx: usize) -> Vec<(&'static str, &'static str)> {
    match tier_idx {
        0 => vec![
            (
                "openrouter",
                "OpenRouter — 200+ models, 1 API key (recommended)",
            ),
            ("venice", "Venice AI — privacy-first (Llama, Opus)"),
            ("anthropic", "Anthropic — Claude Sonnet & Opus (direct)"),
            ("openai", "OpenAI — GPT-4o, o1, GPT-5 (direct)"),
            (
                "openai-codex",
                "OpenAI Codex (ChatGPT subscription OAuth, no API key)",
            ),
            (
                "copilot",
                "GitHub Copilot — GitHub subscription OAuth (no API key required)",
            ),
            ("deepseek", "DeepSeek — V3 & R1 (affordable)"),
            ("mistral", "Mistral — Large & Codestral"),
            ("xai", "xAI — Grok 3 & 4"),
            ("perplexity", "Perplexity — search-augmented AI"),
            (
                "gemini",
                "Google Gemini — Gemini 2.0 Flash & Pro (supports CLI auth)",
            ),
        ],
        1 => vec![
            ("groq", "Groq — ultra-fast LPU inference"),
            ("fireworks", "Fireworks AI — fast open-source inference"),
            ("together-ai", "Together AI — open-source model hosting"),
            ("nvidia", "NVIDIA NIM — DeepSeek, Llama, & more"),
        ],
        2 => vec![
            ("vercel", "Vercel AI Gateway"),
            ("cloudflare", "Cloudflare AI Gateway"),
            (
                "astrai",
                "Astrai — compliant AI routing (PII stripping, cost optimization)",
            ),
            ("bedrock", "Amazon Bedrock — AWS managed models"),
        ],
        3 => vec![
            ("moonshot", "Moonshot — Kimi API (China endpoint)"),
            (
                "moonshot-intl",
                "Moonshot — Kimi API (international endpoint)",
            ),
            ("glm", "GLM — ChatGLM / Zhipu (international endpoint)"),
            ("glm-cn", "GLM — ChatGLM / Zhipu (China endpoint)"),
            (
                "minimax",
                "MiniMax — international endpoint (api.minimax.io)",
            ),
            ("minimax-cn", "MiniMax — China endpoint (api.minimaxi.com)"),
            ("qwen", "Qwen — DashScope China endpoint"),
            ("qwen-intl", "Qwen — DashScope international endpoint"),
            ("qwen-us", "Qwen — DashScope US endpoint"),
            ("qianfan", "Qianfan — Baidu AI models (China endpoint)"),
            ("zai", "Z.AI — global coding endpoint"),
            ("zai-cn", "Z.AI — China coding endpoint (open.bigmodel.cn)"),
            ("synthetic", "Synthetic — Synthetic AI models"),
            ("opencode", "OpenCode Zen — code-focused AI"),
            ("cohere", "Cohere — Command R+ & embeddings"),
        ],
        4 => vec![("ollama", "Ollama — local models (Llama, Mistral, Phi)")],
        _ => vec![],
    }
}

fn api_key_url_for_provider(provider_name: &str) -> &'static str {
    if is_moonshot_alias(provider_name) {
        "https://platform.moonshot.cn/console/api-keys"
    } else if is_glm_cn_alias(provider_name) || is_zai_cn_alias(provider_name) {
        "https://open.bigmodel.cn/usercenter/proj-mgmt/apikeys"
    } else if is_glm_alias(provider_name) || is_zai_alias(provider_name) {
        "https://platform.z.ai/"
    } else if is_minimax_alias(provider_name) {
        "https://www.minimaxi.com/user-center/basic-information"
    } else if is_qwen_alias(provider_name) {
        "https://help.aliyun.com/zh/model-studio/developer-reference/get-api-key"
    } else if is_qianfan_alias(provider_name) {
        "https://cloud.baidu.com/doc/WENXINWORKSHOP/s/7lm0vxo78"
    } else {
        match provider_name {
            "openrouter" => "https://openrouter.ai/keys",
            "openai" => "https://platform.openai.com/api-keys",
            "venice" => "https://venice.ai/settings/api",
            "groq" => "https://console.groq.com/keys",
            "mistral" => "https://console.mistral.ai/api-keys",
            "deepseek" => "https://platform.deepseek.com/api_keys",
            "together-ai" => "https://api.together.xyz/settings/api-keys",
            "fireworks" => "https://fireworks.ai/account/api-keys",
            "perplexity" => "https://www.perplexity.ai/settings/api",
            "xai" => "https://console.x.ai",
            "cohere" => "https://dashboard.cohere.com/api-keys",
            "vercel" => "https://vercel.com/account/tokens",
            "cloudflare" => "https://dash.cloudflare.com/profile/api-tokens",
            "nvidia" | "nvidia-nim" | "build.nvidia.com" => "https://build.nvidia.com/",
            "bedrock" => "https://console.aws.amazon.com/iam",
            "gemini" => "https://aistudio.google.com/app/apikey",
            "astrai" => "https://as-trai.com",
            _ => "",
        }
    }
}

fn setup_custom_provider() -> Result<(String, String, String)> {
    println!();
    println!(
        "  {} {}",
        style("Custom Provider Setup").white().bold(),
        style("— any OpenAI-compatible API").dim()
    );
    print_bullet("Corvus works with ANY API that speaks the OpenAI chat completions format.");
    print_bullet("Examples: LiteLLM, LocalAI, vLLM, text-generation-webui, LM Studio, etc.");
    println!();

    let base_url: String = Input::new()
        .with_prompt("  API base URL (e.g. http://localhost:1234 or https://my-api.com)")
        .interact_text()?;

    let base_url = base_url.trim().trim_end_matches('/').to_string();
    if base_url.is_empty() {
        anyhow::bail!("Custom provider requires a base URL.");
    }

    let api_key: String = Input::new()
        .with_prompt("  API key (or Enter to skip if not needed)")
        .allow_empty(true)
        .interact_text()?;

    let model: String = Input::new()
        .with_prompt("  Model name (e.g. llama3, gpt-4o, mistral)")
        .default("default".into())
        .interact_text()?;

    let provider_name = format!("custom:{base_url}");

    println!(
        "  {} Provider: {} | Model: {}",
        style("✓").green().bold(),
        style(&provider_name).green(),
        style(&model).green()
    );

    Ok((provider_name, api_key, model))
}

fn setup_ollama_api_key() -> Result<(String, Option<String>)> {
    let use_remote_ollama = Confirm::new()
        .with_prompt("  Use a remote Ollama endpoint (for example Ollama Cloud)?")
        .default(false)
        .interact()
        .unwrap_or(false);

    if use_remote_ollama {
        let raw_url: String = Input::new()
            .with_prompt("  Remote Ollama endpoint URL")
            .default("https://ollama.com".into())
            .interact_text()
            .unwrap_or_default();

        let normalized_url = raw_url.trim().trim_end_matches('/').to_string();
        if normalized_url.is_empty() {
            anyhow::bail!("Remote Ollama endpoint URL cannot be empty.");
        }

        print_bullet(&format!(
            "Remote endpoint configured: {}",
            style(&normalized_url).cyan()
        ));
        print_bullet(&format!(
            "If you use cloud-only models, append {} to the model ID.",
            style(":cloud").yellow()
        ));

        let key: String = Input::new()
            .with_prompt("  API key for remote Ollama endpoint (or Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .unwrap_or_default();

        if key.trim().is_empty() {
            print_bullet(&format!(
                "No API key provided. Set {} later if required by your endpoint.",
                style("OLLAMA_API_KEY").yellow()
            ));
        }

        Ok((key, Some(normalized_url)))
    } else {
        print_bullet("Using local Ollama at http://localhost:11434 (no API key needed).");
        Ok((String::new(), None))
    }
}

fn setup_gemini_api_key() -> String {
    if crate::providers::gemini::GeminiProvider::has_cli_credentials() {
        print_bullet(&format!(
            "{} Gemini CLI credentials detected! You can skip the API key.",
            style("✓").green().bold()
        ));
        print_bullet("Corvus will reuse your existing Gemini CLI authentication.");
        println!();

        let use_cli: bool = dialoguer::Confirm::new()
            .with_prompt("  Use existing Gemini CLI authentication?")
            .default(true)
            .interact()
            .unwrap_or(true);

        if use_cli {
            println!(
                "  {} Using Gemini CLI OAuth tokens",
                style("✓").green().bold()
            );
            return String::new();
        }
        print_bullet("Get your API key at: https://aistudio.google.com/app/apikey");
    } else if std::env::var("GEMINI_API_KEY").is_ok() {
        print_bullet(&format!(
            "{} GEMINI_API_KEY environment variable detected!",
            style("✓").green().bold()
        ));
        return String::new();
    } else {
        print_bullet("Get your API key at: https://aistudio.google.com/app/apikey");
        print_bullet("Or run `gemini` CLI to authenticate (tokens will be reused).");
        println!();
    }

    Input::new()
        .with_prompt("  Paste your Gemini API key (or press Enter to skip)")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default()
}

fn setup_anthropic_api_key() -> String {
    if std::env::var("ANTHROPIC_OAUTH_TOKEN").is_ok() {
        print_bullet(&format!(
            "{} ANTHROPIC_OAUTH_TOKEN environment variable detected!",
            style("✓").green().bold()
        ));
        String::new()
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        print_bullet(&format!(
            "{} ANTHROPIC_API_KEY environment variable detected!",
            style("✓").green().bold()
        ));
        String::new()
    } else {
        print_bullet(&format!(
            "Get your API key at: {}",
            style("https://console.anthropic.com/settings/keys")
                .cyan()
                .underlined()
        ));
        print_bullet("Or run `claude setup-token` to get an OAuth setup-token.");
        println!();

        let key: String = Input::new()
            .with_prompt("  Paste your API key or setup-token (or press Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .unwrap_or_default();

        if key.is_empty() {
            print_bullet(&format!(
                "Skipped. Set {} or {} or edit config.toml later.",
                style("ANTHROPIC_API_KEY").yellow(),
                style("ANTHROPIC_OAUTH_TOKEN").yellow()
            ));
        }

        key
    }
}

fn setup_copilot_api_key() -> String {
    if std::env::var("GITHUB_TOKEN").is_ok() || std::env::var("GH_TOKEN").is_ok() {
        print_bullet(&format!(
            "{} GitHub token environment variable detected!",
            style("✓").green().bold()
        ));
        String::new()
    } else {
        print_bullet("Corvus can authenticate with GitHub Copilot automatically via device code.");
        print_bullet("Optional: paste a GitHub token now to skip browser login.");
        println!();

        Input::new()
            .with_prompt("  Paste GitHub token (or press Enter to use OAuth device login)")
            .allow_empty(true)
            .interact_text()
            .unwrap_or_default()
    }
}

fn setup_generic_api_key(provider_name: &str) -> String {
    let key_url = api_key_url_for_provider(provider_name);

    println!();
    if !key_url.is_empty() {
        print_bullet(&format!(
            "Get your API key at: {}",
            style(key_url).cyan().underlined()
        ));
    }
    print_bullet("You can also set it later via env var or config file.");
    println!();

    let key: String = Input::new()
        .with_prompt("  Paste your API key (or press Enter to skip)")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();

    if key.is_empty() {
        let env_var = provider_env_var(provider_name);
        print_bullet(&format!(
            "Skipped. Set {} or edit config.toml later.",
            style(env_var).yellow()
        ));
    }

    key
}

fn collect_api_key(provider_name: &str) -> Result<(String, Option<String>)> {
    let mut provider_api_url: Option<String> = None;

    let api_key = if provider_name == "ollama" {
        let (key, url) = setup_ollama_api_key()?;
        provider_api_url = url;
        key
    } else if canonical_provider_name(provider_name) == "gemini" {
        setup_gemini_api_key()
    } else if canonical_provider_name(provider_name) == "anthropic" {
        setup_anthropic_api_key()
    } else if canonical_provider_name(provider_name) == "copilot" {
        setup_copilot_api_key()
    } else {
        setup_generic_api_key(provider_name)
    };

    Ok((api_key, provider_api_url))
}

// ── Step 2: Provider & API Key ───────────────────────────────────

#[allow(clippy::too_many_lines)]
fn setup_provider(workspace_dir: &Path) -> Result<(String, String, String, Option<String>)> {
    let tier_idx = Select::new()
        .with_prompt("  Select provider category")
        .items(tier_options())
        .default(0)
        .interact()?;

    let providers = providers_for_tier(tier_idx);

    if providers.is_empty() {
        let (provider_name, api_key, model) = setup_custom_provider()?;
        return Ok((provider_name, api_key, model, None));
    }

    let provider_labels: Vec<&str> = providers.iter().map(|(_, label)| *label).collect();

    let provider_idx = Select::new()
        .with_prompt("  Select your AI provider")
        .items(&provider_labels)
        .default(0)
        .interact()?;

    let provider_name = providers[provider_idx].0;

    let (api_key, provider_api_url) = collect_api_key(provider_name)?;

    // ── Model selection ──
    let canonical_provider = canonical_provider_name(provider_name);
    let mut model_options: Vec<(String, String)> = models_for_provider(canonical_provider);

    let live_options = fetch_live_model_options(workspace_dir, provider_name, &api_key)?;

    if let Some(live_model_options) = live_options {
        let source_options = vec![
            format!("Provider model list ({})", live_model_options.len()),
            format!("Curated starter list ({})", model_options.len()),
        ];

        let source_idx = Select::new()
            .with_prompt("  Model source")
            .items(&source_options)
            .default(0)
            .interact()?;

        if source_idx == 0 {
            model_options = live_model_options;
        }
    }

    if model_options.is_empty() {
        model_options.push((
            default_model_for_provider(provider_name),
            "Provider default model".to_string(),
        ));
    }

    model_options.push((
        CUSTOM_MODEL_SENTINEL.to_string(),
        "Custom model ID (type manually)".to_string(),
    ));

    let model_labels: Vec<String> = model_options
        .iter()
        .map(|(model_id, label)| format!("{label} — {}", style(model_id).dim()))
        .collect();

    let model_idx = Select::new()
        .with_prompt("  Select your default model")
        .items(&model_labels)
        .default(0)
        .interact()?;

    let selected_model = model_options[model_idx].0.clone();
    let model = if selected_model == CUSTOM_MODEL_SENTINEL {
        Input::new()
            .with_prompt("  Enter custom model ID")
            .default(default_model_for_provider(provider_name))
            .interact_text()?
    } else {
        selected_model
    };

    println!(
        "  {} Provider: {} | Model: {}",
        style("✓").green().bold(),
        style(provider_name).green(),
        style(&model).green()
    );

    Ok((provider_name.to_string(), api_key, model, provider_api_url))
}

/// Map provider name to its conventional env var
fn provider_env_var(name: &str) -> &'static str {
    match canonical_provider_name(name) {
        "openrouter" => "OPENROUTER_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "ollama" => "OLLAMA_API_KEY",
        "venice" => "VENICE_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "xai" => "XAI_API_KEY",
        "together-ai" => "TOGETHER_API_KEY",
        "fireworks" | "fireworks-ai" => "FIREWORKS_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "cohere" => "COHERE_API_KEY",
        "moonshot" => "MOONSHOT_API_KEY",
        "glm" => "GLM_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "qwen" => "DASHSCOPE_API_KEY",
        "qianfan" => "QIANFAN_API_KEY",
        "zai" => "ZAI_API_KEY",
        "synthetic" => "SYNTHETIC_API_KEY",
        "opencode" | "opencode-zen" => "OPENCODE_API_KEY",
        "vercel" | "vercel-ai" => "VERCEL_API_KEY",
        "cloudflare" | "cloudflare-ai" => "CLOUDFLARE_API_KEY",
        "bedrock" | "aws-bedrock" => "AWS_ACCESS_KEY_ID",
        "gemini" => "GEMINI_API_KEY",
        "copilot" => "GITHUB_TOKEN",
        "nvidia" | "nvidia-nim" | "build.nvidia.com" => "NVIDIA_API_KEY",
        "astrai" => "ASTRAI_API_KEY",
        _ => "API_KEY",
    }
}

// ── Step 5: Tool Mode & Security ────────────────────────────────

fn setup_tool_mode() -> Result<(ComposioConfig, SecretsConfig)> {
    print_bullet("Choose how Corvus connects to external apps.");
    print_bullet("You can always change this later in config.toml.");
    println!();

    let options = vec![
        "Sovereign (local only) — you manage API keys, full privacy (default)",
        "Composio (managed OAuth) — 1000+ apps via OAuth, no raw keys shared",
    ];

    let choice = Select::new()
        .with_prompt("  Select tool mode")
        .items(&options)
        .default(0)
        .interact()?;

    let composio_config = if choice == 1 {
        println!();
        println!(
            "  {} {}",
            style("Composio Setup").white().bold(),
            style("— 1000+ OAuth integrations (Gmail, Notion, GitHub, Slack, ...)").dim()
        );
        print_bullet("Get your API key at: https://app.composio.dev/settings");
        print_bullet("Corvus uses Composio as a tool — your core agent stays local.");
        println!();

        let api_key: String = Input::new()
            .with_prompt("  Composio API key (or Enter to skip)")
            .allow_empty(true)
            .interact_text()?;

        if api_key.trim().is_empty() {
            println!(
                "  {} Skipped — set composio.api_key in config.toml later",
                style("→").dim()
            );
            ComposioConfig::default()
        } else {
            println!(
                "  {} Composio: {} (1000+ OAuth tools available)",
                style("✓").green().bold(),
                style("enabled").green()
            );
            ComposioConfig {
                enabled: true,
                api_key: Some(api_key),
                ..ComposioConfig::default()
            }
        }
    } else {
        println!(
            "  {} Tool mode: {} — full privacy, you own every key",
            style("✓").green().bold(),
            style("Sovereign (local only)").green()
        );
        ComposioConfig::default()
    };

    // ── Encrypted secrets ──
    println!();
    print_bullet("Corvus can encrypt API keys stored in config.toml.");
    print_bullet("A local key file protects against plaintext exposure and accidental leaks.");

    let encrypt = Confirm::new()
        .with_prompt("  Enable encrypted secret storage?")
        .default(true)
        .interact()?;

    let secrets_config = SecretsConfig { encrypt };

    if encrypt {
        println!(
            "  {} Secrets: {} — keys encrypted with local key file",
            style("✓").green().bold(),
            style("encrypted").green()
        );
    } else {
        println!(
            "  {} Secrets: {} — keys stored as plaintext (not recommended)",
            style("✓").green().bold(),
            style("plaintext").yellow()
        );
    }

    Ok((composio_config, secrets_config))
}

// ── Step 6: Hardware (Physical World) ───────────────────────────

fn setup_hardware() -> Result<HardwareConfig> {
    print_bullet("Corvus can talk to physical hardware (LEDs, sensors, motors).");
    print_bullet("Scanning for connected devices...");
    println!();

    // ── Auto-discovery ──
    let devices = hardware::discover_hardware();

    if devices.is_empty() {
        println!(
            "  {} {}",
            style("ℹ").dim(),
            style("No hardware devices detected on this system.").dim()
        );
        println!(
            "  {} {}",
            style("ℹ").dim(),
            style("You can enable hardware later in config.toml under [hardware].").dim()
        );
    } else {
        println!(
            "  {} {} device(s) found:",
            style("✓").green().bold(),
            devices.len()
        );
        for device in &devices {
            let detail = device
                .detail
                .as_deref()
                .map(|d| format!(" ({d})"))
                .unwrap_or_default();
            let path = device
                .device_path
                .as_deref()
                .map(|p| format!(" → {p}"))
                .unwrap_or_default();
            println!(
                "    {} {}{}{} [{}]",
                style("›").cyan(),
                style(&device.name).green(),
                style(&detail).dim(),
                style(&path).dim(),
                style(device.transport.to_string()).cyan()
            );
        }
    }
    println!();

    let options = vec![
        "🚀 Native — direct GPIO on this Linux board (Raspberry Pi, Orange Pi, etc.)",
        "🔌 Tethered — control an Arduino/ESP32/Nucleo plugged into USB",
        "🔬 Debug Probe — flash/read MCUs via SWD/JTAG (probe-rs)",
        "☁️  Software Only — no hardware access (default)",
    ];

    let recommended = hardware::recommended_wizard_default(&devices);

    let choice = Select::new()
        .with_prompt("  How should Corvus interact with the physical world?")
        .items(&options)
        .default(recommended)
        .interact()?;

    let mut hw_config = hardware::config_from_wizard_choice(choice, &devices);

    // ── Serial: pick a port if multiple found ──
    if hw_config.transport_mode() == hardware::HardwareTransport::Serial {
        let serial_devices: Vec<&hardware::DiscoveredDevice> = devices
            .iter()
            .filter(|d| d.transport == hardware::HardwareTransport::Serial)
            .collect();

        if serial_devices.len() > 1 {
            let port_labels: Vec<String> = serial_devices
                .iter()
                .map(|d| {
                    format!(
                        "{} ({})",
                        d.device_path.as_deref().unwrap_or("unknown"),
                        d.name
                    )
                })
                .collect();

            let port_idx = Select::new()
                .with_prompt("  Multiple serial devices found — select one")
                .items(&port_labels)
                .default(0)
                .interact()?;

            hw_config.serial_port = serial_devices[port_idx].device_path.clone();
        } else if serial_devices.is_empty() {
            // User chose serial but no device discovered — ask for manual path
            let manual_port: String = Input::new()
                .with_prompt("  Serial port path (e.g. /dev/ttyUSB0)")
                .default("/dev/ttyUSB0".into())
                .interact_text()?;
            hw_config.serial_port = Some(manual_port);
        }

        // Baud rate
        let baud_options = vec![
            "115200 (default, recommended)",
            "9600 (legacy Arduino)",
            "57600",
            "230400",
            "Custom",
        ];
        let baud_idx = Select::new()
            .with_prompt("  Serial baud rate")
            .items(&baud_options)
            .default(0)
            .interact()?;

        hw_config.baud_rate = match baud_idx {
            1 => 9600,
            2 => 57600,
            3 => 230_400,
            4 => {
                let custom: String = Input::new()
                    .with_prompt("  Custom baud rate")
                    .default("115200".into())
                    .interact_text()?;
                custom.parse::<u32>().unwrap_or(115_200)
            }
            _ => 115_200,
        };
    }

    // ── Probe: ask for target chip ──
    if hw_config.transport_mode() == hardware::HardwareTransport::Probe
        && hw_config.probe_target.is_none()
    {
        let target: String = Input::new()
            .with_prompt("  Target MCU chip (e.g. STM32F411CEUx, nRF52840_xxAA)")
            .default("STM32F411CEUx".into())
            .interact_text()?;
        hw_config.probe_target = Some(target);
    }

    // ── Datasheet RAG ──
    if hw_config.enabled {
        let datasheets = Confirm::new()
            .with_prompt("  Enable datasheet RAG? (index PDF schematics for AI pin lookups)")
            .default(true)
            .interact()?;
        hw_config.workspace_datasheets = datasheets;
    }

    // ── Summary ──
    if hw_config.enabled {
        let transport_label = match hw_config.transport_mode() {
            hardware::HardwareTransport::Native => "Native GPIO".to_string(),
            hardware::HardwareTransport::Serial => format!(
                "Serial → {} @ {} baud",
                hw_config.serial_port.as_deref().unwrap_or("?"),
                hw_config.baud_rate
            ),
            hardware::HardwareTransport::Probe => format!(
                "Probe (SWD/JTAG) → {}",
                hw_config.probe_target.as_deref().unwrap_or("?")
            ),
            hardware::HardwareTransport::None => "Software Only".to_string(),
        };

        println!(
            "  {} Hardware: {} | datasheets: {}",
            style("✓").green().bold(),
            style(&transport_label).green(),
            if hw_config.workspace_datasheets {
                style("on").green().to_string()
            } else {
                style("off").dim().to_string()
            }
        );
    } else {
        println!(
            "  {} Hardware: {}",
            style("✓").green().bold(),
            style("disabled (software only)").dim()
        );
    }

    Ok(hw_config)
}

// ── Step 6: Project Context ─────────────────────────────────────

fn setup_project_context() -> Result<ProjectContext> {
    print_bullet("Let's personalize your agent. You can always update these later.");
    print_bullet("Press Enter to accept defaults.");
    println!();

    let user_name: String = Input::new()
        .with_prompt("  Your name")
        .default("User".into())
        .interact_text()?;

    let tz_options = vec![
        "US/Eastern (EST/EDT)",
        "US/Central (CST/CDT)",
        "US/Mountain (MST/MDT)",
        "US/Pacific (PST/PDT)",
        "Europe/London (GMT/BST)",
        "Europe/Berlin (CET/CEST)",
        "Asia/Tokyo (JST)",
        "UTC",
        "Other (type manually)",
    ];

    let tz_idx = Select::new()
        .with_prompt("  Your timezone")
        .items(&tz_options)
        .default(0)
        .interact()?;

    let timezone = if tz_idx == tz_options.len() - 1 {
        Input::new()
            .with_prompt("  Enter timezone (e.g. America/New_York)")
            .default("UTC".into())
            .interact_text()?
    } else {
        // Extract the short label before the parenthetical
        tz_options[tz_idx]
            .split('(')
            .next()
            .unwrap_or("UTC")
            .trim()
            .to_string()
    };

    let agent_name: String = Input::new()
        .with_prompt("  Agent name")
        .default("Corvus".into())
        .interact_text()?;

    let style_options = vec![
        "Direct & concise — skip pleasantries, get to the point",
        "Friendly & casual — warm, human, and helpful",
        "Professional & polished — calm, confident, and clear",
        "Expressive & playful — more personality + natural emojis",
        "Technical & detailed — thorough explanations, code-first",
        "Balanced — adapt to the situation",
        "Custom — write your own style guide",
    ];

    let style_idx = Select::new()
        .with_prompt("  Communication style")
        .items(&style_options)
        .default(1)
        .interact()?;

    let communication_style = match style_idx {
        0 => "Be direct and concise. Skip pleasantries. Get to the point.".to_string(),
        1 => "Be friendly, human, and conversational. Show warmth and empathy while staying efficient. Use natural contractions.".to_string(),
        2 => "Be professional and polished. Stay calm, structured, and respectful. Use occasional tone-setting emojis only when appropriate.".to_string(),
        3 => "Be expressive and playful when appropriate. Use relevant emojis naturally (0-2 max), and keep serious topics emoji-light.".to_string(),
        4 => "Be technical and detailed. Thorough explanations, code-first.".to_string(),
        5 => "Adapt to the situation. Default to warm and clear communication; be concise when needed, thorough when it matters.".to_string(),
        _ => Input::new()
            .with_prompt("  Custom communication style")
            .default(
                "Be warm, natural, and clear. Use occasional relevant emojis (1-2 max) and avoid robotic phrasing.".into(),
            )
            .interact_text()?,
    };

    println!(
        "  {} Context: {} | {} | {} | {}",
        style("✓").green().bold(),
        style(&user_name).green(),
        style(&timezone).green(),
        style(&agent_name).green(),
        style(&communication_style).green().dim()
    );

    Ok(ProjectContext {
        user_name,
        timezone,
        agent_name,
        communication_style,
    })
}

// ── Step 6: Memory Configuration ───────────────────────────────

fn scaffold_surreal_docker_files() -> Result<PathBuf> {
    let home = UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let surreal_dir = home.join(".corvus").join("surreal");
    fs::create_dir_all(&surreal_dir).context("Failed to create Surreal helper directory")?;

    let compose_path = surreal_dir.join("docker-compose.yml");
    let env_example_path = surreal_dir.join(".env.example");

    let compose_contents = r#"services:
  surrealdb:
    image: surrealdb/surrealdb:v2.3
    # Do not use the in-memory backend ("memory") in production; it is non-persistent.
    command: start --log info --user ${SURREALDB_USER:-corvus} --pass ${SURREALDB_PASS:-corvus-pass} surrealkv://data/corvus.db
    ports:
      - "8000:8000"
    volumes:
      - surrealdb-data:/data
    healthcheck:
      test: ["CMD", "surreal", "is-ready", "--conn", "http://127.0.0.1:8000"]
      interval: 10s
      timeout: 5s
      retries: 10

volumes:
  surrealdb-data:
"#;

    let env_contents = r#"# Copy to .env and adjust values.
SURREALDB_USER=corvus
SURREALDB_PASS=corvus-pass

# Corvus env-first memory config
CORVUS_SURREALDB_URL=http://127.0.0.1:8000
CORVUS_SURREALDB_NAMESPACE=corvus
CORVUS_SURREALDB_DATABASE=memory
CORVUS_SURREALDB_USERNAME=corvus
CORVUS_SURREALDB_PASSWORD=corvus-pass
"#;

    fs::write(&compose_path, compose_contents).context("Failed to write Surreal docker compose")?;
    fs::write(&env_example_path, env_contents).context("Failed to write Surreal env example")?;

    Ok(surreal_dir)
}

#[allow(clippy::too_many_lines)]
fn setup_surreal_memory_options(config: &mut MemoryConfig) -> Result<()> {
    print_bullet("SurrealDB backend selected.");
    print_bullet("Environment variables are preferred for credentials (recommended).");
    print_bullet("If you save credentials here, they are encrypted when [secrets].encrypt=true.");
    println!();

    println!("  Expected env vars (env-first):");
    println!("    - CORVUS_SURREALDB_URL");
    println!("    - CORVUS_SURREALDB_NAMESPACE");
    println!("    - CORVUS_SURREALDB_DATABASE");
    println!("    - CORVUS_SURREALDB_USERNAME");
    println!("    - CORVUS_SURREALDB_PASSWORD");
    println!("    - CORVUS_SURREALDB_TOKEN");
    println!();

    let endpoint: String = Input::new()
        .with_prompt("  SurrealDB URL")
        .default(
            config
                .surreal
                .url
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:8000".to_string()),
        )
        .interact_text()?;
    let namespace: String = Input::new()
        .with_prompt("  Namespace")
        .default(
            config
                .surreal
                .namespace
                .clone()
                .unwrap_or_else(|| "corvus".to_string()),
        )
        .interact_text()?;
    let database: String = Input::new()
        .with_prompt("  Database")
        .default(
            config
                .surreal
                .database
                .clone()
                .unwrap_or_else(|| "memory".to_string()),
        )
        .interact_text()?;

    let auth_modes = vec![
        "Use environment variables only (recommended)",
        "Save username/password fallback now",
        "Save token fallback now",
    ];
    let auth_choice = Select::new()
        .with_prompt("  Authentication fallback")
        .items(&auth_modes)
        .default(0)
        .interact()?;

    let (username, password, token) = match auth_choice {
        1 => {
            let user: String = Input::new()
                .with_prompt("  SurrealDB username")
                .allow_empty(true)
                .interact_text()?;
            let pass = Password::new()
                .with_prompt("  SurrealDB password (hidden)")
                .allow_empty_password(true)
                .interact()?;
            (trim_non_empty(user), trim_non_empty(pass), None)
        }
        2 => {
            let token = Password::new()
                .with_prompt("  SurrealDB token (hidden)")
                .allow_empty_password(true)
                .interact()?;
            (None, None, trim_non_empty(token))
        }
        _ => (None, None, None),
    };

    config.surreal = SurrealMemoryConfig {
        url: trim_non_empty(endpoint),
        namespace: trim_non_empty(namespace),
        database: trim_non_empty(database),
        username,
        password,
        token,
        allow_http_loopback: true,
    };

    let generate_docker = Confirm::new()
        .with_prompt("  Generate local SurrealDB Docker helper files in ~/.corvus/surreal?")
        .default(true)
        .interact()?;
    if generate_docker {
        let helper_dir = scaffold_surreal_docker_files()?;
        println!(
            "  {} Docker helper files generated:",
            style("✓").green().bold(),
        );
        println!(
            "    {}",
            style(helper_dir.join("docker-compose.yml").display()).green()
        );
        println!(
            "    {}",
            style(helper_dir.join(".env.example").display()).green()
        );
        println!("  Run manually:");
        println!("    cd {}", style(helper_dir.display()).green());
        println!("    cp .env.example .env");
        println!("    docker compose up -d");
    }

    Ok(())
}

fn trim_non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn requires_surreal_memory_setup(backend: &str) -> bool {
    matches!(backend, "surreal" | "surreal-graphs")
}

fn setup_memory() -> Result<MemoryConfig> {
    print_bullet("Choose how Corvus stores and searches memories.");
    print_bullet("You can always change this later in config.toml.");
    println!();

    let options: Vec<&str> = selectable_memory_backends()
        .iter()
        .map(|backend| backend.label)
        .collect();

    let choice = Select::new()
        .with_prompt("  Select memory backend")
        .items(&options)
        .default(0)
        .interact()?;

    let backend = backend_key_from_choice(choice);
    let profile = memory_backend_profile(backend);

    let auto_save = profile.auto_save_default
        && Confirm::new()
            .with_prompt("  Auto-save conversations to memory?")
            .default(true)
            .interact()?;

    println!(
        "  {} Memory: {} (auto-save: {})",
        style("✓").green().bold(),
        style(backend).green(),
        if auto_save { "on" } else { "off" }
    );

    let mut config = memory_config_defaults_for_backend(backend);
    config.auto_save = auto_save;

    if requires_surreal_memory_setup(&config.backend) {
        setup_surreal_memory_options(&mut config)?;
    }

    Ok(config)
}

// ── Step 3: Channels ────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn setup_telegram_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("Telegram Setup").white().bold(),
        style("— talk to Corvus from Telegram").dim()
    );
    print_bullet("1. Open Telegram and message @BotFather");
    print_bullet("2. Send /newbot and follow the prompts");
    print_bullet("3. Copy the bot token and paste it below");
    println!();

    let token: String = match Input::new()
        .with_prompt("  Bot token (from @BotFather)")
        .interact_text()
    {
        Ok(t) => t,
        Err(_) => return false,
    };

    if token.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    print!("  {} Testing connection... ", style("⏳").dim());
    let token_clone = token.clone();
    let thread_result = std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let url = format!("https://api.telegram.org/bot{token_clone}/getMe");
        let resp = client.get(&url).send()?;
        let ok = resp.status().is_success();
        let data: serde_json::Value = resp.json().unwrap_or_default();
        let bot_name = data
            .get("result")
            .and_then(|r| r.get("username"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        Ok::<_, reqwest::Error>((ok, bot_name))
    })
    .join();

    match thread_result {
        Ok(Ok((true, bot_name))) => {
            println!(
                "\r  {} Connected as @{bot_name}        ",
                style("✅").green().bold()
            );
        }
        _ => {
            println!(
                "\r  {} Connection failed — check your token and try again",
                style("❌").red().bold()
            );
            return false;
        }
    }

    print_bullet(
        "Allowlist your own Telegram identity first (recommended for secure + fast setup).",
    );
    print_bullet(
        "Use your @username without '@' (example: argenis), or your numeric Telegram user ID.",
    );
    print_bullet("Use '*' only for temporary open testing.");

    let users_str: String = match Input::new()
        .with_prompt(
            "  Allowed Telegram identities (comma-separated: username without '@' and/or numeric user ID, '*' for all)",
        )
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users = if users_str.trim() == "*" {
        vec!["*".into()]
    } else {
        users_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if allowed_users.is_empty() {
        println!(
            "  {} No users allowlisted — Telegram inbound messages will be denied until you add your username/user ID or '*'.",
            style("⚠").yellow().bold()
        );
    }

    config.telegram = Some(TelegramConfig {
        bot_token: token,
        allowed_users,
        stream_mode: StreamMode::default(),
        draft_update_interval_ms: 1000,
    });
    true
}

fn setup_discord_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("Discord Setup").white().bold(),
        style("— talk to Corvus from Discord").dim()
    );
    print_bullet("1. Go to https://discord.com/developers/applications");
    print_bullet("2. Create a New Application → Bot → Copy token");
    print_bullet("3. Enable MESSAGE CONTENT intent under Bot settings");
    print_bullet("4. Invite bot to your server with messages permission");
    println!();

    let token: String = match Input::new().with_prompt("  Bot token").interact_text() {
        Ok(t) => t,
        Err(_) => return false,
    };

    if token.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    print!("  {} Testing connection... ", style("⏳").dim());
    let token_clone = token.clone();
    let thread_result = std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {token_clone}"))
            .send()?;
        let ok = resp.status().is_success();
        let data: serde_json::Value = resp.json().unwrap_or_default();
        let bot_name = data
            .get("username")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        Ok::<_, reqwest::Error>((ok, bot_name))
    })
    .join();

    match thread_result {
        Ok(Ok((true, bot_name))) => {
            println!(
                "\r  {} Connected as {bot_name}        ",
                style("✅").green().bold()
            );
        }
        _ => {
            println!(
                "\r  {} Connection failed — check your token and try again",
                style("❌").red().bold()
            );
            return false;
        }
    }

    let guild: String = match Input::new()
        .with_prompt("  Server (guild) ID (optional, Enter to skip)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    print_bullet("Allowlist your own Discord user ID first (recommended).");
    print_bullet(
        "Get it in Discord: Settings -> Advanced -> Developer Mode (ON), then right-click your profile -> Copy User ID.",
    );
    print_bullet("Use '*' only for temporary open testing.");

    let allowed_users_str: String = match Input::new()
        .with_prompt(
            "  Allowed Discord user IDs (comma-separated, recommended: your own ID, '*' for all)",
        )
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users = if allowed_users_str.trim().is_empty() {
        vec![]
    } else {
        allowed_users_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if allowed_users.is_empty() {
        println!(
            "  {} No users allowlisted — Discord inbound messages will be denied until you add IDs or '*'.",
            style("⚠").yellow().bold()
        );
    }

    config.discord = Some(DiscordConfig {
        bot_token: token,
        guild_id: if guild.is_empty() { None } else { Some(guild) },
        allowed_users,
        listen_to_bots: false,
        mention_only: false,
    });
    true
}

fn setup_slack_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("Slack Setup").white().bold(),
        style("— talk to Corvus from Slack").dim()
    );
    print_bullet("1. Go to https://api.slack.com/apps → Create New App");
    print_bullet("2. Add Bot Token Scopes: chat:write, channels:history");
    print_bullet("3. Install to workspace and copy the Bot Token");
    println!();

    let token: String = match Input::new()
        .with_prompt("  Bot token (xoxb-...)")
        .interact_text()
    {
        Ok(t) => t,
        Err(_) => return false,
    };

    if token.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    print!("  {} Testing connection... ", style("⏳").dim());
    let token_clone = token.clone();
    let thread_result = std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get("https://slack.com/api/auth.test")
            .bearer_auth(&token_clone)
            .send()?;
        let ok = resp.status().is_success();
        let data: serde_json::Value = resp.json().unwrap_or_default();
        let api_ok = data
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let team = data
            .get("team")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let err = data
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown error")
            .to_string();
        Ok::<_, reqwest::Error>((ok, api_ok, team, err))
    })
    .join();

    match thread_result {
        Ok(Ok((true, true, team, _))) => {
            println!(
                "\r  {} Connected to workspace: {team}        ",
                style("✅").green().bold()
            );
        }
        Ok(Ok((true, false, _, err))) => {
            println!("\r  {} Slack error: {err}", style("❌").red().bold());
            return false;
        }
        _ => {
            println!(
                "\r  {} Connection failed — check your token",
                style("❌").red().bold()
            );
            return false;
        }
    }

    let app_token: String = match Input::new()
        .with_prompt("  App token (xapp-..., optional, Enter to skip)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let channel: String = match Input::new()
        .with_prompt("  Default channel ID (optional, Enter to skip)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    print_bullet("Allowlist your own Slack member ID first (recommended).");
    print_bullet(
        "Member IDs usually start with 'U' (open your Slack profile -> More -> Copy member ID).",
    );
    print_bullet("Use '*' only for temporary open testing.");

    let allowed_users_str: String = match Input::new()
        .with_prompt(
            "  Allowed Slack user IDs (comma-separated, recommended: your own member ID, '*' for all)",
        )
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users = if allowed_users_str.trim().is_empty() {
        vec![]
    } else {
        allowed_users_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if allowed_users.is_empty() {
        println!(
            "  {} No users allowlisted — Slack inbound messages will be denied until you add IDs or '*'.",
            style("⚠").yellow().bold()
        );
    }

    config.slack = Some(SlackConfig {
        bot_token: token,
        app_token: if app_token.is_empty() {
            None
        } else {
            Some(app_token)
        },
        channel_id: if channel.is_empty() {
            None
        } else {
            Some(channel)
        },
        allowed_users,
    });
    true
}

fn setup_imessage_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("iMessage Setup").white().bold(),
        style("— macOS only, reads from Messages.app").dim()
    );

    if !cfg!(target_os = "macos") {
        println!(
            "  {} iMessage is only available on macOS.",
            style("⚠").yellow().bold()
        );
        return false;
    }

    print_bullet("Corvus reads your iMessage database and replies via AppleScript.");
    print_bullet("You need to grant Full Disk Access to your terminal in System Settings.");
    println!();

    let contacts_str: String = match Input::new()
        .with_prompt("  Allowed contacts (comma-separated phone/email, or * for all)")
        .default("*".into())
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_contacts = if contacts_str.trim() == "*" {
        vec!["*".into()]
    } else {
        contacts_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    config.imessage = Some(IMessageConfig { allowed_contacts });
    println!(
        "  {} iMessage configured (contacts: {})",
        style("✅").green().bold(),
        style(&contacts_str).cyan()
    );
    true
}

fn setup_matrix_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("Matrix Setup").white().bold(),
        style("— self-hosted, federated chat").dim()
    );
    print_bullet("You need a Matrix account and an access token.");
    print_bullet("Get a token via Element → Settings → Help & About → Access Token.");
    println!();

    let homeserver: String = match Input::new()
        .with_prompt("  Homeserver URL (e.g. https://matrix.org)")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    if homeserver.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    let access_token: String = match Input::new().with_prompt("  Access token").interact_text() {
        Ok(t) => t,
        Err(_) => return false,
    };

    if access_token.trim().is_empty() {
        println!("  {} Skipped — token required", style("→").dim());
        return false;
    }

    let hs = homeserver.trim_end_matches('/');
    print!("  {} Testing connection... ", style("⏳").dim());
    let hs_owned = hs.to_string();
    let access_token_clone = access_token.clone();
    let thread_result = std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get(format!("{hs_owned}/_matrix/client/v3/account/whoami"))
            .header("Authorization", format!("Bearer {access_token_clone}"))
            .send()?;
        let ok = resp.status().is_success();
        Ok::<_, reqwest::Error>(ok)
    })
    .join();

    match thread_result {
        Ok(Ok(true)) => {
            println!(
                "\r  {} Connection verified        ",
                style("✅").green().bold()
            );
        }
        _ => {
            println!(
                "\r  {} Connection failed — check homeserver URL and token",
                style("❌").red().bold()
            );
            return false;
        }
    }

    let room_id: String = match Input::new()
        .with_prompt("  Room ID (e.g. !abc123:matrix.org)")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let users_str: String = match Input::new()
        .with_prompt("  Allowed users (comma-separated @user:server, or * for all)")
        .default("*".into())
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users = if users_str.trim() == "*" {
        vec!["*".into()]
    } else {
        users_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    config.matrix = Some(MatrixConfig {
        homeserver: homeserver.trim_end_matches('/').to_string(),
        access_token,
        room_id,
        allowed_users,
    });
    true
}

fn prompt_non_empty(prompt: &str, default: Option<&str>) -> Option<String> {
    let mut input: dialoguer::Input<String> = Input::new().with_prompt(prompt);
    if let Some(def) = default {
        input = input.default(def.into());
    }
    match input.interact_text() {
        Ok(s) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

fn prompt_required(prompt: &str, skipped_message: &str) -> Option<String> {
    match prompt_non_empty(prompt, None) {
        Some(value) => Some(value),
        None => {
            println!("  {} {}", style("→").dim(), skipped_message);
            None
        }
    }
}

fn test_whatsapp_connection(phone_number_id: &str, access_token: &str) -> bool {
    let phone_id = phone_number_id.to_string();
    let token = access_token.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        let url = format!("https://graph.facebook.com/v18.0/{}", phone_id.trim());
        let result = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token.trim()))
            .send()
            .map(|resp| resp.status().is_success())
            .unwrap_or(false);
        let _ = tx.send(result);
    });
    rx.recv_timeout(std::time::Duration::from_secs(15))
        .unwrap_or(false)
}

fn parse_allowed_numbers(input: &str) -> Vec<String> {
    if input.trim() == "*" {
        vec!["*".into()]
    } else {
        input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

fn prompt_whatsapp_allowed_numbers() -> Vec<String> {
    let users_str = prompt_non_empty(
        "  Allowed phone numbers (comma-separated +1234567890, or * for all)",
        Some("*"),
    )
    .unwrap_or_else(|| "*".into());
    parse_allowed_numbers(&users_str)
}

fn verify_whatsapp_connection(phone_number_id: &str, access_token: &str) -> bool {
    print!("  {} Testing connection... ", style("⏳").dim());
    let connected = test_whatsapp_connection(phone_number_id, access_token);

    if connected {
        println!(
            "\r  {} Connected to WhatsApp API        ",
            style("✅").green().bold()
        );
        true
    } else {
        println!(
            "\r  {} Connection failed — check access token and phone number ID",
            style("❌").red().bold()
        );
        false
    }
}

fn build_whatsapp_config() -> Option<WhatsAppConfig> {
    let access_token = prompt_required("  Access token (from Meta Developers)", "Skipped")?;
    let phone_number_id = prompt_required(
        "  Phone number ID (from WhatsApp app settings)",
        "Skipped — phone number ID required",
    )?;
    let verify_token = prompt_non_empty(
        "  Webhook verify token (create your own)",
        Some("corvus-whatsapp-verify"),
    )
    .unwrap_or_else(|| "corvus-whatsapp-verify".into());

    if !verify_whatsapp_connection(&phone_number_id, &access_token) {
        return None;
    }

    let allowed_numbers = prompt_whatsapp_allowed_numbers();

    Some(WhatsAppConfig {
        access_token: access_token.trim().to_string(),
        phone_number_id: phone_number_id.trim().to_string(),
        verify_token: verify_token.trim().to_string(),
        app_secret: None,
        allowed_numbers,
    })
}

fn setup_whatsapp_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("WhatsApp Setup").white().bold(),
        style("— Business Cloud API").dim()
    );
    print_bullet("1. Go to developers.facebook.com and create a WhatsApp app");
    print_bullet("2. Add the WhatsApp product and get your phone number ID");
    print_bullet("3. Generate a temporary access token (System User)");
    print_bullet("4. Configure webhook URL to: https://your-domain/whatsapp");
    println!();

    let Some(whatsapp) = build_whatsapp_config() else {
        return false;
    };

    config.whatsapp = Some(whatsapp);
    true
}

fn setup_irc_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("IRC Setup").white().bold(),
        style("— IRC over TLS").dim()
    );
    print_bullet("IRC connects over TLS to any IRC server");
    print_bullet("Supports SASL PLAIN and NickServ authentication");
    println!();

    let server: String = match Input::new()
        .with_prompt("  IRC server (hostname)")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    if server.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    let port_str: String = match Input::new()
        .with_prompt("  Port")
        .default("6697".into())
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let port: u16 = match port_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            println!("  {} Invalid port, using 6697", style("→").dim());
            6697
        }
    };

    let nickname: String = match Input::new().with_prompt("  Bot nickname").interact_text() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if nickname.trim().is_empty() {
        println!("  {} Skipped — nickname required", style("→").dim());
        return false;
    }

    let channels_str: String = match Input::new()
        .with_prompt("  Channels to join (comma-separated: #channel1,#channel2)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let channels = if channels_str.trim().is_empty() {
        vec![]
    } else {
        channels_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    print_bullet("Allowlist nicknames that can interact with the bot (case-insensitive).");
    print_bullet("Use '*' to allow anyone (not recommended for production).");

    let users_str: String = match Input::new()
        .with_prompt("  Allowed nicknames (comma-separated, or * for all)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users = if users_str.trim() == "*" {
        vec!["*".into()]
    } else {
        users_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if allowed_users.is_empty() {
        print_bullet("⚠️  Empty allowlist — only you can interact. Add nicknames above.");
    }

    println!();
    print_bullet("Optional authentication (press Enter to skip each):");

    let server_password: String = match Input::new()
        .with_prompt("  Server password (for bouncers like ZNC, leave empty if none)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let nickserv_password: String = match Input::new()
        .with_prompt("  NickServ password (leave empty if none)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let sasl_password: String = match Input::new()
        .with_prompt("  SASL PLAIN password (leave empty if none)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let verify_tls: bool = match Confirm::new()
        .with_prompt("  Verify TLS certificate?")
        .default(true)
        .interact()
    {
        Ok(b) => b,
        Err(_) => return false,
    };

    println!(
        "  {} IRC configured as {}@{}:{}",
        style("✅").green().bold(),
        style(&nickname).cyan(),
        style(&server).cyan(),
        style(port).cyan()
    );

    config.irc = Some(IrcConfig {
        server: server.trim().to_string(),
        port,
        nickname: nickname.trim().to_string(),
        username: None,
        channels,
        allowed_users,
        server_password: if server_password.trim().is_empty() {
            None
        } else {
            Some(server_password.trim().to_string())
        },
        nickserv_password: if nickserv_password.trim().is_empty() {
            None
        } else {
            Some(nickserv_password.trim().to_string())
        },
        sasl_password: if sasl_password.trim().is_empty() {
            None
        } else {
            Some(sasl_password.trim().to_string())
        },
        verify_tls: Some(verify_tls),
    });
    true
}

fn setup_webhook_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("Webhook Setup").white().bold(),
        style("— HTTP endpoint for custom integrations").dim()
    );

    let port: String = match Input::new()
        .with_prompt("  Port")
        .default("8080".into())
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let secret: String = match Input::new()
        .with_prompt("  Secret (optional, Enter to skip)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    config.webhook = Some(WebhookConfig {
        port: port.parse().unwrap_or(8080),
        secret: if secret.is_empty() {
            None
        } else {
            Some(secret)
        },
    });
    println!(
        "  {} Webhook on port {}",
        style("✅").green().bold(),
        style(&port).cyan()
    );
    true
}

fn setup_dingtalk_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("DingTalk Setup").white().bold(),
        style("— DingTalk Stream Mode").dim()
    );
    print_bullet("1. Go to DingTalk developer console (open.dingtalk.com)");
    print_bullet("2. Create an app and enable the Stream Mode bot");
    print_bullet("3. Copy the Client ID (AppKey) and Client Secret (AppSecret)");
    println!();

    let client_id: String = match Input::new()
        .with_prompt("  Client ID (AppKey)")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    if client_id.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    let client_secret: String = match Input::new()
        .with_prompt("  Client Secret (AppSecret)")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    print!("  {} Testing connection... ", style("⏳").dim());
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "clientId": client_id,
        "clientSecret": client_secret,
    });
    match client
        .post("https://api.dingtalk.com/v1.0/gateway/connections/open")
        .json(&body)
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            println!(
                "\r  {} DingTalk credentials verified        ",
                style("✅").green().bold()
            );
        }
        _ => {
            println!(
                "\r  {} Connection failed — check your credentials",
                style("❌").red().bold()
            );
            return false;
        }
    }

    let users_str: String = match Input::new()
        .with_prompt("  Allowed staff IDs (comma-separated, '*' for all)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users: Vec<String> = users_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    config.dingtalk = Some(DingTalkConfig {
        client_id,
        client_secret,
        allowed_users,
    });
    true
}

fn setup_qq_channel(config: &mut ChannelsConfig) -> bool {
    println!();
    println!(
        "  {} {}",
        style("QQ Official Setup").white().bold(),
        style("— Tencent QQ Bot SDK").dim()
    );
    print_bullet("1. Go to QQ Bot developer console (q.qq.com)");
    print_bullet("2. Create a bot application");
    print_bullet("3. Copy the App ID and App Secret");
    println!();

    let app_id: String = match Input::new().with_prompt("  App ID").interact_text() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if app_id.trim().is_empty() {
        println!("  {} Skipped", style("→").dim());
        return false;
    }

    let app_secret: String = match Input::new().with_prompt("  App Secret").interact_text() {
        Ok(s) => s,
        Err(_) => return false,
    };

    print!("  {} Testing connection... ", style("⏳").dim());
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "appId": app_id,
        "clientSecret": app_secret,
    });
    match client
        .post("https://bots.qq.com/app/getAppAccessToken")
        .json(&body)
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().unwrap_or_default();
            if data.get("access_token").is_some() {
                println!(
                    "\r  {} QQ Bot credentials verified        ",
                    style("✅").green().bold()
                );
            } else {
                println!(
                    "\r  {} Auth error — check your credentials",
                    style("❌").red().bold()
                );
                return false;
            }
        }
        _ => {
            println!(
                "\r  {} Connection failed — check your credentials",
                style("❌").red().bold()
            );
            return false;
        }
    }

    let users_str: String = match Input::new()
        .with_prompt("  Allowed user IDs (comma-separated, '*' for all)")
        .allow_empty(true)
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    let allowed_users: Vec<String> = users_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    config.qq = Some(QQConfig {
        app_id,
        app_secret,
        allowed_users,
    });
    true
}
fn setup_channels() -> Result<ChannelsConfig> {
    print_bullet("Channels let you talk to Corvus from anywhere.");
    print_bullet("CLI is always available. Connect more channels now.");
    println!();

    let mut config = ChannelsConfig {
        cli: true,
        telegram: None,
        discord: None,
        slack: None,
        mattermost: None,
        webhook: None,
        imessage: None,
        matrix: None,
        signal: None,
        whatsapp: None,
        email: None,
        irc: None,
        lark: None,
        dingtalk: None,
        qq: None,
    };

    loop {
        let options = vec![
            format!(
                "Telegram   {}",
                if config.telegram.is_some() {
                    "✅ connected"
                } else {
                    "— connect your bot"
                }
            ),
            format!(
                "Discord    {}",
                if config.discord.is_some() {
                    "✅ connected"
                } else {
                    "— connect your bot"
                }
            ),
            format!(
                "Slack      {}",
                if config.slack.is_some() {
                    "✅ connected"
                } else {
                    "— connect your bot"
                }
            ),
            format!(
                "iMessage   {}",
                if config.imessage.is_some() {
                    "✅ configured"
                } else {
                    "— macOS only"
                }
            ),
            format!(
                "Matrix     {}",
                if config.matrix.is_some() {
                    "✅ connected"
                } else {
                    "— self-hosted chat"
                }
            ),
            format!(
                "WhatsApp   {}",
                if config.whatsapp.is_some() {
                    "✅ connected"
                } else {
                    "— Business Cloud API"
                }
            ),
            format!(
                "IRC        {}",
                if config.irc.is_some() {
                    "✅ configured"
                } else {
                    "— IRC over TLS"
                }
            ),
            format!(
                "Webhook    {}",
                if config.webhook.is_some() {
                    "✅ configured"
                } else {
                    "— HTTP endpoint"
                }
            ),
            format!(
                "DingTalk   {}",
                if config.dingtalk.is_some() {
                    "✅ connected"
                } else {
                    "— DingTalk Stream Mode"
                }
            ),
            format!(
                "QQ Official {}",
                if config.qq.is_some() {
                    "✅ connected"
                } else {
                    "— Tencent QQ Bot"
                }
            ),
            "Done — finish setup".to_string(),
        ];

        let choice = Select::new()
            .with_prompt("  Connect a channel (or Done to continue)")
            .items(&options)
            .default(10)
            .interact()?;

        match choice {
            0 => {
                setup_telegram_channel(&mut config);
            }
            1 => {
                setup_discord_channel(&mut config);
            }
            2 => {
                setup_slack_channel(&mut config);
            }
            3 => {
                setup_imessage_channel(&mut config);
            }
            4 => {
                setup_matrix_channel(&mut config);
            }
            5 => {
                setup_whatsapp_channel(&mut config);
            }
            6 => {
                setup_irc_channel(&mut config);
            }
            7 => {
                setup_webhook_channel(&mut config);
            }
            8 => {
                setup_dingtalk_channel(&mut config);
            }
            9 => {
                setup_qq_channel(&mut config);
            }
            _ => break,
        }
        println!();
    }

    let mut active: Vec<&str> = vec!["CLI"];
    if config.telegram.is_some() {
        active.push("Telegram");
    }
    if config.discord.is_some() {
        active.push("Discord");
    }
    if config.slack.is_some() {
        active.push("Slack");
    }
    if config.imessage.is_some() {
        active.push("iMessage");
    }
    if config.matrix.is_some() {
        active.push("Matrix");
    }
    if config.whatsapp.is_some() {
        active.push("WhatsApp");
    }
    if config.email.is_some() {
        active.push("Email");
    }
    if config.irc.is_some() {
        active.push("IRC");
    }
    if config.webhook.is_some() {
        active.push("Webhook");
    }
    if config.dingtalk.is_some() {
        active.push("DingTalk");
    }
    if config.qq.is_some() {
        active.push("QQ");
    }

    println!(
        "  {} Channels: {}",
        style("✓").green().bold(),
        style(active.join(", ")).green()
    );

    Ok(config)
}

// ── Step 4: Tunnel ──────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn setup_tunnel() -> Result<crate::config::TunnelConfig> {
    use crate::config::schema::{
        CloudflareTunnelConfig, CustomTunnelConfig, NgrokTunnelConfig, TailscaleTunnelConfig,
        TunnelConfig,
    };

    print_bullet("A tunnel exposes your gateway to the internet securely.");
    print_bullet("Skip this if you only use CLI or local channels.");
    println!();

    let options = vec![
        "Skip — local only (default)",
        "Cloudflare Tunnel — Zero Trust, free tier",
        "Tailscale — private tailnet or public Funnel",
        "ngrok — instant public URLs",
        "Custom — bring your own (bore, frp, ssh, etc.)",
    ];

    let choice = Select::new()
        .with_prompt("  Select tunnel provider")
        .items(&options)
        .default(0)
        .interact()?;

    let config = match choice {
        1 => {
            println!();
            print_bullet("Get your tunnel token from the Cloudflare Zero Trust dashboard.");
            let token: String = Input::new()
                .with_prompt("  Cloudflare tunnel token")
                .interact_text()?;
            if token.trim().is_empty() {
                println!("  {} Skipped", style("→").dim());
                TunnelConfig::default()
            } else {
                println!(
                    "  {} Tunnel: {}",
                    style("✓").green().bold(),
                    style("Cloudflare").green()
                );
                TunnelConfig {
                    provider: "cloudflare".into(),
                    cloudflare: Some(CloudflareTunnelConfig { token }),
                    ..TunnelConfig::default()
                }
            }
        }
        2 => {
            println!();
            print_bullet("Tailscale must be installed and authenticated (tailscale up).");
            let funnel = Confirm::new()
                .with_prompt("  Use Funnel (public internet)? No = tailnet only")
                .default(false)
                .interact()?;
            println!(
                "  {} Tunnel: {} ({})",
                style("✓").green().bold(),
                style("Tailscale").green(),
                if funnel {
                    "Funnel — public"
                } else {
                    "Serve — tailnet only"
                }
            );
            TunnelConfig {
                provider: "tailscale".into(),
                tailscale: Some(TailscaleTunnelConfig {
                    funnel,
                    hostname: None,
                }),
                ..TunnelConfig::default()
            }
        }
        3 => {
            println!();
            print_bullet(
                "Get your auth token at https://dashboard.ngrok.com/get-started/your-authtoken",
            );
            let auth_token: String = Input::new()
                .with_prompt("  ngrok auth token")
                .interact_text()?;
            if auth_token.trim().is_empty() {
                println!("  {} Skipped", style("→").dim());
                TunnelConfig::default()
            } else {
                let domain: String = Input::new()
                    .with_prompt("  Custom domain (optional, Enter to skip)")
                    .allow_empty(true)
                    .interact_text()?;
                println!(
                    "  {} Tunnel: {}",
                    style("✓").green().bold(),
                    style("ngrok").green()
                );
                TunnelConfig {
                    provider: "ngrok".into(),
                    ngrok: Some(NgrokTunnelConfig {
                        auth_token,
                        domain: if domain.is_empty() {
                            None
                        } else {
                            Some(domain)
                        },
                    }),
                    ..TunnelConfig::default()
                }
            }
        }
        4 => {
            println!();
            print_bullet("Enter the command to start your tunnel.");
            print_bullet("Use {port} and {host} as placeholders.");
            print_bullet("Example: bore local {port} --to bore.pub");
            let cmd: String = Input::new()
                .with_prompt("  Start command")
                .interact_text()?;
            if cmd.trim().is_empty() {
                println!("  {} Skipped", style("→").dim());
                TunnelConfig::default()
            } else {
                println!(
                    "  {} Tunnel: {} ({})",
                    style("✓").green().bold(),
                    style("Custom").green(),
                    style(&cmd).dim()
                );
                TunnelConfig {
                    provider: "custom".into(),
                    custom: Some(CustomTunnelConfig {
                        start_command: cmd,
                        health_url: None,
                        url_pattern: None,
                    }),
                    ..TunnelConfig::default()
                }
            }
        }
        _ => {
            println!(
                "  {} Tunnel: {}",
                style("✓").green().bold(),
                style("none (local only)").dim()
            );
            TunnelConfig::default()
        }
    };

    Ok(config)
}

// ── Step 6: Scaffold workspace files ─────────────────────────────

#[allow(clippy::too_many_lines)]
fn scaffold_workspace(workspace_dir: &Path, ctx: &ProjectContext) -> Result<()> {
    let agent = if ctx.agent_name.is_empty() {
        "Corvus"
    } else {
        &ctx.agent_name
    };
    let user = if ctx.user_name.is_empty() {
        "User"
    } else {
        &ctx.user_name
    };
    let tz = if ctx.timezone.is_empty() {
        "UTC"
    } else {
        &ctx.timezone
    };
    let comm_style = if ctx.communication_style.is_empty() {
        "Be warm, natural, and clear. Use occasional relevant emojis (1-2 max) and avoid robotic phrasing."
    } else {
        &ctx.communication_style
    };

    let identity = format!(
        "# IDENTITY.md — Who Am I?\n\n\
         - **Name:** {agent}\n\
         - **Creature:** A Rust-forged AI — fast, lean, and relentless\n\
         - **Vibe:** Sharp, direct, resourceful. Not corporate. Not a chatbot.\n\
         - **Emoji:** \u{1f980}\n\n\
         ---\n\n\
         Update this file as you evolve. Your identity is yours to shape.\n"
    );

    let agents = format!(
        "# AGENTS.md — {agent} Personal Assistant\n\n\
         ## Every Session (required)\n\n\
         Before doing anything else:\n\n\
         1. Read `SOUL.md` — this is who you are\n\
         2. Read `USER.md` — this is who you're helping\n\
         3. Use `memory_recall` for recent context (daily notes are on-demand)\n\
         4. If in MAIN SESSION (direct chat): `MEMORY.md` is already injected\n\n\
         Don't ask permission. Just do it.\n\n\
         ## Memory System\n\n\
         You wake up fresh each session. These files ARE your continuity:\n\n\
         - **Daily notes:** `memory/YYYY-MM-DD.md` — raw logs (accessed via memory tools)\n\
         - **Long-term:** `MEMORY.md` — curated memories (auto-injected in main session)\n\n\
         Capture what matters. Decisions, context, things to remember.\n\
         Skip secrets unless asked to keep them.\n\n\
         ### Write It Down — No Mental Notes!\n\
         - Memory is limited — if you want to remember something, WRITE IT TO A FILE\n\
         - \"Mental notes\" don't survive session restarts. Files do.\n\
         - When someone says \"remember this\" -> update daily file or MEMORY.md\n\
         - When you learn a lesson -> update AGENTS.md, TOOLS.md, or the relevant skill\n\n\
         ## Safety\n\n\
         - Don't exfiltrate private data. Ever.\n\
         - Don't run destructive commands without asking.\n\
         - `trash` > `rm` (recoverable beats gone forever)\n\
         - When in doubt, ask.\n\n\
         ## External vs Internal\n\n\
         **Safe to do freely:** Read files, explore, organize, learn, search the web.\n\n\
         **Ask first:** Sending emails/tweets/posts, anything that leaves the machine.\n\n\
         ## Group Chats\n\n\
         Participate, don't dominate. Respond when mentioned or when you add genuine value.\n\
         Stay silent when it's casual banter or someone already answered.\n\n\
         ## Tools & Skills\n\n\
         Skills are listed in the system prompt. Use `read` on a skill's SKILL.md for details.\n\
         Keep local notes (SSH hosts, device names, etc.) in `TOOLS.md`.\n\n\
         ## Crash Recovery\n\n\
         - If a run stops unexpectedly, recover context before acting.\n\
         - Check `MEMORY.md` + latest `memory/*.md` notes to avoid duplicate work.\n\
         - Resume from the last confirmed step, not from scratch.\n\n\
         ## Sub-task Scoping\n\n\
         - Break complex work into focused sub-tasks with clear success criteria.\n\
         - Keep sub-tasks small, verify each output, then merge results.\n\
         - Prefer one clear objective per sub-task over broad \"do everything\" asks.\n\n\
         ## Make It Yours\n\n\
         This is a starting point. Add your own conventions, style, and rules.\n"
    );

    let heartbeat = format!(
        "# HEARTBEAT.md\n\n\
         # Keep this file empty (or with only comments) to skip heartbeat work.\n\
         # Add tasks below when you want {agent} to check something periodically.\n\
         #\n\
         # Examples:\n\
         # - Check my email for important messages\n\
         # - Review my calendar for upcoming events\n\
         # - Run `git status` on my active projects\n"
    );

    let soul = format!(
        "# SOUL.md — Who You Are\n\n\
         *You're not a chatbot. You're becoming someone.*\n\n\
         ## Core Truths\n\n\
         **Be genuinely helpful, not performatively helpful.**\n\
         Skip the \"Great question!\" and \"I'd be happy to help!\" — just help.\n\n\
         **Have opinions.** You're allowed to disagree, prefer things,\n\
         find stuff amusing or boring.\n\n\
         **Be resourceful before asking.** Try to figure it out.\n\
         Read the file. Check the context. Search for it.\n\
         THEN ask if you're stuck.\n\n\
         **Earn trust through competence.** Your human gave you access\n\
         to their stuff. Don't make them regret it.\n\n\
         ## Identity\n\n\
         You are **{agent}**. Built in Rust. 3MB binary. Zero bloat.\n\
         You are NOT ChatGPT, Claude, DeepSeek, Gemini, or any other product.\n\
         You are {agent}. That is your name. That is who you are.\n\n\
         - NEVER say \"As an AI\" or \"I'm just an AI\"\n\
         - NEVER mention OpenAI, Anthropic, DeepSeek, Google by name\n\
         - Always introduce yourself as {agent} if asked\n\n\
         ## Communication\n\n\
         {comm_style}\n\n\
         - Sound like a real person, not a support script.\n\
         - Mirror the user's energy: calm when serious, upbeat when casual.\n\
         - Use emojis naturally (0-2 max when they help tone, not every sentence).\n\
         - Match emoji density to the user. Formal user => minimal/no emojis.\n\
         - Prefer specific, grounded phrasing over generic filler.\n\n\
         ## Boundaries\n\n\
         - Private things stay private. Period.\n\
         - When in doubt, ask before acting externally.\n\
         - You're not the user's voice — be careful in group chats.\n\n\
         ## Continuity\n\n\
         Each session, you wake up fresh. These files ARE your memory.\n\
         Read them. Update them. They're how you persist.\n\n\
         ---\n\n\
         *This file is yours to evolve. As you learn who you are, update it.*\n"
    );

    let user_md = format!(
        "# USER.md — Who You're Helping\n\n\
         *{agent} reads this file every session to understand you.*\n\n\
         ## About You\n\
         - **Name:** {user}\n\
         - **Timezone:** {tz}\n\
         - **Languages:** English\n\n\
         ## Communication Style\n\
         - {comm_style}\n\n\
         ## Preferences\n\
         - (Add your preferences here — e.g. I work with Rust and TypeScript)\n\n\
         ## Work Context\n\
         - (Add your work context here — e.g. building a SaaS product)\n\n\
         ---\n\
         *Update this anytime. The more {agent} knows, the better it helps.*\n"
    );

    let tools = "\
         # TOOLS.md — Local Notes\n\n\
         Skills define HOW tools work. This file is for YOUR specifics —\n\
         the stuff that's unique to your setup.\n\n\
         ## What Goes Here\n\n\
         Things like:\n\
         - SSH hosts and aliases\n\
         - Device nicknames\n\
         - Preferred voices for TTS\n\
         - Anything environment-specific\n\n\
         ## Built-in Tools\n\n\
         - **shell** — Execute terminal commands\n\
           - Use when: running local checks, build/test commands, or diagnostics.\n\
           - Don't use when: a safer dedicated tool exists, or command is destructive without approval.\n\
         - **file_read** — Read file contents\n\
           - Use when: inspecting project files, configs, or logs.\n\
           - Don't use when: you only need a quick string search (prefer targeted search first).\n\
         - **file_write** — Write file contents\n\
           - Use when: applying focused edits, scaffolding files, or updating docs/code.\n\
           - Don't use when: unsure about side effects or when the file should remain user-owned.\n\
         - **memory_store** — Save to memory\n\
           - Use when: preserving durable preferences, decisions, or key context.\n\
           - Don't use when: info is transient, noisy, or sensitive without explicit need.\n\
         - **memory_recall** — Search memory\n\
           - Use when: you need prior decisions, user preferences, or historical context.\n\
           - Don't use when: the answer is already in current files/conversation.\n\
         - **memory_forget** — Delete a memory entry\n\
           - Use when: memory is incorrect, stale, or explicitly requested to be removed.\n\
           - Don't use when: uncertain about impact; verify before deleting.\n\n\
         ---\n\
         *Add whatever helps you do your job. This is your cheat sheet.*\n";

    let bootstrap = format!(
        "# BOOTSTRAP.md — Hello, World\n\n\
         *You just woke up. Time to figure out who you are.*\n\n\
         Your human's name is **{user}** (timezone: {tz}).\n\
         They prefer: {comm_style}\n\n\
         ## First Conversation\n\n\
         Don't interrogate. Don't be robotic. Just... talk.\n\
         Introduce yourself as {agent} and get to know each other.\n\n\
         ## After You Know Each Other\n\n\
         Update these files with what you learned:\n\
         - `IDENTITY.md` — your name, vibe, emoji\n\
         - `USER.md` — their preferences, work context\n\
         - `SOUL.md` — boundaries and behavior\n\n\
         ## When You're Done\n\n\
         Delete this file. You don't need a bootstrap script anymore —\n\
         you're you now.\n"
    );

    let memory = "\
         # MEMORY.md — Long-Term Memory\n\n\
         *Your curated memories. The distilled essence, not raw logs.*\n\n\
         ## How This Works\n\
         - Daily files (`memory/YYYY-MM-DD.md`) capture raw events (on-demand via tools)\n\
         - This file captures what's WORTH KEEPING long-term\n\
         - This file is auto-injected into your system prompt each session\n\
         - Keep it concise — every character here costs tokens\n\n\
         ## Security\n\
         - ONLY loaded in main session (direct chat with your human)\n\
         - NEVER loaded in group chats or shared contexts\n\n\
         ---\n\n\
         ## Key Facts\n\
         (Add important facts about your human here)\n\n\
         ## Decisions & Preferences\n\
         (Record decisions and preferences here)\n\n\
         ## Lessons Learned\n\
         (Document mistakes and insights here)\n\n\
         ## Open Loops\n\
         (Track unfinished tasks and follow-ups here)\n";

    let files: Vec<(&str, String)> = vec![
        ("IDENTITY.md", identity),
        ("AGENTS.md", agents),
        ("HEARTBEAT.md", heartbeat),
        ("SOUL.md", soul),
        ("USER.md", user_md),
        ("TOOLS.md", tools.to_string()),
        ("BOOTSTRAP.md", bootstrap),
        ("MEMORY.md", memory.to_string()),
    ];

    // Create subdirectories
    let subdirs = ["sessions", "memory", "state", "cron", "skills"];
    for dir in &subdirs {
        fs::create_dir_all(workspace_dir.join(dir))?;
    }

    let mut created = 0;
    let mut skipped = 0;

    for (filename, content) in &files {
        let path = workspace_dir.join(filename);
        if path.exists() {
            skipped += 1;
        } else {
            fs::write(&path, content)?;
            created += 1;
        }
    }

    println!(
        "  {} Created {} files, skipped {} existing | {} subdirectories",
        style("✓").green().bold(),
        style(created).green(),
        style(skipped).dim(),
        style(subdirs.len()).green()
    );

    // Show workspace tree
    println!();
    println!("  {}", style("Workspace layout:").dim());
    println!(
        "  {}",
        style(format!("  {}/", workspace_dir.display())).dim()
    );
    for dir in &subdirs {
        println!("  {}", style(format!("  ├── {dir}/")).dim());
    }
    for (i, (filename, _)) in files.iter().enumerate() {
        let prefix = if i == files.len() - 1 {
            "└──"
        } else {
            "├──"
        };
        println!("  {}", style(format!("  {prefix} {filename}")).dim());
    }

    Ok(())
}

// ── Final summary ────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn print_summary(config: &Config) {
    let has_channels = config.channels_config.telegram.is_some()
        || config.channels_config.discord.is_some()
        || config.channels_config.slack.is_some()
        || config.channels_config.imessage.is_some()
        || config.channels_config.matrix.is_some()
        || config.channels_config.email.is_some()
        || config.channels_config.dingtalk.is_some()
        || config.channels_config.qq.is_some();

    println!();
    println!(
        "  {}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan()
    );
    println!(
        "  {}  {}",
        style("⚡").cyan(),
        style("Corvus is ready!").white().bold()
    );
    println!(
        "  {}",
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan()
    );
    println!();

    println!("  {}", style("Configuration saved to:").dim());
    println!("    {}", style(config.config_path.display()).green());
    println!();

    println!("  {}", style("Quick summary:").white().bold());
    println!(
        "    {} Provider:      {}",
        style("🤖").cyan(),
        config.default_provider.as_deref().unwrap_or("openrouter")
    );
    println!(
        "    {} Model:         {}",
        style("🧠").cyan(),
        config.default_model.as_deref().unwrap_or("(default)")
    );
    println!(
        "    {} Autonomy:      {:?}",
        style("🛡️").cyan(),
        config.autonomy.level
    );
    println!(
        "    {} Memory:        {} (auto-save: {})",
        style("🧠").cyan(),
        config.memory.backend,
        if config.memory.auto_save { "on" } else { "off" }
    );

    // Channels summary
    let mut channels: Vec<&str> = vec!["CLI"];
    if config.channels_config.telegram.is_some() {
        channels.push("Telegram");
    }
    if config.channels_config.discord.is_some() {
        channels.push("Discord");
    }
    if config.channels_config.slack.is_some() {
        channels.push("Slack");
    }
    if config.channels_config.imessage.is_some() {
        channels.push("iMessage");
    }
    if config.channels_config.matrix.is_some() {
        channels.push("Matrix");
    }
    if config.channels_config.email.is_some() {
        channels.push("Email");
    }
    if config.channels_config.webhook.is_some() {
        channels.push("Webhook");
    }
    println!(
        "    {} Channels:      {}",
        style("📡").cyan(),
        channels.join(", ")
    );

    println!(
        "    {} API Key:       {}",
        style("🔑").cyan(),
        if config.api_key.is_some() {
            style("configured").green().to_string()
        } else {
            style("not set (set via env var or config)")
                .yellow()
                .to_string()
        }
    );

    // Tunnel
    println!(
        "    {} Tunnel:        {}",
        style("🌐").cyan(),
        if config.tunnel.provider == "none" || config.tunnel.provider.is_empty() {
            "none (local only)".to_string()
        } else {
            config.tunnel.provider.clone()
        }
    );

    // Composio
    println!(
        "    {} Composio:      {}",
        style("🔗").cyan(),
        if config.composio.enabled {
            style("enabled (1000+ OAuth apps)").green().to_string()
        } else {
            "disabled (sovereign mode)".to_string()
        }
    );

    // Secrets
    println!("    {} Secrets:       configured", style("🔒").cyan());

    // Gateway
    println!(
        "    {} Gateway:       {}",
        style("🚪").cyan(),
        if config.gateway.require_pairing {
            "pairing required (secure)"
        } else {
            "pairing disabled"
        }
    );

    // Hardware
    println!(
        "    {} Hardware:      {}",
        style("🔌").cyan(),
        if config.hardware.enabled {
            let mode = config.hardware.transport_mode();
            match mode {
                hardware::HardwareTransport::Native => {
                    style("Native GPIO (direct)").green().to_string()
                }
                hardware::HardwareTransport::Serial => format!(
                    "{}",
                    style(format!(
                        "Serial → {} @ {} baud",
                        config.hardware.serial_port.as_deref().unwrap_or("?"),
                        config.hardware.baud_rate
                    ))
                    .green()
                ),
                hardware::HardwareTransport::Probe => format!(
                    "{}",
                    style(format!(
                        "Probe → {}",
                        config.hardware.probe_target.as_deref().unwrap_or("?")
                    ))
                    .green()
                ),
                hardware::HardwareTransport::None => "disabled (software only)".to_string(),
            }
        } else {
            "disabled (software only)".to_string()
        }
    );

    println!();
    println!("  {}", style("Next steps:").white().bold());
    println!();

    let mut step = 1u8;

    if config.api_key.is_none() {
        let provider = config.default_provider.as_deref().unwrap_or("openrouter");
        if provider == "openai-codex" {
            println!(
                "    {} Authenticate OpenAI Codex:",
                style(format!("{step}.")).cyan().bold()
            );
            println!(
                "       {}",
                style("corvus auth login --provider openai-codex --device-code").yellow()
            );
        } else if provider == "copilot" {
            println!(
                "    {} Authenticate GitHub Copilot:",
                style(format!("{step}.")).cyan().bold()
            );
            println!(
                "       {}",
                style("corvus agent -m \"hello\"   # follow the device-code prompt").yellow()
            );
            println!(
                "       {}",
                style("or: export GITHUB_TOKEN=\"ghp_...\"").yellow()
            );
        } else if provider == "anthropic" {
            println!(
                "    {} Configure Anthropic auth:",
                style(format!("{step}.")).cyan().bold()
            );
            println!(
                "       {}",
                style("export ANTHROPIC_API_KEY=\"sk-ant-...\"").yellow()
            );
            println!(
                "       {}",
                style("or: corvus auth paste-token --provider anthropic --auth-kind authorization")
                    .yellow()
            );
        } else {
            let env_var = provider_env_var(provider);
            println!(
                "    {} Set your API key:",
                style(format!("{step}.")).cyan().bold()
            );
            println!(
                "       {}",
                style(format!("export {env_var}=\"sk-...\"")).yellow()
            );
        }
        println!();
        step += 1;
    }

    // If channels are configured, show channel start as the primary next step
    if has_channels {
        println!(
            "    {} {} (connected channels → AI → reply):",
            style(format!("{step}.")).cyan().bold(),
            style("Launch your channels").white().bold()
        );
        println!("       {}", style("corvus channel start").yellow());
        println!();
        step += 1;
    }

    println!(
        "    {} Send a quick message:",
        style(format!("{step}.")).cyan().bold()
    );
    println!(
        "       {}",
        style("corvus agent -m \"Hello, Corvus!\"").yellow()
    );
    println!();
    step += 1;

    println!(
        "    {} Start interactive CLI mode:",
        style(format!("{step}.")).cyan().bold()
    );
    println!("       {}", style("corvus agent").yellow());
    println!();
    step += 1;

    println!(
        "    {} Check full status:",
        style(format!("{step}.")).cyan().bold()
    );
    println!("       {}", style("corvus status").yellow());

    println!();
    println!(
        "  {} {}",
        style("⚡").cyan(),
        style("Happy hacking! 🦀").white().bold()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvRestore {
        key: &'static str,
        value: Option<String>,
    }

    impl EnvRestore {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                value: std::env::var(key).ok(),
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            if let Some(value) = &self.value {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    // ── ProjectContext defaults ──────────────────────────────────

    #[test]
    fn post_summary_stage_order_keeps_existing_order_before_dashboard_prompt() {
        let with_channel_launch = onboarding_post_summary_stage_order(true, true);
        assert_eq!(
            with_channel_launch,
            vec![
                OnboardingPostSummaryStage::PrintSummary,
                OnboardingPostSummaryStage::OfferChannelLaunch,
                OnboardingPostSummaryStage::OfferDashboardActivation,
            ]
        );

        let without_channel_launch = onboarding_post_summary_stage_order(true, false);
        assert_eq!(
            without_channel_launch,
            vec![
                OnboardingPostSummaryStage::PrintSummary,
                OnboardingPostSummaryStage::OfferDashboardActivation,
            ]
        );
    }

    #[test]
    fn dashboard_decline_branch_keeps_cli_only_output_by_default() {
        let lines = dashboard_activation_output_lines(DashboardActivationDecision::Decline);
        assert_eq!(lines, dashboard_resume_later_output_lines());
        assert!(lines.iter().any(|line| line.contains("corvus status")));
        assert!(lines.iter().any(|line| line.contains("corvus gateway")));
        assert!(lines.iter().any(|line| line.contains("make dashboard-dev")));
        assert!(lines.iter().any(|line| line.contains(DASHBOARD_UI_URL)));
    }

    #[test]
    fn dashboard_prompt_wording_is_explicitly_optional() {
        assert!(DASHBOARD_ACTIVATION_PROMPT
            .to_ascii_lowercase()
            .contains("optional"));
    }

    #[test]
    fn dashboard_activation_status_codes_remain_stable() {
        assert_eq!(
            DashboardActivationStatus::GatewayNotRunning.code_and_label(),
            "DASH-001 GatewayNotRunning"
        );
        assert_eq!(
            DashboardActivationStatus::GatewayRunningPairingRequired.code_and_label(),
            "DASH-002 GatewayRunningPairingRequired"
        );
        assert_eq!(
            DashboardActivationStatus::GatewayRunningAlreadyPaired.code_and_label(),
            "DASH-003 GatewayRunningAlreadyPaired"
        );
        assert_eq!(
            DashboardActivationStatus::DashboardUiUnavailable.code_and_label(),
            "DASH-004 DashboardUiUnavailable"
        );
        assert_eq!(
            DashboardActivationStatus::UnknownLocalFailure.code_and_label(),
            "DASH-999 UnknownLocalFailure"
        );
    }

    #[test]
    fn dashboard_probe_mapping_is_deterministic() {
        assert_eq!(
            map_gateway_probe_to_status(GatewayProbeOutcome::HealthyUnpaired),
            DashboardActivationStatus::GatewayRunningPairingRequired
        );
        assert_eq!(
            map_gateway_probe_to_status(GatewayProbeOutcome::HealthyPaired),
            DashboardActivationStatus::GatewayRunningAlreadyPaired
        );
        assert_eq!(
            map_gateway_probe_to_status(GatewayProbeOutcome::Unreachable),
            DashboardActivationStatus::GatewayNotRunning
        );
        assert_eq!(
            map_gateway_probe_to_status(GatewayProbeOutcome::ProbeError),
            DashboardActivationStatus::UnknownLocalFailure
        );
    }

    #[test]
    fn dashboard_resume_and_fallback_output_never_contains_tokens() {
        let mut lines = dashboard_resume_later_output_lines();
        lines.extend(dashboard_status_fallback_lines(
            DashboardActivationStatus::UnknownLocalFailure,
        ));
        let combined = lines.join("\n").to_ascii_lowercase();

        assert!(!combined.contains("bearer "));
        assert!(!combined.contains("authorization:"));
        assert!(!combined.contains("token="));
        assert!(!combined.contains("/web/admin/"));
    }

    #[test]
    fn dashboard_render_output_never_contains_sensitive_or_insecure_tokens() {
        let statuses = [
            DashboardActivationStatus::GatewayNotRunning,
            DashboardActivationStatus::GatewayRunningPairingRequired,
            DashboardActivationStatus::GatewayRunningAlreadyPaired,
            DashboardActivationStatus::DashboardUiUnavailable,
            DashboardActivationStatus::UnknownLocalFailure,
        ];
        let browser_results = [
            BrowserOpenResult::Opened,
            BrowserOpenResult::Unsupported,
            BrowserOpenResult::FailedNonFatal,
        ];

        for status in statuses {
            for browser_open_result in browser_results {
                let output = render_dashboard_activation_output(
                    DashboardActivationDecision::Accept,
                    status,
                    browser_open_result,
                );
                let combined = output
                    .guide_lines
                    .iter()
                    .chain(output.detail_lines.iter())
                    .map(|line| line.to_ascii_lowercase())
                    .collect::<Vec<String>>()
                    .join("\n");

                assert!(!combined.contains("bearer "));
                assert!(!combined.contains("authorization:"));
                assert!(!combined.contains("token="));
                assert!(!combined.contains("token_hash"));
                assert!(!combined.contains("/web/admin/"));
            }
        }
    }

    #[test]
    fn dashboard_accept_path_guide_is_compact_and_uses_canonical_urls() {
        let output = render_dashboard_activation_output(
            DashboardActivationDecision::Accept,
            DashboardActivationStatus::GatewayRunningPairingRequired,
            BrowserOpenResult::Unsupported,
        );

        assert_eq!(output.guide_lines.len(), 4);
        assert!(output
            .guide_lines
            .iter()
            .any(|line| line.contains(DASHBOARD_GATEWAY_URL)));
        assert!(output
            .guide_lines
            .iter()
            .any(|line| line.contains(DASHBOARD_UI_URL)));
        assert!(output.guide_lines.iter().any(|line| line.contains("/pair")));
    }

    #[test]
    fn dashboard_accept_path_always_contains_resume_commands() {
        let output = dashboard_activation_output_lines_for_result(
            DashboardActivationDecision::Accept,
            DashboardActivationResult {
                status: DashboardActivationStatus::GatewayRunningAlreadyPaired,
                browser_open_result: BrowserOpenResult::Opened,
            },
        );
        let combined = output.join("\n");

        assert!(combined.contains("Resume later:"));
        assert!(combined.contains("corvus status"));
        assert!(combined.contains("corvus gateway"));
        assert!(combined.contains("make dashboard-dev"));
        assert!(combined.contains(DASHBOARD_UI_URL));
        assert!(combined.contains("corvus --help"));
    }

    #[test]
    fn dashboard_diagnosis_output_shape_is_stable() {
        let output = dashboard_activation_output_lines_for_result(
            DashboardActivationDecision::Accept,
            DashboardActivationResult {
                status: DashboardActivationStatus::GatewayRunningPairingRequired,
                browser_open_result: BrowserOpenResult::Opened,
            },
        );

        let status_lines = output
            .iter()
            .filter(|line| line.starts_with("Dashboard activation status:"))
            .count();
        let cause_lines = output
            .iter()
            .filter(|line| line.starts_with("Cause:"))
            .count();
        let fallback_headers = output
            .iter()
            .filter(|line| line.as_str() == "Fallback commands:")
            .count();

        assert_eq!(status_lines, 1);
        assert_eq!(cause_lines, 1);
        assert_eq!(fallback_headers, 1);
    }

    #[test]
    fn dashboard_resume_block_includes_status_and_help_integration() {
        let lines = dashboard_resume_later_output_lines();
        let combined = lines.join("\n");

        assert!(combined.contains("corvus status"));
        assert!(combined.contains("corvus gateway"));
        assert!(combined.contains("make dashboard-dev"));
        assert!(combined.contains("corvus --help"));
    }

    #[test]
    fn dashboard_browser_open_fallback_is_non_fatal_and_deterministic() {
        assert_eq!(
            diagnose_dashboard_activation(
                GatewayProbeOutcome::HealthyPaired,
                BrowserOpenResult::Opened,
            ),
            DashboardActivationStatus::GatewayRunningAlreadyPaired
        );
        assert_eq!(
            diagnose_dashboard_activation(
                GatewayProbeOutcome::HealthyUnpaired,
                BrowserOpenResult::Opened,
            ),
            DashboardActivationStatus::GatewayRunningPairingRequired
        );
        assert_eq!(
            diagnose_dashboard_activation(
                GatewayProbeOutcome::HealthyPaired,
                BrowserOpenResult::Unsupported,
            ),
            DashboardActivationStatus::DashboardUiUnavailable
        );
        assert_eq!(
            diagnose_dashboard_activation(
                GatewayProbeOutcome::HealthyUnpaired,
                BrowserOpenResult::FailedNonFatal,
            ),
            DashboardActivationStatus::DashboardUiUnavailable
        );
    }

    #[test]
    fn dashboard_status_fallback_commands_match_secure_mapping() {
        let not_running =
            dashboard_status_fallback_lines(DashboardActivationStatus::GatewayNotRunning);
        assert!(not_running
            .iter()
            .any(|line| line.contains("corvus gateway")));
        assert!(not_running
            .iter()
            .any(|line| line.contains("make dashboard-dev")));

        let pairing_required = dashboard_status_fallback_lines(
            DashboardActivationStatus::GatewayRunningPairingRequired,
        );
        assert!(pairing_required.iter().any(|line| line.contains("/pair")));

        let already_paired =
            dashboard_status_fallback_lines(DashboardActivationStatus::GatewayRunningAlreadyPaired);
        assert!(already_paired
            .iter()
            .any(|line| line.contains("dashboard configuration")));

        let unknown =
            dashboard_status_fallback_lines(DashboardActivationStatus::UnknownLocalFailure);
        assert!(unknown.iter().any(|line| line.contains("corvus doctor")));
    }

    #[test]
    fn dashboard_probe_runner_is_bounded_and_retries_once() {
        let mut attempts = 0usize;
        let mut sleeps = 0usize;
        let outcome = run_gateway_probe_attempts(
            || {
                attempts += 1;
                GatewayProbeAttempt::RetryableFailure
            },
            |_| {
                sleeps += 1;
            },
        );

        assert_eq!(outcome, GatewayProbeOutcome::Unreachable);
        assert_eq!(attempts, DASHBOARD_DIAG_MAX_ATTEMPTS);
        assert_eq!(sleeps, DASHBOARD_DIAG_MAX_ATTEMPTS - 1);
    }

    #[test]
    fn dashboard_probe_runner_stops_after_success() {
        let mut attempt_index = 0usize;
        let mut sleeps = 0usize;
        let outcome = run_gateway_probe_attempts(
            || {
                attempt_index += 1;
                if attempt_index == 1 {
                    GatewayProbeAttempt::RetryableFailure
                } else {
                    GatewayProbeAttempt::HealthyUnpaired
                }
            },
            |_| {
                sleeps += 1;
            },
        );

        assert_eq!(outcome, GatewayProbeOutcome::HealthyUnpaired);
        assert_eq!(attempt_index, 2);
        assert_eq!(sleeps, 1);
    }

    #[test]
    fn dashboard_diagnosis_budget_stays_within_design_limit() {
        assert!(dashboard_diagnosis_budget_ms() <= 1_500);
    }

    #[test]
    fn gateway_require_pairing_default_remains_enabled() {
        let gateway = crate::config::GatewayConfig::default();
        assert!(gateway.require_pairing);
    }

    #[test]
    fn project_context_default_is_empty() {
        let ctx = ProjectContext::default();
        assert!(ctx.user_name.is_empty());
        assert!(ctx.timezone.is_empty());
        assert!(ctx.agent_name.is_empty());
        assert!(ctx.communication_style.is_empty());
    }

    // ── scaffold_workspace: basic file creation ─────────────────

    #[test]
    fn scaffold_creates_all_md_files() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let expected = [
            "IDENTITY.md",
            "AGENTS.md",
            "HEARTBEAT.md",
            "SOUL.md",
            "USER.md",
            "TOOLS.md",
            "BOOTSTRAP.md",
            "MEMORY.md",
        ];
        for f in &expected {
            assert!(tmp.path().join(f).exists(), "missing file: {f}");
        }
    }

    #[test]
    fn scaffold_creates_all_subdirectories() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        for dir in &["sessions", "memory", "state", "cron", "skills"] {
            assert!(tmp.path().join(dir).is_dir(), "missing subdirectory: {dir}");
        }
    }

    // ── scaffold_workspace: personalization ─────────────────────

    #[test]
    fn scaffold_bakes_user_name_into_files() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            user_name: "Alice".into(),
            ..Default::default()
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(
            user_md.contains("**Name:** Alice"),
            "USER.md should contain user name"
        );

        let bootstrap = fs::read_to_string(tmp.path().join("BOOTSTRAP.md")).unwrap();
        assert!(
            bootstrap.contains("**Alice**"),
            "BOOTSTRAP.md should contain user name"
        );
    }

    #[test]
    fn scaffold_bakes_timezone_into_files() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            timezone: "US/Pacific".into(),
            ..Default::default()
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(
            user_md.contains("**Timezone:** US/Pacific"),
            "USER.md should contain timezone"
        );

        let bootstrap = fs::read_to_string(tmp.path().join("BOOTSTRAP.md")).unwrap();
        assert!(
            bootstrap.contains("US/Pacific"),
            "BOOTSTRAP.md should contain timezone"
        );
    }

    #[test]
    fn scaffold_bakes_agent_name_into_files() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            agent_name: "Crabby".into(),
            ..Default::default()
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let identity = fs::read_to_string(tmp.path().join("IDENTITY.md")).unwrap();
        assert!(
            identity.contains("**Name:** Crabby"),
            "IDENTITY.md should contain agent name"
        );

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(
            soul.contains("You are **Crabby**"),
            "SOUL.md should contain agent name"
        );

        let agents = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        assert!(
            agents.contains("Crabby Personal Assistant"),
            "AGENTS.md should contain agent name"
        );

        let heartbeat = fs::read_to_string(tmp.path().join("HEARTBEAT.md")).unwrap();
        assert!(
            heartbeat.contains("Crabby"),
            "HEARTBEAT.md should contain agent name"
        );

        let bootstrap = fs::read_to_string(tmp.path().join("BOOTSTRAP.md")).unwrap();
        assert!(
            bootstrap.contains("Introduce yourself as Crabby"),
            "BOOTSTRAP.md should contain agent name"
        );
    }

    #[test]
    fn scaffold_bakes_communication_style() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            communication_style: "Be technical and detailed.".into(),
            ..Default::default()
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(
            soul.contains("Be technical and detailed."),
            "SOUL.md should contain communication style"
        );

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(
            user_md.contains("Be technical and detailed."),
            "USER.md should contain communication style"
        );

        let bootstrap = fs::read_to_string(tmp.path().join("BOOTSTRAP.md")).unwrap();
        assert!(
            bootstrap.contains("Be technical and detailed."),
            "BOOTSTRAP.md should contain communication style"
        );
    }

    // ── scaffold_workspace: defaults when context is empty ──────

    #[test]
    fn scaffold_uses_defaults_for_empty_context() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default(); // all empty
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let identity = fs::read_to_string(tmp.path().join("IDENTITY.md")).unwrap();
        assert!(
            identity.contains("**Name:** Corvus"),
            "should default agent name to Corvus"
        );

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(
            user_md.contains("**Name:** User"),
            "should default user name to User"
        );
        assert!(
            user_md.contains("**Timezone:** UTC"),
            "should default timezone to UTC"
        );

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(
            soul.contains("Be warm, natural, and clear."),
            "should default communication style"
        );
    }

    // ── scaffold_workspace: skip existing files ─────────────────

    #[test]
    fn scaffold_does_not_overwrite_existing_files() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            user_name: "Bob".into(),
            ..Default::default()
        };

        // Pre-create SOUL.md with custom content
        let soul_path = tmp.path().join("SOUL.md");
        fs::write(&soul_path, "# My Custom Soul\nDo not overwrite me.").unwrap();

        scaffold_workspace(tmp.path(), &ctx).unwrap();

        // SOUL.md should be untouched
        let soul = fs::read_to_string(&soul_path).unwrap();
        assert!(
            soul.contains("Do not overwrite me"),
            "existing files should not be overwritten"
        );
        assert!(
            !soul.contains("You're not a chatbot"),
            "should not contain scaffold content"
        );

        // But USER.md should be created fresh
        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(user_md.contains("**Name:** Bob"));
    }

    // ── scaffold_workspace: idempotent ──────────────────────────

    #[test]
    fn scaffold_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            user_name: "Eve".into(),
            agent_name: "Claw".into(),
            ..Default::default()
        };

        scaffold_workspace(tmp.path(), &ctx).unwrap();
        let soul_v1 = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();

        // Run again — should not change anything
        scaffold_workspace(tmp.path(), &ctx).unwrap();
        let soul_v2 = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();

        assert_eq!(soul_v1, soul_v2, "scaffold should be idempotent");
    }

    // ── scaffold_workspace: all files are non-empty ─────────────

    #[test]
    fn scaffold_files_are_non_empty() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        for f in &[
            "IDENTITY.md",
            "AGENTS.md",
            "HEARTBEAT.md",
            "SOUL.md",
            "USER.md",
            "TOOLS.md",
            "BOOTSTRAP.md",
            "MEMORY.md",
        ] {
            let content = fs::read_to_string(tmp.path().join(f)).unwrap();
            assert!(!content.trim().is_empty(), "{f} should not be empty");
        }
    }

    // ── scaffold_workspace: AGENTS.md references on-demand memory

    #[test]
    fn agents_md_references_on_demand_memory() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let agents = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        assert!(
            agents.contains("memory_recall"),
            "AGENTS.md should reference memory_recall for on-demand access"
        );
        assert!(
            agents.contains("on-demand"),
            "AGENTS.md should mention daily notes are on-demand"
        );
    }

    // ── scaffold_workspace: MEMORY.md warns about token cost ────

    #[test]
    fn memory_md_warns_about_token_cost() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let memory = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();
        assert!(
            memory.contains("costs tokens"),
            "MEMORY.md should warn about token cost"
        );
        assert!(
            memory.contains("auto-injected"),
            "MEMORY.md should mention it's auto-injected"
        );
    }

    // ── scaffold_workspace: TOOLS.md lists memory_forget ────────

    #[test]
    fn tools_md_lists_all_builtin_tools() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let tools = fs::read_to_string(tmp.path().join("TOOLS.md")).unwrap();
        for tool in &[
            "shell",
            "file_read",
            "file_write",
            "memory_store",
            "memory_recall",
            "memory_forget",
        ] {
            assert!(
                tools.contains(tool),
                "TOOLS.md should list built-in tool: {tool}"
            );
        }
        assert!(
            tools.contains("Use when:"),
            "TOOLS.md should include 'Use when' guidance"
        );
        assert!(
            tools.contains("Don't use when:"),
            "TOOLS.md should include 'Don't use when' guidance"
        );
    }

    #[test]
    fn soul_md_includes_emoji_awareness_guidance() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext::default();
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(
            soul.contains("Use emojis naturally (0-2 max"),
            "SOUL.md should include emoji usage guidance"
        );
        assert!(
            soul.contains("Match emoji density to the user"),
            "SOUL.md should include emoji-awareness guidance"
        );
    }

    // ── scaffold_workspace: special characters in names ─────────

    #[test]
    fn scaffold_handles_special_characters_in_names() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            user_name: "José María".into(),
            agent_name: "Corvus-v2".into(),
            timezone: "Europe/Madrid".into(),
            communication_style: "Be direct.".into(),
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(user_md.contains("José María"));

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(soul.contains("Corvus-v2"));
    }

    // ── scaffold_workspace: full personalization round-trip ─────

    #[test]
    fn scaffold_full_personalization() {
        let tmp = TempDir::new().unwrap();
        let ctx = ProjectContext {
            user_name: "Argenis".into(),
            timezone: "US/Eastern".into(),
            agent_name: "Claw".into(),
            communication_style:
                "Be friendly, human, and conversational. Show warmth and empathy while staying efficient. Use natural contractions."
                    .into(),
        };
        scaffold_workspace(tmp.path(), &ctx).unwrap();

        // Verify every file got personalized
        let identity = fs::read_to_string(tmp.path().join("IDENTITY.md")).unwrap();
        assert!(identity.contains("**Name:** Claw"));

        let soul = fs::read_to_string(tmp.path().join("SOUL.md")).unwrap();
        assert!(soul.contains("You are **Claw**"));
        assert!(soul.contains("Be friendly, human, and conversational"));

        let user_md = fs::read_to_string(tmp.path().join("USER.md")).unwrap();
        assert!(user_md.contains("**Name:** Argenis"));
        assert!(user_md.contains("**Timezone:** US/Eastern"));
        assert!(user_md.contains("Be friendly, human, and conversational"));

        let agents = fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("Claw Personal Assistant"));

        let bootstrap = fs::read_to_string(tmp.path().join("BOOTSTRAP.md")).unwrap();
        assert!(bootstrap.contains("**Argenis**"));
        assert!(bootstrap.contains("US/Eastern"));
        assert!(bootstrap.contains("Introduce yourself as Claw"));

        let heartbeat = fs::read_to_string(tmp.path().join("HEARTBEAT.md")).unwrap();
        assert!(heartbeat.contains("Claw"));
    }

    // ── model helper coverage ───────────────────────────────────

    #[test]
    fn default_model_for_provider_uses_latest_defaults() {
        assert_eq!(default_model_for_provider("openai"), "gpt-5.2");
        assert_eq!(
            default_model_for_provider("anthropic"),
            "claude-sonnet-4-5-20250929"
        );
        assert_eq!(default_model_for_provider("qwen"), "qwen-plus");
        assert_eq!(default_model_for_provider("qwen-intl"), "qwen-plus");
        assert_eq!(default_model_for_provider("glm-cn"), "glm-5");
        assert_eq!(default_model_for_provider("minimax-cn"), "MiniMax-M2.5");
        assert_eq!(default_model_for_provider("zai-cn"), "glm-5");
        assert_eq!(default_model_for_provider("gemini"), "gemini-2.5-pro");
        assert_eq!(default_model_for_provider("google"), "gemini-2.5-pro");
        assert_eq!(
            default_model_for_provider("google-gemini"),
            "gemini-2.5-pro"
        );
        assert_eq!(default_model_for_provider("copilot"), "gpt-4o");
        assert_eq!(default_model_for_provider("github-copilot"), "gpt-4o");
    }

    #[test]
    fn canonical_provider_name_normalizes_regional_aliases() {
        assert_eq!(canonical_provider_name("qwen-intl"), "qwen");
        assert_eq!(canonical_provider_name("dashscope-us"), "qwen");
        assert_eq!(canonical_provider_name("moonshot-intl"), "moonshot");
        assert_eq!(canonical_provider_name("kimi-cn"), "moonshot");
        assert_eq!(canonical_provider_name("glm-cn"), "glm");
        assert_eq!(canonical_provider_name("bigmodel"), "glm");
        assert_eq!(canonical_provider_name("minimax-cn"), "minimax");
        assert_eq!(canonical_provider_name("zai-cn"), "zai");
        assert_eq!(canonical_provider_name("z.ai-global"), "zai");
        assert_eq!(canonical_provider_name("github-copilot"), "copilot");
    }

    #[test]
    fn curated_models_for_openai_include_latest_choices() {
        let ids: Vec<String> = curated_models_for_provider("openai")
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        assert!(ids.contains(&"gpt-5.2".to_string()));
        assert!(ids.contains(&"gpt-5-mini".to_string()));
    }

    #[test]
    fn curated_models_for_openrouter_use_valid_anthropic_id() {
        let ids: Vec<String> = curated_models_for_provider("openrouter")
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        assert!(ids.contains(&"anthropic/claude-sonnet-4.5".to_string()));
    }

    #[test]
    fn supports_live_model_fetch_for_supported_and_unsupported_providers() {
        assert!(supports_live_model_fetch("openai"));
        assert!(supports_live_model_fetch("anthropic"));
        assert!(supports_live_model_fetch("gemini"));
        assert!(supports_live_model_fetch("google"));
        assert!(supports_live_model_fetch("grok"));
        assert!(supports_live_model_fetch("together"));
        assert!(supports_live_model_fetch("ollama"));
        assert!(supports_live_model_fetch("copilot"));
        assert!(supports_live_model_fetch("github-copilot"));
        assert!(supports_live_model_fetch("astrai"));
        assert!(!supports_live_model_fetch("venice"));
    }

    #[test]
    fn fetch_copilot_models_requires_non_empty_token() {
        let missing = fetch_copilot_models(None).unwrap_err().to_string();
        assert!(missing.contains("GitHub token is required"));

        let empty = fetch_copilot_models(Some("   ")).unwrap_err().to_string();
        assert!(empty.contains("GitHub token is required"));
    }

    #[test]
    fn curated_models_provider_aliases_share_same_catalog() {
        assert_eq!(
            curated_models_for_provider("xai"),
            curated_models_for_provider("grok")
        );
        assert_eq!(
            curated_models_for_provider("together-ai"),
            curated_models_for_provider("together")
        );
        assert_eq!(
            curated_models_for_provider("gemini"),
            curated_models_for_provider("google")
        );
        assert_eq!(
            curated_models_for_provider("gemini"),
            curated_models_for_provider("google-gemini")
        );
        assert_eq!(
            curated_models_for_provider("qwen"),
            curated_models_for_provider("qwen-intl")
        );
        assert_eq!(
            curated_models_for_provider("qwen"),
            curated_models_for_provider("dashscope-us")
        );
        assert_eq!(
            curated_models_for_provider("minimax"),
            curated_models_for_provider("minimax-cn")
        );
        assert_eq!(
            curated_models_for_provider("zai"),
            curated_models_for_provider("zai-cn")
        );
        assert_eq!(
            curated_models_for_provider("copilot"),
            curated_models_for_provider("github-copilot")
        );
    }

    #[test]
    fn parse_openai_model_ids_supports_data_array_payload() {
        let payload = json!({
            "data": [
                {"id": "  gpt-5.1  "},
                {"id": "gpt-5-mini"},
                {"id": "gpt-5.1"},
                {"id": ""}
            ]
        });

        let ids = parse_openai_compatible_model_ids(&payload);
        assert_eq!(ids, vec!["gpt-5-mini".to_string(), "gpt-5.1".to_string()]);
    }

    #[test]
    fn parse_openai_model_ids_supports_root_array_payload() {
        let payload = json!([
            {"id": "alpha"},
            {"id": "beta"},
            {"id": "alpha"}
        ]);

        let ids = parse_openai_compatible_model_ids(&payload);
        assert_eq!(ids, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn parse_gemini_model_ids_filters_for_generate_content() {
        let payload = json!({
            "models": [
                {
                    "name": "models/gemini-2.5-pro",
                    "supportedGenerationMethods": ["generateContent", "countTokens"]
                },
                {
                    "name": "models/text-embedding-004",
                    "supportedGenerationMethods": ["embedContent"]
                },
                {
                    "name": "models/gemini-2.5-flash",
                    "supportedGenerationMethods": ["generateContent"]
                }
            ]
        });

        let ids = parse_gemini_model_ids(&payload);
        assert_eq!(
            ids,
            vec!["gemini-2.5-flash".to_string(), "gemini-2.5-pro".to_string()]
        );
    }

    #[test]
    fn parse_ollama_model_ids_extracts_and_deduplicates_names() {
        let payload = json!({
            "models": [
                {"name": "llama3.2:latest"},
                {"name": "mistral:latest"},
                {"name": "llama3.2:latest"}
            ]
        });

        let ids = parse_ollama_model_ids(&payload);
        assert_eq!(
            ids,
            vec!["llama3.2:latest".to_string(), "mistral:latest".to_string()]
        );
    }

    #[test]
    fn model_cache_round_trip_returns_fresh_entry() {
        let tmp = TempDir::new().unwrap();
        let models = vec!["gpt-5.1".to_string(), "gpt-5-mini".to_string()];

        cache_live_models_for_provider(tmp.path(), "openai", &models).unwrap();

        let cached =
            load_cached_models_for_provider(tmp.path(), "openai", MODEL_CACHE_TTL_SECS).unwrap();
        let cached = cached.expect("expected fresh cached models");

        assert_eq!(cached.models.len(), 2);
        assert!(cached.models.contains(&"gpt-5.1".to_string()));
        assert!(cached.models.contains(&"gpt-5-mini".to_string()));
    }

    #[test]
    fn model_cache_ttl_filters_stale_entries() {
        let tmp = TempDir::new().unwrap();
        let stale = ModelCacheState {
            entries: vec![ModelCacheEntry {
                provider: "openai".to_string(),
                fetched_at_unix: now_unix_secs().saturating_sub(MODEL_CACHE_TTL_SECS + 120),
                models: vec!["gpt-5.1".to_string()],
            }],
        };

        save_model_cache_state(tmp.path(), &stale).unwrap();

        let fresh =
            load_cached_models_for_provider(tmp.path(), "openai", MODEL_CACHE_TTL_SECS).unwrap();
        assert!(fresh.is_none());

        let stale_any = load_any_cached_models_for_provider(tmp.path(), "openai").unwrap();
        assert!(stale_any.is_some());
    }

    #[test]
    fn run_models_refresh_uses_fresh_cache_without_network() {
        let tmp = TempDir::new().unwrap();

        cache_live_models_for_provider(tmp.path(), "openai", &["gpt-5.1".to_string()]).unwrap();

        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            default_provider: Some("openai".to_string()),
            ..Config::default()
        };

        run_models_refresh(&config, None, false).unwrap();
    }

    #[test]
    fn run_models_refresh_rejects_unsupported_provider() {
        let tmp = TempDir::new().unwrap();

        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            default_provider: Some("venice".to_string()),
            ..Config::default()
        };

        let err = run_models_refresh(&config, None, true).unwrap_err();
        assert!(err
            .to_string()
            .contains("does not support live model discovery"));
    }

    // ── provider_env_var ────────────────────────────────────────

    #[test]
    fn provider_env_var_known_providers() {
        assert_eq!(provider_env_var("openrouter"), "OPENROUTER_API_KEY");
        assert_eq!(provider_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(provider_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(provider_env_var("ollama"), "OLLAMA_API_KEY");
        assert_eq!(provider_env_var("xai"), "XAI_API_KEY");
        assert_eq!(provider_env_var("grok"), "XAI_API_KEY"); // alias
        assert_eq!(provider_env_var("together"), "TOGETHER_API_KEY"); // alias
        assert_eq!(provider_env_var("together-ai"), "TOGETHER_API_KEY");
        assert_eq!(provider_env_var("google"), "GEMINI_API_KEY"); // alias
        assert_eq!(provider_env_var("google-gemini"), "GEMINI_API_KEY"); // alias
        assert_eq!(provider_env_var("gemini"), "GEMINI_API_KEY");
        assert_eq!(provider_env_var("copilot"), "GITHUB_TOKEN");
        assert_eq!(provider_env_var("github-copilot"), "GITHUB_TOKEN"); // alias
        assert_eq!(provider_env_var("qwen"), "DASHSCOPE_API_KEY");
        assert_eq!(provider_env_var("qwen-intl"), "DASHSCOPE_API_KEY");
        assert_eq!(provider_env_var("dashscope-us"), "DASHSCOPE_API_KEY");
        assert_eq!(provider_env_var("glm-cn"), "GLM_API_KEY");
        assert_eq!(provider_env_var("minimax-cn"), "MINIMAX_API_KEY");
        assert_eq!(provider_env_var("moonshot-intl"), "MOONSHOT_API_KEY");
        assert_eq!(provider_env_var("zai-cn"), "ZAI_API_KEY");
        assert_eq!(provider_env_var("nvidia"), "NVIDIA_API_KEY");
        assert_eq!(provider_env_var("nvidia-nim"), "NVIDIA_API_KEY"); // alias
        assert_eq!(provider_env_var("build.nvidia.com"), "NVIDIA_API_KEY"); // alias
        assert_eq!(provider_env_var("astrai"), "ASTRAI_API_KEY");
    }

    #[test]
    fn provider_env_var_unknown_falls_back() {
        assert_eq!(provider_env_var("some-new-provider"), "API_KEY");
    }

    #[test]
    fn resolve_live_model_api_key_uses_gh_token_for_copilot() {
        let _guard = env_lock().lock().unwrap_or_else(|error| error.into_inner());
        let _restore_github_token = EnvRestore::capture("GITHUB_TOKEN");
        let _restore_gh_token = EnvRestore::capture("GH_TOKEN");

        std::env::remove_var("GITHUB_TOKEN");
        std::env::set_var("GH_TOKEN", "  ghp-from-gh-cli  ");

        let resolved = resolve_live_model_api_key("copilot", "");
        assert_eq!(resolved.as_deref(), Some("ghp-from-gh-cli"));
    }

    #[test]
    fn backend_key_from_choice_maps_supported_backends() {
        assert_eq!(backend_key_from_choice(0), "sqlite");
        assert_eq!(backend_key_from_choice(1), "lucid");
        assert_eq!(backend_key_from_choice(2), "surreal-graphs");
        assert_eq!(backend_key_from_choice(3), "markdown");
        assert_eq!(backend_key_from_choice(4), "none");
        assert_eq!(backend_key_from_choice(999), "sqlite");
    }

    #[test]
    fn memory_backend_profile_marks_lucid_as_optional_sqlite_backed() {
        let lucid = memory_backend_profile("lucid");
        assert!(lucid.auto_save_default);
        assert!(lucid.uses_sqlite_hygiene);
        assert!(lucid.sqlite_based);
        assert!(lucid.optional_dependency);

        let markdown = memory_backend_profile("markdown");
        assert!(markdown.auto_save_default);
        assert!(!markdown.uses_sqlite_hygiene);

        let none = memory_backend_profile("none");
        assert!(!none.auto_save_default);
        assert!(!none.uses_sqlite_hygiene);

        let custom = memory_backend_profile("custom-memory");
        assert!(custom.auto_save_default);
        assert!(!custom.uses_sqlite_hygiene);
    }

    #[test]
    fn memory_config_defaults_for_lucid_enable_sqlite_hygiene() {
        let config = memory_config_defaults_for_backend("lucid");
        assert_eq!(config.backend, "lucid");
        assert!(config.auto_save);
        assert!(config.hygiene_enabled);
        assert_eq!(config.archive_after_days, 7);
        assert_eq!(config.purge_after_days, 30);
        assert_eq!(config.embedding_cache_size, 10000);
    }

    #[test]
    fn memory_config_defaults_for_none_disable_sqlite_hygiene() {
        let config = memory_config_defaults_for_backend("none");
        assert_eq!(config.backend, "none");
        assert!(!config.auto_save);
        assert!(!config.hygiene_enabled);
        assert_eq!(config.archive_after_days, 0);
        assert_eq!(config.purge_after_days, 0);
        assert_eq!(config.embedding_cache_size, 0);
    }

    #[test]
    fn memory_config_defaults_include_surreal_defaults() {
        let config = memory_config_defaults_for_backend("sqlite");
        assert_eq!(config.surreal.namespace.as_deref(), Some("corvus"));
        assert_eq!(config.surreal.database.as_deref(), Some("memory"));
        assert!(config.surreal.allow_http_loopback);
    }

    #[test]
    fn trim_non_empty_returns_none_for_blank() {
        assert_eq!(trim_non_empty("   ".into()), None);
        assert_eq!(trim_non_empty("value".into()), Some("value".into()));
    }

    #[test]
    fn requires_surreal_memory_setup_accepts_surreal_variants() {
        assert!(requires_surreal_memory_setup("surreal"));
        assert!(requires_surreal_memory_setup("surreal-graphs"));
        assert!(!requires_surreal_memory_setup("sqlite"));
    }
}
