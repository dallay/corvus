use crate::providers::{is_glm_alias, is_zai_alias};
use crate::security::AutonomyLevel;
use anyhow::{Context, Result};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use url::Url;

// ── Top-level config ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Workspace directory - computed from home, not serialized
    #[serde(skip)]
    pub workspace_dir: PathBuf,
    /// Path to config.toml - computed from home, not serialized
    #[serde(skip)]
    pub config_path: PathBuf,
    pub api_key: Option<String>,
    /// Base URL override for provider API (e.g. "http://10.0.0.1:11434" for remote Ollama)
    pub api_url: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default)]
    pub autonomy: AutonomyConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    #[serde(default)]
    pub reliability: ReliabilityConfig,

    #[serde(default)]
    pub scheduler: SchedulerConfig,

    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub mission: MissionConfig,

    /// Model routing rules — route `hint:<name>` to specific provider+model combos.
    #[serde(default)]
    pub model_routes: Vec<ModelRouteConfig>,

    /// Automatic query classification — maps user messages to model hints.
    #[serde(default)]
    pub query_classification: QueryClassificationConfig,

    #[serde(default)]
    pub heartbeat: HeartbeatConfig,

    #[serde(default)]
    pub cron: CronConfig,

    #[serde(default)]
    pub channels_config: ChannelsConfig,

    #[serde(default)]
    pub updates: UpdateConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub tunnel: TunnelConfig,

    #[serde(default)]
    pub gateway: GatewayConfig,

    #[serde(default)]
    pub composio: ComposioConfig,

    #[serde(default)]
    pub secrets: SecretsConfig,

    #[serde(default)]
    pub browser: BrowserConfig,

    #[serde(default)]
    pub http_request: HttpRequestConfig,

    #[serde(default)]
    pub web_search: WebSearchConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    #[serde(default)]
    pub identity: IdentityConfig,

    #[serde(default)]
    pub cost: CostConfig,

    #[serde(default)]
    pub peripherals: PeripheralsConfig,

    /// Delegate agent configurations for multi-agent workflows.
    #[serde(default)]
    pub agents: HashMap<String, DelegateAgentConfig>,

    /// Hardware configuration (wizard-driven physical world setup).
    #[serde(default)]
    pub hardware: HardwareConfig,

    #[serde(default)]
    pub skills: SkillsConfig,

    #[serde(default)]
    pub multimodal: MultimodalConfig,

    #[serde(default)]
    pub audio: AudioConfig,
}

// ── Delegate Agents ──────────────────────────────────────────────

/// Configuration for a delegate sub-agent used by the `delegate` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateAgentConfig {
    /// Provider name (e.g. "ollama", "openrouter", "anthropic")
    pub provider: String,
    /// Model name
    pub model: String,
    /// Optional system prompt for the sub-agent
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional API key override
    #[serde(default)]
    pub api_key: Option<String>,
    /// Temperature override
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Max recursion depth for nested delegation
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// Execution mode for delegated sessions.
    #[serde(default)]
    pub execution_mode: DelegateExecutionMode,
    /// Max tool iterations override for delegated sessions (None = inherit from agent config).
    #[serde(default)]
    pub max_iterations: Option<usize>,
    /// Max wall-clock time in milliseconds for delegated sessions (None = no override).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

fn default_max_depth() -> u32 {
    3
}

// ── Code Session Config ──────────────────────────────────────────

/// How a delegate agent executes its session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DelegateExecutionMode {
    /// Single LLM call, no tool loop (one-shot).
    #[default]
    OneShot,
    /// Full agent loop with tool iteration (session).
    Session,
}

/// Configuration for a validation command run after code changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCommandConfig {
    /// Shell command to run (e.g. "cargo test").
    pub command: String,
    /// Whether this validation step is required to pass.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Maximum time in milliseconds to wait for the command.
    #[serde(default = "default_validation_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_validation_timeout_ms() -> u64 {
    60_000
}

impl Default for ValidationCommandConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            required: true,
            timeout_ms: default_validation_timeout_ms(),
        }
    }
}

/// Configuration for the code-session capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSessionConfig {
    /// Whether code-session mode is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Validation commands to run after changes (e.g. build, test, lint).
    #[serde(default)]
    pub validation_commands: Vec<ValidationCommandConfig>,
    /// Maximum tool iterations for a code session (overrides agent.max_tool_iterations).
    #[serde(default = "default_code_session_max_iterations")]
    pub max_iterations: usize,
    /// Maximum wall-clock time in milliseconds for a code session.
    #[serde(default = "default_code_session_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_code_session_max_iterations() -> usize {
    50
}

fn default_code_session_timeout_ms() -> u64 {
    600_000
}

impl Default for CodeSessionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            validation_commands: Vec::new(),
            max_iterations: default_code_session_max_iterations(),
            timeout_ms: default_code_session_timeout_ms(),
        }
    }
}

// ── Skills config ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Override the official skills catalog repository URL.
    #[serde(default)]
    pub catalog_repo_url: Option<String>,
    /// Cache TTL in hours for the catalog index (default: 24).
    #[serde(default)]
    pub catalog_cache_ttl_hours: Option<u64>,
    /// Enable content integrity verification on load (default: true).
    #[serde(default = "default_true")]
    pub verify_integrity: bool,
    /// Prompt injection scan threshold score (default: 50). None disables scanning.
    #[serde(default = "default_scan_threshold")]
    pub scan_threshold: Option<u32>,
}

fn default_scan_threshold() -> Option<u32> {
    Some(50)
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            catalog_repo_url: None,
            catalog_cache_ttl_hours: None,
            verify_integrity: true,
            scan_threshold: default_scan_threshold(),
        }
    }
}

// ── Multimodal rollout controls ─────────────────────────────────

/// Valid MVP channel names for multimodal image ingress.
const MVP_VALID_MULTIMODAL_CHANNELS: &[&str] = &["telegram", "whatsapp", "discord"];

/// Multimodal image ingress rollout controls.
///
/// Default-deny: `enabled = false` means no channel emits
/// image parts in production.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultimodalConfig {
    /// Global kill switch for image ingress (default: false).
    #[serde(default)]
    pub enabled: bool,
    /// Channel allowlist; MVP-valid: "telegram", "whatsapp", "discord".
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// Existing route hint used only for image turns.
    #[serde(default)]
    pub vision_model_hint: Option<String>,
    /// Operator override for the default 10 MiB limit.
    #[serde(default)]
    pub max_image_bytes: Option<u64>,
}

// ── Audio input rollout controls ────────────────────────────────

/// Valid channel names for audio ingress.
const VALID_AUDIO_CHANNELS: &[&str] = &["telegram", "gateway", "cli"];

// Hard ceilings imported from the canonical definition in audio_media.
use crate::channels::audio_media::{MAX_AUDIO_BYTES_CEILING, MAX_AUDIO_DURATION_SECS_CEILING};

/// Audio input processing and transcription controls.
///
/// Default-deny: `enabled = false` means no channel processes audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioConfig {
    /// Global kill switch for audio ingress (default: false).
    #[serde(default)]
    pub enabled: bool,
    /// Channel allowlist for audio ingress.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// Maximum audio file size in bytes (default: 25 MiB).
    #[serde(default = "default_max_audio_bytes")]
    pub max_audio_bytes: u64,
    /// Maximum audio duration in seconds (default: 600 = 10 min).
    #[serde(default = "default_max_audio_duration_secs")]
    pub max_audio_duration_secs: u64,
    /// Whisper model name (default: "base").
    #[serde(default = "default_transcription_model")]
    pub transcription_model: String,
    /// Language hint for transcription (default: "es").
    #[serde(default = "default_transcription_language")]
    pub transcription_language: String,
    /// Path to whisper.cpp binary (default: "whisper-cli").
    #[serde(default = "default_whisper_binary")]
    pub whisper_binary: String,
    /// Max concurrent transcriptions (default: 1).
    #[serde(default = "default_max_concurrent_transcriptions")]
    pub max_concurrent_transcriptions: usize,
    /// Per-transcription timeout in seconds (default: 120).
    #[serde(default = "default_transcription_timeout_secs")]
    pub transcription_timeout_secs: u64,
}

fn default_max_audio_bytes() -> u64 {
    26_214_400 // 25 MiB
}

fn default_max_audio_duration_secs() -> u64 {
    600 // 10 minutes
}

fn default_transcription_model() -> String {
    "base".into()
}

fn default_transcription_language() -> String {
    "es".into()
}

fn default_whisper_binary() -> String {
    "whisper-cli".into()
}

fn default_max_concurrent_transcriptions() -> usize {
    1
}

fn default_transcription_timeout_secs() -> u64 {
    120
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_channels: Vec::new(),
            max_audio_bytes: default_max_audio_bytes(),
            max_audio_duration_secs: default_max_audio_duration_secs(),
            transcription_model: default_transcription_model(),
            transcription_language: default_transcription_language(),
            whisper_binary: default_whisper_binary(),
            max_concurrent_transcriptions: default_max_concurrent_transcriptions(),
            transcription_timeout_secs: default_transcription_timeout_secs(),
        }
    }
}

// ── Hardware Config (wizard-driven) ─────────────────────────────

/// Hardware transport mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HardwareTransport {
    #[default]
    None,
    Native,
    Serial,
    Probe,
}

impl std::fmt::Display for HardwareTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Native => write!(f, "native"),
            Self::Serial => write!(f, "serial"),
            Self::Probe => write!(f, "probe"),
        }
    }
}

/// Wizard-driven hardware configuration for physical world interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// Whether hardware access is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Transport mode
    #[serde(default)]
    pub transport: HardwareTransport,
    /// Serial port path (e.g. "/dev/ttyACM0")
    #[serde(default)]
    pub serial_port: Option<String>,
    /// Serial baud rate
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    /// Probe target chip (e.g. "STM32F401RE")
    #[serde(default)]
    pub probe_target: Option<String>,
    /// Enable workspace datasheet RAG (index PDF schematics for AI pin lookups)
    #[serde(default)]
    pub workspace_datasheets: bool,
}

fn default_baud_rate() -> u32 {
    115_200
}

impl HardwareConfig {
    /// Return the active transport mode.
    pub fn transport_mode(&self) -> HardwareTransport {
        self.transport.clone()
    }
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: HardwareTransport::None,
            serial_port: None,
            baud_rate: default_baud_rate(),
            probe_target: None,
            workspace_datasheets: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// When true: compact bootstrap context and rag_chunk_limit=2. Use for 13B or smaller models.
    #[serde(default)]
    pub compact_context: bool,
    /// Capability profile used to compose tools and memory behavior.
    /// Supported values: "full" (default), "code", "lite".
    #[serde(default = "default_agent_profile")]
    pub profile: String,
    #[serde(default = "default_agent_max_tool_iterations")]
    pub max_tool_iterations: usize,
    #[serde(default = "default_agent_max_history_messages")]
    pub max_history_messages: usize,
    #[serde(default)]
    pub parallel_tools: bool,
    #[serde(default = "default_agent_tool_dispatcher")]
    pub tool_dispatcher: String,
    /// Code-session specific configuration.
    #[serde(default)]
    pub code_session: CodeSessionConfig,
}

fn default_agent_max_tool_iterations() -> usize {
    10
}

fn default_agent_profile() -> String {
    "full".into()
}

fn is_supported_agent_profile(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "" | "full" | "code" | "lite"
    )
}

fn default_agent_max_history_messages() -> usize {
    50
}

fn default_agent_tool_dispatcher() -> String {
    "auto".into()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            compact_context: false,
            profile: default_agent_profile(),
            max_tool_iterations: default_agent_max_tool_iterations(),
            max_history_messages: default_agent_max_history_messages(),
            parallel_tools: false,
            tool_dispatcher: default_agent_tool_dispatcher(),
            code_session: CodeSessionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mission_max_runtime_ms")]
    pub max_runtime_ms: u64,
    #[serde(default = "default_mission_max_steps")]
    pub max_steps: u32,
    #[serde(default = "default_mission_max_estimated_cost_cents")]
    pub max_estimated_cost_cents: u32,
}

fn default_mission_max_runtime_ms() -> u64 {
    300_000
}

fn default_mission_max_steps() -> u32 {
    10
}

fn default_mission_max_estimated_cost_cents() -> u32 {
    100
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_runtime_ms: default_mission_max_runtime_ms(),
            max_steps: default_mission_max_steps(),
            max_estimated_cost_cents: default_mission_max_estimated_cost_cents(),
        }
    }
}

// ── Identity (AIEOS / OpenClaw format) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Identity format: "openclaw" (default) or "aieos"
    #[serde(default = "default_identity_format")]
    pub format: String,
    /// Path to AIEOS JSON file (relative to workspace)
    #[serde(default)]
    pub aieos_path: Option<String>,
    /// Inline AIEOS JSON (alternative to file path)
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

fn default_identity_format() -> String {
    "openclaw".into()
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            format: default_identity_format(),
            aieos_path: None,
            aieos_inline: None,
        }
    }
}

// ── Cost tracking and budget enforcement ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Enable cost tracking (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Session spending limit in USD (default: 0.00, disabled)
    #[serde(default = "default_session_limit")]
    pub session_limit_usd: f64,

    /// Daily spending limit in USD (default: 10.00)
    #[serde(default = "default_daily_limit")]
    pub daily_limit_usd: f64,

    /// Monthly spending limit in USD (default: 100.00)
    #[serde(default = "default_monthly_limit")]
    pub monthly_limit_usd: f64,

    /// Warn when spending reaches this percentage of limit (default: 80)
    #[serde(default = "default_warn_percent")]
    pub warn_at_percent: u8,

    /// Allow requests to exceed budget with --override flag (default: false)
    #[serde(default)]
    pub allow_override: bool,

    /// Per-model pricing (USD per 1M tokens)
    #[serde(default)]
    pub prices: std::collections::HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Input price per 1M tokens
    #[serde(default)]
    pub input: f64,

    /// Output price per 1M tokens
    #[serde(default)]
    pub output: f64,
}

fn default_daily_limit() -> f64 {
    10.0
}

fn default_session_limit() -> f64 {
    0.0
}

fn default_monthly_limit() -> f64 {
    100.0
}

fn default_warn_percent() -> u8 {
    80
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            session_limit_usd: default_session_limit(),
            daily_limit_usd: default_daily_limit(),
            monthly_limit_usd: default_monthly_limit(),
            warn_at_percent: default_warn_percent(),
            allow_override: false,
            prices: get_default_pricing(),
        }
    }
}

/// Default pricing for popular models (USD per 1M tokens)
fn get_default_pricing() -> std::collections::HashMap<String, ModelPricing> {
    let mut prices = std::collections::HashMap::new();

    // Anthropic models
    prices.insert(
        "anthropic/claude-sonnet-4-20250514".into(),
        ModelPricing {
            input: 3.0,
            output: 15.0,
        },
    );
    prices.insert(
        "anthropic/claude-opus-4-20250514".into(),
        ModelPricing {
            input: 15.0,
            output: 75.0,
        },
    );
    prices.insert(
        "anthropic/claude-3.5-sonnet".into(),
        ModelPricing {
            input: 3.0,
            output: 15.0,
        },
    );
    prices.insert(
        "anthropic/claude-3-haiku".into(),
        ModelPricing {
            input: 0.25,
            output: 1.25,
        },
    );

    // OpenAI models
    prices.insert(
        "openai/gpt-4o".into(),
        ModelPricing {
            input: 5.0,
            output: 15.0,
        },
    );
    prices.insert(
        "openai/gpt-4o-mini".into(),
        ModelPricing {
            input: 0.15,
            output: 0.60,
        },
    );
    prices.insert(
        "openai/o1-preview".into(),
        ModelPricing {
            input: 15.0,
            output: 60.0,
        },
    );

    // Google models
    prices.insert(
        "google/gemini-2.0-flash".into(),
        ModelPricing {
            input: 0.10,
            output: 0.40,
        },
    );
    prices.insert(
        "google/gemini-1.5-pro".into(),
        ModelPricing {
            input: 1.25,
            output: 5.0,
        },
    );

    prices
}

// ── Peripherals (hardware: STM32, RPi GPIO, etc.) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeripheralsConfig {
    /// Enable peripheral support (boards become agent tools)
    #[serde(default)]
    pub enabled: bool,
    /// Board configurations (nucleo-f401re, rpi-gpio, etc.)
    #[serde(default)]
    pub boards: Vec<PeripheralBoardConfig>,
    /// Path to datasheet docs (relative to workspace) for RAG retrieval.
    /// Place .md/.txt files named by board (e.g. nucleo-f401re.md, rpi-gpio.md).
    #[serde(default)]
    pub datasheet_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralBoardConfig {
    /// Board type: "nucleo-f401re", "rpi-gpio", "esp32", etc.
    pub board: String,
    /// Transport: "serial", "native", "bridge"
    #[serde(default = "default_peripheral_transport")]
    pub transport: String,
    /// Path for serial: "/dev/ttyACM0", "/dev/ttyUSB0"
    #[serde(default)]
    pub path: Option<String>,
    /// Baud rate for serial (default: 115200)
    #[serde(default = "default_peripheral_baud")]
    pub baud: u32,
}

fn default_peripheral_transport() -> String {
    "serial".into()
}

fn default_peripheral_baud() -> u32 {
    115_200
}

impl Default for PeripheralBoardConfig {
    fn default() -> Self {
        Self {
            board: String::new(),
            transport: default_peripheral_transport(),
            path: None,
            baud: default_peripheral_baud(),
        }
    }
}

// ── Gateway security ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
// Gateway config aggregates multiple security toggles; refactor is out of scope here.
#[allow(clippy::struct_excessive_bools)]
pub struct GatewayConfig {
    /// Gateway port (default: 3000)
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    /// Gateway host (default: 127.0.0.1)
    #[serde(default = "default_gateway_host")]
    pub host: String,
    /// Allow admin HTTP API to read/patch provider account pools (default: false).
    #[serde(default)]
    pub admin_expose_provider_pools: bool,
    /// Require pairing before accepting requests (default: true)
    #[serde(default = "default_true")]
    pub require_pairing: bool,
    /// Allow binding to non-localhost without a tunnel (default: false)
    #[serde(default)]
    pub allow_public_bind: bool,
    /// Allow `/session/list` token scoping without paired-token validation when pairing is disabled.
    /// Disabled by default to preserve deny-by-default session access.
    #[serde(default)]
    pub allow_unpaired_session_scopes: bool,
    /// Paired bearer tokens (managed automatically, not user-edited)
    #[serde(default)]
    pub paired_tokens: Vec<String>,

    /// Max `/pair` requests per minute per client key.
    #[serde(default = "default_pair_rate_limit")]
    pub pair_rate_limit_per_minute: u32,

    /// Max `/webhook` requests per minute per client key.
    #[serde(default = "default_webhook_rate_limit")]
    pub webhook_rate_limit_per_minute: u32,

    /// Trust proxy-forwarded client IP headers (`X-Forwarded-For`, `X-Real-IP`).
    /// Disabled by default; enable only behind a trusted reverse proxy.
    #[serde(default)]
    pub trust_forwarded_headers: bool,

    /// Maximum distinct client keys tracked by gateway rate limiter maps.
    #[serde(default = "default_gateway_rate_limit_max_keys")]
    pub rate_limit_max_keys: usize,

    /// TTL for webhook idempotency keys.
    #[serde(default = "default_idempotency_ttl_secs")]
    pub idempotency_ttl_secs: u64,

    /// Maximum distinct idempotency keys retained in memory.
    #[serde(default = "default_gateway_idempotency_max_keys")]
    pub idempotency_max_keys: usize,

    /// Route `/webhook` through the canonical dispatcher-backed runtime.
    ///
    /// Keep this disabled during rollout to preserve the legacy `simple_chat()` path. When enabled,
    /// operators should compare dispatcher versus legacy telemetry, and can roll back immediately by
    /// disabling the flag again. This switch does not affect `/whatsapp`, which remains on its
    /// separate deferred path for this change.
    #[serde(default)]
    pub webhook_dispatcher_enabled: bool,
}

fn default_gateway_port() -> u16 {
    3000
}

fn default_gateway_host() -> String {
    "127.0.0.1".into()
}

fn default_pair_rate_limit() -> u32 {
    10
}

fn default_webhook_rate_limit() -> u32 {
    60
}

fn default_idempotency_ttl_secs() -> u64 {
    300
}

fn default_gateway_rate_limit_max_keys() -> usize {
    10_000
}

fn default_gateway_idempotency_max_keys() -> usize {
    10_000
}

fn default_true() -> bool {
    true
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_gateway_port(),
            host: default_gateway_host(),
            admin_expose_provider_pools: false,
            require_pairing: true,
            allow_public_bind: false,
            allow_unpaired_session_scopes: false,
            paired_tokens: Vec::new(),
            pair_rate_limit_per_minute: default_pair_rate_limit(),
            webhook_rate_limit_per_minute: default_webhook_rate_limit(),
            trust_forwarded_headers: false,
            rate_limit_max_keys: default_gateway_rate_limit_max_keys(),
            idempotency_ttl_secs: default_idempotency_ttl_secs(),
            idempotency_max_keys: default_gateway_idempotency_max_keys(),
            webhook_dispatcher_enabled: false,
        }
    }
}

// ── Composio (managed tool surface) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposioConfig {
    /// Enable Composio integration for 1000+ OAuth tools
    #[serde(default)]
    pub enabled: bool,
    /// Composio API key (stored encrypted when secrets.encrypt = true)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Default entity ID for multi-user setups
    #[serde(default = "default_entity_id")]
    pub entity_id: String,
}

fn default_entity_id() -> String {
    "default".into()
}

impl Default for ComposioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            entity_id: default_entity_id(),
        }
    }
}

// ── Secrets (encrypted credential store) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Enable encryption for API keys and tokens in config.toml
    #[serde(default = "default_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

// ── Browser (friendly-service browsing only) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserComputerUseConfig {
    /// Sidecar endpoint for computer-use actions (OS-level mouse/keyboard/screenshot)
    #[serde(default = "default_browser_computer_use_endpoint")]
    pub endpoint: String,
    /// Optional bearer token for computer-use sidecar
    #[serde(default)]
    pub api_key: Option<String>,
    /// Per-action request timeout in milliseconds
    #[serde(default = "default_browser_computer_use_timeout_ms")]
    pub timeout_ms: u64,
    /// Allow remote/public endpoint for computer-use sidecar (default: false)
    #[serde(default)]
    pub allow_remote_endpoint: bool,
    /// Optional window title/process allowlist forwarded to sidecar policy
    #[serde(default)]
    pub window_allowlist: Vec<String>,
    /// Optional X-axis boundary for coordinate-based actions
    #[serde(default)]
    pub max_coordinate_x: Option<i64>,
    /// Optional Y-axis boundary for coordinate-based actions
    #[serde(default)]
    pub max_coordinate_y: Option<i64>,
}

fn default_browser_computer_use_endpoint() -> String {
    "http://127.0.0.1:8787/v1/actions".into()
}

fn default_browser_computer_use_timeout_ms() -> u64 {
    15_000
}

impl Default for BrowserComputerUseConfig {
    fn default() -> Self {
        Self {
            endpoint: default_browser_computer_use_endpoint(),
            api_key: None,
            timeout_ms: default_browser_computer_use_timeout_ms(),
            allow_remote_endpoint: false,
            window_allowlist: Vec::new(),
            max_coordinate_x: None,
            max_coordinate_y: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Enable `browser_open` tool (opens URLs in Brave without scraping)
    #[serde(default)]
    pub enabled: bool,
    /// Allowed domains for `browser_open` (exact or subdomain match)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Browser session name (for agent-browser automation)
    #[serde(default)]
    pub session_name: Option<String>,
    /// Browser automation backend: "agent_browser" | "rust_native" | "computer_use" | "auto"
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    /// Headless mode for rust-native backend
    #[serde(default = "default_true")]
    pub native_headless: bool,
    /// WebDriver endpoint URL for rust-native backend (e.g. http://127.0.0.1:9515)
    #[serde(default = "default_browser_webdriver_url")]
    pub native_webdriver_url: String,
    /// Optional Chrome/Chromium executable path for rust-native backend
    #[serde(default)]
    pub native_chrome_path: Option<String>,
    /// Computer-use sidecar configuration
    #[serde(default)]
    pub computer_use: BrowserComputerUseConfig,
}

fn default_browser_backend() -> String {
    "agent_browser".into()
}

fn default_browser_webdriver_url() -> String {
    "http://127.0.0.1:9515".into()
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: Vec::new(),
            session_name: None,
            backend: default_browser_backend(),
            native_headless: default_true(),
            native_webdriver_url: default_browser_webdriver_url(),
            native_chrome_path: None,
            computer_use: BrowserComputerUseConfig::default(),
        }
    }
}

// ── HTTP request tool ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpRequestConfig {
    /// Enable `http_request` tool for API interactions
    #[serde(default)]
    pub enabled: bool,
    /// Allowed domains for HTTP requests (exact or subdomain match)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Maximum response size in bytes (default: 1MB)
    #[serde(default = "default_http_max_response_size")]
    pub max_response_size: usize,
    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_http_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_http_max_response_size() -> usize {
    1_000_000 // 1MB
}

fn default_http_timeout_secs() -> u64 {
    30
}

// ── Web search ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// Enable `web_search_tool` for web searches
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Search provider: "duckduckgo" (free, no API key) or "brave" (requires API key)
    #[serde(default = "default_web_search_provider")]
    pub provider: String,
    /// Brave Search API key (required if provider is "brave")
    #[serde(default)]
    pub brave_api_key: Option<String>,
    /// Maximum results per search (1-10)
    #[serde(default = "default_web_search_max_results")]
    pub max_results: usize,
    /// Request timeout in seconds
    #[serde(default = "default_web_search_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_web_search_provider() -> String {
    "duckduckgo".into()
}

fn default_web_search_max_results() -> usize {
    5
}

fn default_web_search_timeout_secs() -> u64 {
    15
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: default_web_search_provider(),
            brave_api_key: None,
            max_results: default_web_search_max_results(),
            timeout_secs: default_web_search_timeout_secs(),
        }
    }
}

// -- MCP ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_mcp_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
    #[serde(default = "default_mcp_call_timeout_ms")]
    pub call_timeout_ms: u64,
    #[serde(default = "default_mcp_output_limit_bytes")]
    pub output_limit_bytes: usize,
    /// Which MCP capability types to discover and register.
    /// Default: `["tools"]` for backward compatibility.
    /// Valid values: `"tools"`, `"resources"`, `"prompts"`
    #[serde(default = "default_mcp_capabilities")]
    pub capabilities: Vec<String>,
    /// Optional per-capability output limit override for resources.
    #[serde(default)]
    pub resource_output_limit_bytes: Option<usize>,
    /// Optional per-capability output limit override for prompts.
    #[serde(default)]
    pub prompt_output_limit_bytes: Option<usize>,
}

fn default_mcp_startup_timeout_ms() -> u64 {
    5_000
}

fn default_mcp_call_timeout_ms() -> u64 {
    30_000
}

fn default_mcp_output_limit_bytes() -> usize {
    64 * 1024
}

pub fn default_mcp_capabilities() -> Vec<String> {
    vec!["tools".to_string()]
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            startup_timeout_ms: default_mcp_startup_timeout_ms(),
            call_timeout_ms: default_mcp_call_timeout_ms(),
            output_limit_bytes: default_mcp_output_limit_bytes(),
            capabilities: default_mcp_capabilities(),
            resource_output_limit_bytes: None,
            prompt_output_limit_bytes: None,
        }
    }
}

// ── Memory ───────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryCerebroConfig {
    /// MCP endpoint URL, e.g. "https://cerebro.example.com/mcp" or "wss://cerebro.example.com/mcp"
    #[serde(default)]
    pub endpoint: Option<String>,
    /// MCP auth token for Cerebro
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Request timeout in milliseconds
    #[serde(default = "default_cerebro_timeout_ms")]
    pub request_timeout_ms: u64,
    /// Allow plain HTTP/WS for loopback addresses only.
    #[serde(default)]
    pub allow_insecure_loopback: bool,
}

fn default_cerebro_timeout_ms() -> u64 {
    30_000
}

impl Default for MemoryCerebroConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            auth_token: None,
            request_timeout_ms: default_cerebro_timeout_ms(),
            allow_insecure_loopback: false,
        }
    }
}

impl fmt::Debug for MemoryCerebroConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryCerebroConfig")
            .field("endpoint", &self.endpoint)
            .field(
                "auth_token",
                &self.auth_token.as_ref().map(|_| "<redacted>"),
            )
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field("allow_insecure_loopback", &self.allow_insecure_loopback)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MemoryConfig {
    /// "sqlite" | "lucid" | "markdown" | "none" (`none` = explicit no-op memory)
    pub backend: String,
    /// Auto-save conversation context to memory
    pub auto_save: bool,
    /// Run memory/session hygiene (archiving + retention cleanup)
    #[serde(default = "default_hygiene_enabled")]
    pub hygiene_enabled: bool,
    /// Archive daily/session files older than this many days
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u32,
    /// Purge archived files older than this many days
    #[serde(default = "default_purge_after_days")]
    pub purge_after_days: u32,
    /// For sqlite backend: prune conversation rows older than this many days
    #[serde(default = "default_conversation_retention_days")]
    pub conversation_retention_days: u32,
    /// Embedding provider: "none" | "openai" | "custom:URL"
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    /// Embedding model name (e.g. "text-embedding-3-small")
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Embedding vector dimensions
    #[serde(default = "default_embedding_dims")]
    pub embedding_dimensions: usize,
    /// Weight for vector similarity in hybrid search (0.0–1.0)
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    /// Weight for keyword BM25 in hybrid search (0.0–1.0)
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f64,
    /// Minimum hybrid score (0.0–1.0) for a memory to be included in context.
    /// Memories scoring below this threshold are dropped to prevent irrelevant
    /// context from bleeding into conversations. Default: 0.4
    #[serde(default = "default_min_relevance_score")]
    pub min_relevance_score: f64,
    /// Max embedding cache entries before LRU eviction
    #[serde(default = "default_cache_size")]
    pub embedding_cache_size: usize,
    /// Max tokens per chunk for document splitting
    #[serde(default = "default_chunk_size")]
    pub chunk_max_tokens: usize,

    // ── Response Cache (saves tokens on repeated prompts) ──────
    /// Enable LLM response caching to avoid paying for duplicate prompts
    #[serde(default)]
    pub response_cache_enabled: bool,
    /// TTL in minutes for cached responses (default: 60)
    #[serde(default = "default_response_cache_ttl")]
    pub response_cache_ttl_minutes: u32,
    /// Max number of cached responses before LRU eviction (default: 5000)
    #[serde(default = "default_response_cache_max")]
    pub response_cache_max_entries: usize,

    // ── Memory Snapshot (soul backup to Markdown) ─────────────
    /// Enable periodic export of core memories to MEMORY_SNAPSHOT.md
    #[serde(default)]
    pub snapshot_enabled: bool,
    /// Run snapshot during hygiene passes (heartbeat-driven)
    #[serde(default)]
    pub snapshot_on_hygiene: bool,
    /// Auto-hydrate from MEMORY_SNAPSHOT.md when brain.db is missing
    #[serde(default = "default_true")]
    pub auto_hydrate: bool,

    // ── SQLite backend options ─────────────────────────────────
    /// For sqlite backend: max seconds to wait when opening the DB (e.g. file locked).
    /// None = wait indefinitely (default). Recommended max: 300.
    #[serde(default)]
    pub sqlite_open_timeout_secs: Option<u64>,

    /// Cerebro MCP endpoint settings.
    #[serde(default)]
    pub cerebro: MemoryCerebroConfig,
}

fn default_embedding_provider() -> String {
    "none".into()
}
fn default_hygiene_enabled() -> bool {
    true
}
fn default_archive_after_days() -> u32 {
    7
}
fn default_purge_after_days() -> u32 {
    30
}
fn default_conversation_retention_days() -> u32 {
    30
}
fn default_embedding_model() -> String {
    "text-embedding-3-small".into()
}
fn default_embedding_dims() -> usize {
    1536
}
fn default_vector_weight() -> f64 {
    0.7
}
fn default_keyword_weight() -> f64 {
    0.3
}
fn default_min_relevance_score() -> f64 {
    0.4
}
fn default_cache_size() -> usize {
    10_000
}
fn default_chunk_size() -> usize {
    512
}
fn default_response_cache_ttl() -> u32 {
    60
}
fn default_response_cache_max() -> usize {
    5_000
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            auto_save: true,
            hygiene_enabled: default_hygiene_enabled(),
            archive_after_days: default_archive_after_days(),
            purge_after_days: default_purge_after_days(),
            conversation_retention_days: default_conversation_retention_days(),
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
            embedding_dimensions: default_embedding_dims(),
            vector_weight: default_vector_weight(),
            keyword_weight: default_keyword_weight(),
            min_relevance_score: default_min_relevance_score(),
            embedding_cache_size: default_cache_size(),
            chunk_max_tokens: default_chunk_size(),
            response_cache_enabled: false,
            response_cache_ttl_minutes: default_response_cache_ttl(),
            response_cache_max_entries: default_response_cache_max(),
            snapshot_enabled: false,
            snapshot_on_hygiene: false,
            auto_hydrate: true,
            sqlite_open_timeout_secs: None,
            cerebro: MemoryCerebroConfig::default(),
        }
    }
}

// ── Observability ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// "none" | "log" | "prometheus" | "otel"
    pub backend: String,

    /// OTLP endpoint (e.g. "http://localhost:4318"). Only used when backend = "otel".
    #[serde(default)]
    pub otel_endpoint: Option<String>,

    /// Service name reported to the OTel collector. Defaults to "corvus".
    #[serde(default)]
    pub otel_service_name: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            backend: "none".into(),
            otel_endpoint: None,
            otel_service_name: None,
        }
    }
}

// ── Autonomy / Security ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AutonomyConfig {
    pub level: AutonomyLevel,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: u32,

    /// Require explicit approval for medium-risk shell commands.
    #[serde(default = "default_true")]
    pub require_approval_for_medium_risk: bool,

    /// Block high-risk shell commands even if allowlisted.
    #[serde(default = "default_true")]
    pub block_high_risk_commands: bool,

    /// Tools that never require approval (e.g. read-only tools).
    #[serde(default = "default_auto_approve")]
    pub auto_approve: Vec<String>,

    /// Tools that always require interactive approval, even after "Always".
    #[serde(default = "default_always_ask")]
    pub always_ask: Vec<String>,

    #[serde(skip, default)]
    pub deprecated_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAutonomyConfig {
    #[serde(default)]
    level: Option<AutonomyLevel>,
    #[serde(default)]
    workspace_only: Option<bool>,
    #[serde(default)]
    allowed_commands: Option<Vec<String>>,
    #[serde(default)]
    forbidden_paths: Option<Vec<String>>,
    #[serde(default)]
    max_actions_per_hour: Option<u32>,
    #[serde(default)]
    max_cost_per_day_cents: Option<u32>,
    #[serde(default)]
    require_approval_for_medium_risk: Option<bool>,
    #[serde(default)]
    block_high_risk_commands: Option<bool>,
    #[serde(default)]
    auto_approve: Option<Vec<String>>,
    #[serde(default)]
    always_ask: Option<Vec<String>>,
}

fn default_auto_approve() -> Vec<String> {
    vec!["file_read".into(), "memory_recall".into()]
}

fn default_always_ask() -> Vec<String> {
    vec![]
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            max_actions_per_hour: 20,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
            auto_approve: default_auto_approve(),
            always_ask: default_always_ask(),
            deprecated_fields: Vec::new(),
        }
    }
}

impl<'de> Deserialize<'de> for AutonomyConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawAutonomyConfig::deserialize(deserializer)?;
        let mut config = Self::default();

        if let Some(level) = raw.level {
            config.level = level;
        }
        if let Some(workspace_only) = raw.workspace_only {
            config.workspace_only = workspace_only;
        }
        if let Some(allowed_commands) = raw.allowed_commands {
            config.allowed_commands = allowed_commands;
        }
        if let Some(forbidden_paths) = raw.forbidden_paths {
            config.forbidden_paths = forbidden_paths;
        }
        if let Some(max_actions_per_hour) = raw.max_actions_per_hour {
            config.max_actions_per_hour = max_actions_per_hour;
        }
        if let Some(max_cost_per_day_cents) = raw.max_cost_per_day_cents {
            config
                .deprecated_fields
                .push("autonomy.max_cost_per_day_cents".to_string());
            if raw.max_actions_per_hour.is_none() {
                config.max_actions_per_hour = max_cost_per_day_cents;
            }
        }
        if let Some(require_approval_for_medium_risk) = raw.require_approval_for_medium_risk {
            config.require_approval_for_medium_risk = require_approval_for_medium_risk;
        }
        if let Some(block_high_risk_commands) = raw.block_high_risk_commands {
            config.block_high_risk_commands = block_high_risk_commands;
        }
        if let Some(auto_approve) = raw.auto_approve {
            config.auto_approve = auto_approve;
        }
        if let Some(always_ask) = raw.always_ask {
            config.always_ask = always_ask;
        }

        Ok(config)
    }
}

impl AutonomyConfig {
    pub fn deprecated_fields(&self) -> &[String] {
        &self.deprecated_fields
    }

    pub fn action_rate_deprecation_warning(&self) -> Option<String> {
        self.deprecated_fields
            .iter()
            .any(|field| field == "autonomy.max_cost_per_day_cents")
            .then(|| {
                "autonomy.max_cost_per_day_cents is deprecated and has been normalized to autonomy.max_actions_per_hour".to_string()
            })
    }
}

// ── Runtime ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime kind (`native` | `docker`).
    #[serde(default = "default_runtime_kind")]
    pub kind: String,

    /// Docker runtime settings (used when `kind = "docker"`).
    #[serde(default)]
    pub docker: DockerRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRuntimeConfig {
    /// Runtime image used to execute shell commands.
    #[serde(default = "default_docker_image")]
    pub image: String,

    /// Docker network mode (`none`, `bridge`, etc.).
    #[serde(default = "default_docker_network")]
    pub network: String,

    /// Optional memory limit in MB (`None` = no explicit limit).
    #[serde(default = "default_docker_memory_limit_mb")]
    pub memory_limit_mb: Option<u64>,

    /// Optional CPU limit (`None` = no explicit limit).
    #[serde(default = "default_docker_cpu_limit")]
    pub cpu_limit: Option<f64>,

    /// Mount root filesystem as read-only.
    #[serde(default = "default_true")]
    pub read_only_rootfs: bool,

    /// Mount configured workspace into `/workspace`.
    #[serde(default = "default_true")]
    pub mount_workspace: bool,

    /// Optional workspace root allowlist for Docker mount validation.
    #[serde(default)]
    pub allowed_workspace_roots: Vec<String>,
}

fn default_runtime_kind() -> String {
    "native".into()
}

fn default_docker_image() -> String {
    "alpine:3.20".into()
}

fn default_docker_network() -> String {
    "none".into()
}

fn default_docker_memory_limit_mb() -> Option<u64> {
    Some(512)
}

fn default_docker_cpu_limit() -> Option<f64> {
    Some(1.0)
}

impl Default for DockerRuntimeConfig {
    fn default() -> Self {
        Self {
            image: default_docker_image(),
            network: default_docker_network(),
            memory_limit_mb: default_docker_memory_limit_mb(),
            cpu_limit: default_docker_cpu_limit(),
            read_only_rootfs: true,
            mount_workspace: true,
            allowed_workspace_roots: Vec::new(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: default_runtime_kind(),
            docker: DockerRuntimeConfig::default(),
        }
    }
}

// ── Reliability / supervision ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccountPoolStrategy {
    #[default]
    RoundRobin,
    WeightedRoundRobin,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderAccountConfig {
    pub id: String,
    pub api_key: String,
    #[serde(default)]
    pub api_url: Option<String>,
    #[serde(default = "default_account_weight")]
    pub weight: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl fmt::Debug for ProviderAccountConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderAccountConfig")
            .field("id", &self.id)
            .field("api_key", &"<redacted>")
            .field("api_url", &self.api_url)
            .field("weight", &self.weight)
            .field("enabled", &self.enabled)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAccountPoolConfig {
    #[serde(default)]
    pub strategy: AccountPoolStrategy,
    pub accounts: Vec<ProviderAccountConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Retries per provider before failing over.
    #[serde(default = "default_provider_retries")]
    pub provider_retries: u32,
    /// Base backoff (ms) for provider retry delay.
    #[serde(default = "default_provider_backoff_ms")]
    pub provider_backoff_ms: u64,
    /// Fallback provider chain (e.g. `["anthropic", "openai"]`).
    #[serde(default)]
    pub fallback_providers: Vec<String>,
    /// Additional API keys for round-robin rotation on rate-limit (429) errors.
    /// The primary `api_key` is always tried first; these are extras.
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Per-model fallback chains. When a model fails, try these alternatives in order.
    /// Example: `{ "claude-opus-4-20250514" = ["claude-sonnet-4-20250514", "gpt-4o"] }`
    #[serde(default)]
    pub model_fallbacks: std::collections::HashMap<String, Vec<String>>,
    /// Provider-specific account pools keyed by provider name.
    #[serde(default)]
    pub account_pools: std::collections::HashMap<String, ProviderAccountPoolConfig>,
    /// Initial backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_secs")]
    pub channel_initial_backoff_secs: u64,
    /// Max backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_max_secs")]
    pub channel_max_backoff_secs: u64,
    /// Scheduler polling cadence in seconds.
    #[serde(default = "default_scheduler_poll_secs")]
    pub scheduler_poll_secs: u64,
    /// Max retries for cron job execution attempts.
    #[serde(default = "default_scheduler_retries")]
    pub scheduler_retries: u32,
}

fn default_provider_retries() -> u32 {
    2
}

fn default_provider_backoff_ms() -> u64 {
    500
}

fn default_account_weight() -> u32 {
    1
}

fn default_channel_backoff_secs() -> u64 {
    2
}

fn default_channel_backoff_max_secs() -> u64 {
    60
}

fn default_scheduler_poll_secs() -> u64 {
    15
}

fn default_scheduler_retries() -> u32 {
    2
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            provider_retries: default_provider_retries(),
            provider_backoff_ms: default_provider_backoff_ms(),
            fallback_providers: Vec::new(),
            api_keys: Vec::new(),
            model_fallbacks: std::collections::HashMap::new(),
            account_pools: std::collections::HashMap::new(),
            channel_initial_backoff_secs: default_channel_backoff_secs(),
            channel_max_backoff_secs: default_channel_backoff_max_secs(),
            scheduler_poll_secs: default_scheduler_poll_secs(),
            scheduler_retries: default_scheduler_retries(),
        }
    }
}

// ── Scheduler ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Enable the built-in scheduler loop.
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    /// Maximum number of persisted scheduled tasks.
    #[serde(default = "default_scheduler_max_tasks")]
    pub max_tasks: usize,
    /// Maximum tasks executed per scheduler polling cycle.
    #[serde(default = "default_scheduler_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_scheduler_max_tasks() -> usize {
    64
}

fn default_scheduler_max_concurrent() -> usize {
    4
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            max_tasks: default_scheduler_max_tasks(),
            max_concurrent: default_scheduler_max_concurrent(),
        }
    }
}

// ── Model routing ────────────────────────────────────────────────

/// Route a task hint to a specific provider + model.
///
/// ```toml
/// [[model_routes]]
/// hint = "reasoning"
/// provider = "openrouter"
/// model = "anthropic/claude-opus-4-20250514"
///
/// [[model_routes]]
/// hint = "fast"
/// provider = "groq"
/// model = "llama-3.3-70b-versatile"
/// ```
///
/// Usage: pass `hint:reasoning` as the model parameter to route the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRouteConfig {
    /// Task hint name (e.g. "reasoning", "fast", "code", "summarize")
    pub hint: String,
    /// Provider to route to (must match a known provider name)
    pub provider: String,
    /// Model to use with that provider
    pub model: String,
    /// Optional API key override for this route's provider
    #[serde(default)]
    pub api_key: Option<String>,
    /// Explicit opt-in for multimodal image routing.
    #[serde(default)]
    pub allow_image_input: bool,
}

// ── Query Classification ─────────────────────────────────────────

/// Automatic query classification — classifies user messages by keyword/pattern
/// and routes to the appropriate model hint. Disabled by default.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryClassificationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<ClassificationRule>,
}

/// A single classification rule mapping message patterns to a model hint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationRule {
    /// Must match a `[[model_routes]]` hint value.
    pub hint: String,
    /// Case-insensitive substring matches.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Case-sensitive literal matches (for "```", "fn ", etc.).
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Only match if message length >= N chars.
    #[serde(default)]
    pub min_length: Option<usize>,
    /// Only match if message length <= N chars.
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Higher priority rules are checked first.
    #[serde(default)]
    pub priority: i32,
}

// ── Heartbeat ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_minutes: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 30,
        }
    }
}

// ── Cron ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_run_history")]
    pub max_run_history: u32,
}

fn default_max_run_history() -> u32 {
    50
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_run_history: default_max_run_history(),
        }
    }
}

// ── Tunnel ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// "none", "cloudflare", "tailscale", "ngrok", "custom"
    pub provider: String,

    #[serde(default)]
    pub cloudflare: Option<CloudflareTunnelConfig>,

    #[serde(default)]
    pub tailscale: Option<TailscaleTunnelConfig>,

    #[serde(default)]
    pub ngrok: Option<NgrokTunnelConfig>,

    #[serde(default)]
    pub custom: Option<CustomTunnelConfig>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            cloudflare: None,
            tailscale: None,
            ngrok: None,
            custom: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareTunnelConfig {
    /// Cloudflare Tunnel token (from Zero Trust dashboard)
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleTunnelConfig {
    /// Use Tailscale Funnel (public internet) vs Serve (tailnet only)
    #[serde(default)]
    pub funnel: bool,
    /// Optional hostname override
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgrokTunnelConfig {
    /// ngrok auth token
    pub auth_token: String,
    /// Optional custom domain
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTunnelConfig {
    /// Command template to start the tunnel. Use {port} and {host} placeholders.
    /// Example: "bore local {port} --to bore.pub"
    pub start_command: String,
    /// Optional URL to check tunnel health
    pub health_url: Option<String>,
    /// Optional regex to extract public URL from command stdout
    pub url_pattern: Option<String>,
}

// ── Channels ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub cli: bool,
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub mattermost: Option<MattermostConfig>,
    pub webhook: Option<WebhookConfig>,
    pub imessage: Option<IMessageConfig>,
    pub matrix: Option<MatrixConfig>,
    pub signal: Option<SignalConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub email: Option<crate::channels::email_channel::EmailConfig>,
    pub irc: Option<IrcConfig>,
    pub lark: Option<LarkConfig>,
    pub dingtalk: Option<DingTalkConfig>,
    pub qq: Option<QQConfig>,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

/// Streaming mode for channels that support progressive message updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    /// No streaming -- send the complete response as a single message (default).
    #[default]
    Off,
    /// Update a draft message with every flush interval.
    Partial,
}

fn default_draft_update_interval_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
    /// Streaming mode for progressive response delivery via message edits.
    #[serde(default)]
    pub stream_mode: StreamMode,
    /// Minimum interval (ms) between draft message edits to avoid rate limits.
    #[serde(default = "default_draft_update_interval_ms")]
    pub draft_update_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub guild_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// When true, process messages from other bots (not just humans).
    /// The bot still ignores its own messages to prevent feedback loops.
    #[serde(default)]
    pub listen_to_bots: bool,
    /// When true, only respond to messages that @-mention the bot.
    /// Other messages in the guild are silently ignored.
    #[serde(default)]
    pub mention_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    pub url: String,
    pub bot_token: String,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// When true (default), replies thread on the original post.
    /// When false, replies go to the channel root.
    #[serde(default)]
    pub thread_replies: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub port: u16,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    pub allowed_contacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver: String,
    pub access_token: String,
    pub room_id: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Base URL for the signal-cli HTTP daemon (e.g. "http://127.0.0.1:8686").
    pub http_url: String,
    /// E.164 phone number of the signal-cli account (e.g. "+1234567890").
    pub account: String,
    /// Optional group ID to filter messages.
    /// - `None` or omitted: accept all messages (DMs and groups)
    /// - `"dm"`: only accept direct messages
    /// - Specific group ID: only accept messages from that group
    #[serde(default)]
    pub group_id: Option<String>,
    /// Allowed sender phone numbers (E.164) or "*" for all.
    #[serde(default)]
    pub allowed_from: Vec<String>,
    /// Skip messages that are attachment-only (no text body).
    #[serde(default)]
    pub ignore_attachments: bool,
    /// Skip incoming story messages.
    #[serde(default)]
    pub ignore_stories: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Access token from Meta Business Suite
    pub access_token: String,
    /// Phone number ID from Meta Business API
    pub phone_number_id: String,
    /// Webhook verify token (you define this, Meta sends it back for verification)
    pub verify_token: String,
    /// App secret from Meta Business Suite (for webhook signature verification)
    /// Can also be set via `CORVUS_WHATSAPP_APP_SECRET` environment variable
    #[serde(default)]
    pub app_secret: Option<String>,
    /// Allowed phone numbers (E.164 format: +1234567890) or "*" for all
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcConfig {
    /// IRC server hostname
    pub server: String,
    /// IRC server port (default: 6697 for TLS)
    #[serde(default = "default_irc_port")]
    pub port: u16,
    /// Bot nickname
    pub nickname: String,
    /// Username (defaults to nickname if not set)
    pub username: Option<String>,
    /// Channels to join on connect
    #[serde(default)]
    pub channels: Vec<String>,
    /// Allowed nicknames (case-insensitive) or "*" for all
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Server password (for bouncers like ZNC)
    pub server_password: Option<String>,
    /// NickServ IDENTIFY password
    pub nickserv_password: Option<String>,
    /// SASL PLAIN password (IRCv3)
    pub sasl_password: Option<String>,
    /// Verify TLS certificate (default: true)
    pub verify_tls: Option<bool>,
}

fn default_irc_port() -> u16 {
    6697
}

/// How Corvus receives events from Feishu / Lark.
///
/// - `websocket` (default) — persistent WSS long-connection; no public URL required.
/// - `webhook`             — HTTP callback server; requires a public HTTPS endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LarkReceiveMode {
    #[default]
    Websocket,
    Webhook,
}

/// Lark/Feishu configuration for messaging integration.
/// Lark is the international version; Feishu is the Chinese version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkConfig {
    /// App ID from Lark/Feishu developer console
    pub app_id: String,
    /// App Secret from Lark/Feishu developer console
    pub app_secret: String,
    /// Encrypt key for webhook message decryption (optional)
    #[serde(default)]
    pub encrypt_key: Option<String>,
    /// Verification token for webhook validation (optional)
    #[serde(default)]
    pub verification_token: Option<String>,
    /// Allowed user IDs or union IDs (empty = deny all, "*" = allow all)
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Whether to use the Feishu (Chinese) endpoint instead of Lark (International)
    #[serde(default)]
    pub use_feishu: bool,
    /// Event receive mode: "websocket" (default) or "webhook"
    #[serde(default)]
    pub receive_mode: LarkReceiveMode,
    /// HTTP port for webhook mode only. Must be set when receive_mode = "webhook".
    /// Not required (and ignored) for websocket mode.
    #[serde(default)]
    pub port: Option<u16>,
}

// ── Security Config ─────────────────────────────────────────────────

/// Security configuration for sandboxing, resource limits, and audit logging
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// Resource limits
    #[serde(default)]
    pub resources: ResourceLimitsConfig,

    /// Audit logging configuration
    #[serde(default)]
    pub audit: AuditConfig,
}

/// Sandbox configuration for OS-level isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing (None = auto-detect, Some = explicit)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Sandbox backend to use
    #[serde(default)]
    pub backend: SandboxBackend,

    /// When true, refuse to start if no OS-level sandbox backend is available.
    /// When false (default), fall back to NoopSandbox with a warning.
    #[serde(default)]
    pub require: bool,

    /// Custom Firejail arguments (when backend = firejail)
    #[serde(default)]
    pub firejail_args: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: None, // Auto-detect
            backend: SandboxBackend::Auto,
            require: false,
            firejail_args: Vec::new(),
        }
    }
}

/// Sandbox backend selection
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SandboxBackend {
    /// Auto-detect best available (default)
    #[default]
    Auto,
    /// Landlock (Linux kernel LSM, native)
    Landlock,
    /// Firejail (user-space sandbox)
    Firejail,
    /// Bubblewrap (user namespaces)
    Bubblewrap,
    /// Docker container isolation
    Docker,
    /// No sandboxing (application-layer only)
    None,
}

/// Resource limits for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum memory in MB per command
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,

    /// Maximum CPU time in seconds per command
    #[serde(default = "default_max_cpu_time_seconds")]
    pub max_cpu_time_seconds: u64,

    /// Maximum number of subprocesses
    #[serde(default = "default_max_subprocesses")]
    pub max_subprocesses: u32,

    /// Enable memory monitoring
    #[serde(default = "default_memory_monitoring_enabled")]
    pub memory_monitoring: bool,
}

fn default_max_memory_mb() -> u32 {
    512
}

fn default_max_cpu_time_seconds() -> u64 {
    60
}

fn default_max_subprocesses() -> u32 {
    10
}

fn default_memory_monitoring_enabled() -> bool {
    true
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_cpu_time_seconds: default_max_cpu_time_seconds(),
            max_subprocesses: default_max_subprocesses(),
            memory_monitoring: default_memory_monitoring_enabled(),
        }
    }
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,

    /// Path to audit log file (relative to corvus dir)
    #[serde(default = "default_audit_log_path")]
    pub log_path: String,

    /// Maximum log size in MB before rotation
    #[serde(default = "default_audit_max_size_mb")]
    pub max_size_mb: u32,

    /// Fail startup if audit logging cannot be initialized
    #[serde(default)]
    pub strict: bool,

    /// Sign events with HMAC for tamper evidence
    #[serde(default)]
    pub sign_events: bool,
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_log_path() -> String {
    "audit.log".to_string()
}

fn default_audit_max_size_mb() -> u32 {
    100
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_enabled(),
            log_path: default_audit_log_path(),
            max_size_mb: default_audit_max_size_mb(),
            strict: false,
            sign_events: false,
        }
    }
}

/// DingTalk configuration for Stream Mode messaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// Client ID (AppKey) from DingTalk developer console
    pub client_id: String,
    /// Client Secret (AppSecret) from DingTalk developer console
    pub client_secret: String,
    /// Allowed user IDs (staff IDs). Empty = deny all, "*" = allow all
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// QQ Official Bot configuration (Tencent QQ Bot SDK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QQConfig {
    /// App ID from QQ Bot developer console
    pub app_id: String,
    /// App Secret from QQ Bot developer console
    pub app_secret: String,
    /// Allowed user IDs. Empty = deny all, "*" = allow all
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

// ── Config impl ──────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        let home =
            UserDirs::new().map_or_else(|| PathBuf::from("."), |u| u.home_dir().to_path_buf());
        let corvus_dir = home.join(".corvus");

        Self {
            workspace_dir: corvus_dir.join("workspace"),
            config_path: corvus_dir.join("config.toml"),
            api_key: None,
            api_url: None,
            default_provider: Some("openrouter".to_string()),
            default_model: Some("anthropic/claude-sonnet-4".to_string()),
            default_temperature: 0.7,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            security: SecurityConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            agent: AgentConfig::default(),
            mission: MissionConfig::default(),
            model_routes: Vec::new(),
            heartbeat: HeartbeatConfig::default(),
            cron: CronConfig::default(),
            channels_config: ChannelsConfig::default(),
            updates: UpdateConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            web_search: WebSearchConfig::default(),
            mcp: McpConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            query_classification: QueryClassificationConfig::default(),
            skills: SkillsConfig::default(),
            multimodal: MultimodalConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdateConfig {
    /// Enable periodic update checks + notifications in daemon mode.
    #[serde(default = "default_updates_enabled")]
    pub enabled: bool,
    /// Auto-install policy; disabled by default for safety.
    #[serde(default)]
    pub auto_install_enabled: bool,
    /// Channel-side update visibility.
    #[serde(default = "default_true")]
    pub channel_visibility_enabled: bool,
    /// CLI startup notice visibility.
    #[serde(default = "default_true")]
    pub cli_startup_notice_enabled: bool,
    /// Poll interval for update checks while daemon is running.
    #[serde(
        default = "default_update_check_interval_minutes",
        deserialize_with = "deserialize_nonzero_u64"
    )]
    pub check_interval_minutes: u64,
    /// Lifetime for a confirmation nonce before it expires.
    #[serde(
        default = "default_update_confirmation_ttl_minutes",
        deserialize_with = "deserialize_nonzero_u64"
    )]
    pub confirmation_ttl_minutes: u64,
    /// Per-channel destination overrides for update notifications.
    ///
    /// Key: channel name (e.g. telegram, slack, discord)
    /// Value: list of destination identifiers for that channel.
    #[serde(default)]
    pub notify_destinations: HashMap<String, Vec<String>>,
    /// Optional install method override.
    #[serde(default)]
    pub install_method_override: Option<String>,
    /// Restart policy after successful install.
    #[serde(default = "default_update_restart_policy")]
    pub restart_policy: String,
    /// Maximum retained history entries.
    #[serde(default = "default_update_history_max_entries")]
    pub history_max_entries: u32,
}

fn deserialize_nonzero_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    if value == 0 {
        Err(serde::de::Error::custom(
            "value must be greater than zero (zero would cause a busy-loop or instant TTL expiry)",
        ))
    } else {
        Ok(value)
    }
}

fn default_updates_enabled() -> bool {
    true
}

fn default_update_check_interval_minutes() -> u64 {
    30
}

fn default_update_confirmation_ttl_minutes() -> u64 {
    30
}

fn default_update_restart_policy() -> String {
    "prompt".to_string()
}

fn default_update_history_max_entries() -> u32 {
    200
}

fn normalize_install_method_override(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "npm" | "pnpm" | "yarn" | "bun" | "homebrew" | "cargo" | "script_binary" => {
            Some(normalized)
        }
        _ => None,
    }
}

fn normalize_restart_policy(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "never" | "prompt" | "auto_managed_service" => Some(normalized),
        _ => None,
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: default_updates_enabled(),
            auto_install_enabled: false,
            channel_visibility_enabled: true,
            cli_startup_notice_enabled: true,
            check_interval_minutes: default_update_check_interval_minutes(),
            confirmation_ttl_minutes: default_update_confirmation_ttl_minutes(),
            notify_destinations: HashMap::new(),
            install_method_override: None,
            restart_policy: default_update_restart_policy(),
            history_max_entries: default_update_history_max_entries(),
        }
    }
}

fn default_config_and_workspace_dirs() -> Result<(PathBuf, PathBuf)> {
    let config_dir = default_config_dir()?;
    Ok((config_dir.clone(), config_dir.join("workspace")))
}

const ACTIVE_WORKSPACE_STATE_FILE: &str = "active_workspace.toml";

#[derive(Debug, Serialize, Deserialize)]
struct ActiveWorkspaceState {
    config_dir: String,
}

fn default_config_dir() -> Result<PathBuf> {
    let home = UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home.join(".corvus"))
}

fn active_workspace_state_path(default_dir: &Path) -> PathBuf {
    default_dir.join(ACTIVE_WORKSPACE_STATE_FILE)
}

fn load_persisted_workspace_dirs(default_config_dir: &Path) -> Result<Option<(PathBuf, PathBuf)>> {
    let state_path = active_workspace_state_path(default_config_dir);
    if !state_path.exists() {
        return Ok(None);
    }

    let contents = match fs::read_to_string(&state_path) {
        Ok(contents) => contents,
        Err(error) => {
            tracing::warn!(
                "Failed to read active workspace marker {}: {error}",
                state_path.display()
            );
            return Ok(None);
        }
    };

    let state: ActiveWorkspaceState = match toml::from_str(&contents) {
        Ok(state) => state,
        Err(error) => {
            tracing::warn!(
                "Failed to parse active workspace marker {}: {error}",
                state_path.display()
            );
            return Ok(None);
        }
    };

    let raw_config_dir = state.config_dir.trim();
    if raw_config_dir.is_empty() {
        tracing::warn!(
            "Ignoring active workspace marker {} because config_dir is empty",
            state_path.display()
        );
        return Ok(None);
    }

    let parsed_dir = PathBuf::from(raw_config_dir);
    let config_dir = if parsed_dir.is_absolute() {
        parsed_dir
    } else {
        default_config_dir.join(parsed_dir)
    };
    Ok(Some((config_dir.clone(), config_dir.join("workspace"))))
}

pub(crate) fn persist_active_workspace_config_dir(config_dir: &Path) -> Result<()> {
    let default_config_dir = default_config_dir()?;
    let state_path = active_workspace_state_path(&default_config_dir);

    if config_dir == default_config_dir {
        if state_path.exists() {
            fs::remove_file(&state_path).with_context(|| {
                format!(
                    "Failed to clear active workspace marker: {}",
                    state_path.display()
                )
            })?;
        }
        return Ok(());
    }

    fs::create_dir_all(&default_config_dir).with_context(|| {
        format!(
            "Failed to create default config directory: {}",
            default_config_dir.display()
        )
    })?;

    let state = ActiveWorkspaceState {
        config_dir: config_dir.to_string_lossy().into_owned(),
    };
    let serialized =
        toml::to_string_pretty(&state).context("Failed to serialize active workspace marker")?;

    let temp_path = default_config_dir.join(format!(
        ".{ACTIVE_WORKSPACE_STATE_FILE}.tmp-{}",
        uuid::Uuid::new_v4()
    ));
    fs::write(&temp_path, serialized).with_context(|| {
        format!(
            "Failed to write temporary active workspace marker: {}",
            temp_path.display()
        )
    })?;

    if let Err(error) = fs::rename(&temp_path, &state_path) {
        let _ = fs::remove_file(&temp_path);
        anyhow::bail!(
            "Failed to atomically persist active workspace marker {}: {error}",
            state_path.display()
        );
    }

    sync_directory(&default_config_dir)?;
    Ok(())
}

fn resolve_config_dir_for_workspace(workspace_dir: &Path) -> (PathBuf, PathBuf) {
    let workspace_config_dir = workspace_dir.to_path_buf();
    if workspace_config_dir.join("config.toml").exists() {
        return (
            workspace_config_dir.clone(),
            workspace_config_dir.join("workspace"),
        );
    }

    let legacy_config_dir = workspace_dir.parent().map(|parent| parent.join(".corvus"));
    if let Some(legacy_dir) = legacy_config_dir {
        if legacy_dir.join("config.toml").exists() {
            return (legacy_dir, workspace_config_dir);
        }

        if workspace_dir
            .file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new("workspace"))
        {
            return (legacy_dir, workspace_config_dir);
        }
    }

    (
        workspace_config_dir.clone(),
        workspace_config_dir.join("workspace"),
    )
}

fn decrypt_optional_secret(
    store: &crate::security::SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    if let Some(raw) = value.clone() {
        if crate::security::SecretStore::is_encrypted(&raw) {
            *value = Some(
                store
                    .decrypt(&raw)
                    .with_context(|| format!("Failed to decrypt {field_name}"))?,
            );
        }
    }
    Ok(())
}

fn encrypt_optional_secret(
    store: &crate::security::SecretStore,
    value: &mut Option<String>,
    field_name: &str,
) -> Result<()> {
    if let Some(raw) = value.clone() {
        if !crate::security::SecretStore::is_encrypted(&raw) {
            *value = Some(
                store
                    .encrypt(&raw)
                    .with_context(|| format!("Failed to encrypt {field_name}"))?,
            );
        }
    }
    Ok(())
}

fn decrypt_required_secret(
    store: &crate::security::SecretStore,
    value: &mut String,
    field_name: &str,
) -> Result<()> {
    if !value.is_empty() && crate::security::SecretStore::is_encrypted(value) {
        *value = store
            .decrypt(value)
            .with_context(|| format!("Failed to decrypt {field_name}"))?;
    }
    Ok(())
}

fn encrypt_required_secret(
    store: &crate::security::SecretStore,
    value: &mut String,
    field_name: &str,
) -> Result<()> {
    if !value.is_empty() && !crate::security::SecretStore::is_encrypted(value) {
        *value = store
            .encrypt(value)
            .with_context(|| format!("Failed to encrypt {field_name}"))?;
    }
    Ok(())
}

fn env_override_optional(var_name: &str, target: &mut Option<String>) {
    if let Ok(raw) = std::env::var(var_name) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            *target = Some(trimmed.to_string());
        }
    }
}

fn env_override_string(primary: &str, secondary: &str, target: &mut Option<String>) {
    if let Ok(value) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *target = Some(trimmed.to_string());
        }
    }
}

fn env_override_string_plain(primary: &str, secondary: &str, target: &mut String) {
    if let Ok(value) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *target = trimmed.to_string();
        }
    }
}

fn env_override_web_search_provider(primary: &str, secondary: &str, target: &mut String) {
    if let Ok(value) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return;
        }

        let normalized = trimmed.to_ascii_lowercase();
        if matches!(normalized.as_str(), "duckduckgo" | "brave") {
            *target = normalized;
        } else {
            tracing::warn!(
                "ignoring unknown web search provider override '{}'; allowed: duckduckgo, brave",
                trimmed
            );
        }
    }
}

fn env_override_port(primary: &str, secondary: &str, target: &mut u16) {
    if let Ok(port_str) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        if let Ok(port) = port_str.parse::<u16>() {
            *target = port;
        }
    }
}

fn env_override_bool(primary: &str, secondary: Option<&str>, target: &mut bool) {
    let fallback = secondary.and_then(|s| std::env::var(s).ok());
    let value = std::env::var(primary).ok().or(fallback);
    if let Some(val) = value {
        *target = val == "1" || val.eq_ignore_ascii_case("true");
    }
}

fn env_override_f64_clamped(primary: &str, min: f64, max: f64, target: &mut f64) {
    if let Ok(temp_str) = std::env::var(primary) {
        if let Ok(temp) = temp_str.parse::<f64>() {
            if (min..=max).contains(&temp) {
                *target = temp;
            }
        }
    }
}

fn env_override_usize_clamped(
    primary: &str,
    secondary: &str,
    min: usize,
    max: usize,
    target: &mut usize,
) {
    if let Ok(value) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        if let Ok(parsed) = value.parse::<usize>() {
            if (min..=max).contains(&parsed) {
                *target = parsed;
            }
        }
    }
}

fn env_override_u64_positive(primary: &str, secondary: &str, target: &mut u64) {
    if let Ok(value) = std::env::var(primary).or_else(|_| std::env::var(secondary)) {
        if let Ok(parsed) = value.parse::<u64>() {
            if parsed > 0 {
                *target = parsed;
            }
        }
    }
}

fn env_override_api_key_with_fallback(primary: &str, fallback: &str, target: &mut Option<String>) {
    if let Ok(key) = std::env::var(primary).or_else(|_| std::env::var(fallback)) {
        if !key.is_empty() {
            *target = Some(key);
        }
    }
}

fn is_valid_mcp_identifier(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("mcp") {
        return false;
    }

    trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Config {
    fn normalize_query_classification_keywords(&mut self) {
        for rule in &mut self.query_classification.rules {
            for keyword in &mut rule.keywords {
                *keyword = keyword.to_ascii_lowercase();
            }
        }
    }

    pub fn load_or_init() -> Result<Self> {
        let (default_corvus_dir, default_workspace_dir) = default_config_and_workspace_dirs()?;

        // Resolution priority:
        // 1. CORVUS_WORKSPACE env override
        // 2. Persisted active workspace marker from onboarding/custom profile
        // 3. Default ~/.corvus layout
        let (corvus_dir, workspace_dir) = match std::env::var("CORVUS_WORKSPACE") {
            Ok(custom_workspace) if !custom_workspace.is_empty() => {
                resolve_config_dir_for_workspace(&PathBuf::from(custom_workspace))
            }
            _ => load_persisted_workspace_dirs(&default_corvus_dir)?
                .unwrap_or((default_corvus_dir, default_workspace_dir)),
        };

        let config_path = corvus_dir.join("config.toml");

        fs::create_dir_all(&corvus_dir).context("Failed to create config directory")?;
        fs::create_dir_all(&workspace_dir).context("Failed to create workspace directory")?;

        if config_path.exists() {
            enforce_secure_config_permissions(&config_path)?;

            let contents =
                fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            // Set computed paths that are skipped during serialization
            config.config_path = config_path.clone();
            config.workspace_dir = workspace_dir;
            let store = crate::security::SecretStore::new(&corvus_dir, config.secrets.encrypt);
            decrypt_optional_secret(&store, &mut config.api_key, "config.api_key")?;
            decrypt_optional_secret(
                &store,
                &mut config.composio.api_key,
                "config.composio.api_key",
            )?;

            decrypt_optional_secret(
                &store,
                &mut config.browser.computer_use.api_key,
                "config.browser.computer_use.api_key",
            )?;

            decrypt_optional_secret(
                &store,
                &mut config.web_search.brave_api_key,
                "config.web_search.brave_api_key",
            )?;
            decrypt_optional_secret(
                &store,
                &mut config.memory.cerebro.auth_token,
                "config.memory.cerebro.auth_token",
            )?;

            for agent in config.agents.values_mut() {
                decrypt_optional_secret(&store, &mut agent.api_key, "config.agents.*.api_key")?;
            }

            for (provider, pool) in &mut config.reliability.account_pools {
                for (idx, account) in pool.accounts.iter_mut().enumerate() {
                    decrypt_required_secret(
                        &store,
                        &mut account.api_key,
                        &format!(
                            "config.reliability.account_pools.{provider}.accounts[{idx}].api_key"
                        ),
                    )?;
                }
            }

            config.apply_env_overrides();
            config.emit_deprecation_warnings();
            config.validate_for_runtime()?;
            Ok(config)
        } else {
            let mut config = Config::default();
            config.config_path = config_path.clone();
            config.workspace_dir = workspace_dir;
            config.save()?;

            config.apply_env_overrides();
            config.validate_for_runtime()?;
            Ok(config)
        }
    }

    /// Apply environment variable overrides to config
    pub fn apply_env_overrides(&mut self) {
        env_override_api_key_with_fallback("CORVUS_API_KEY", "API_KEY", &mut self.api_key);

        env_override_string("CORVUS_PROVIDER", "PROVIDER", &mut self.default_provider);
        env_override_string("CORVUS_MODEL", "MODEL", &mut self.default_model);
        self.apply_regional_api_key_overrides();
        self.apply_memory_backend_override();
        self.apply_workspace_override();
        self.apply_gateway_env_overrides();
        self.apply_web_search_env_overrides();
        self.apply_cerebro_env_overrides();
        self.apply_updates_env_overrides();

        self.normalize_query_classification_keywords();
    }

    fn apply_regional_api_key_overrides(&mut self) {
        self.apply_regional_api_key_override(is_glm_alias, "GLM_API_KEY");
        self.apply_regional_api_key_override(is_zai_alias, "ZAI_API_KEY");
    }

    fn apply_regional_api_key_override(
        &mut self,
        matches_provider: fn(&str) -> bool,
        env_name: &str,
    ) {
        if !self
            .default_provider
            .as_deref()
            .is_some_and(matches_provider)
        {
            return;
        }

        if let Ok(key) = std::env::var(env_name) {
            if !key.is_empty() {
                self.api_key = Some(key);
            }
        }
    }

    fn apply_memory_backend_override(&mut self) {
        if let Ok(backend) =
            std::env::var("CORVUS_MEMORY_BACKEND").or_else(|_| std::env::var("MEMORY_BACKEND"))
        {
            let backend_raw = backend.trim();
            if !backend_raw.is_empty() {
                let backend = backend_raw.to_ascii_lowercase();
                if matches!(backend.as_str(), "sqlite" | "lucid" | "markdown" | "none") {
                    self.memory.backend = backend;
                } else {
                    tracing::error!(
                        "invalid memory backend override '{}'; allowed: sqlite, lucid, markdown, none",
                        backend_raw
                    );
                }
            }
        }
    }

    fn apply_workspace_override(&mut self) {
        if let Ok(workspace) = std::env::var("CORVUS_WORKSPACE") {
            if !workspace.is_empty() {
                let (config_dir, workspace_dir) =
                    resolve_config_dir_for_workspace(&PathBuf::from(workspace));
                self.config_path = config_dir.join("config.toml");
                self.workspace_dir = workspace_dir;
            }
        }
    }

    fn apply_gateway_env_overrides(&mut self) {
        env_override_port("CORVUS_GATEWAY_PORT", "PORT", &mut self.gateway.port);
        env_override_string_plain("CORVUS_GATEWAY_HOST", "HOST", &mut self.gateway.host);
        env_override_bool(
            "CORVUS_ALLOW_PUBLIC_BIND",
            None,
            &mut self.gateway.allow_public_bind,
        );
        env_override_bool(
            "CORVUS_GATEWAY_WEBHOOK_DISPATCHER",
            None,
            &mut self.gateway.webhook_dispatcher_enabled,
        );
        env_override_f64_clamped(
            "CORVUS_TEMPERATURE",
            0.0,
            2.0,
            &mut self.default_temperature,
        );
    }

    fn apply_web_search_env_overrides(&mut self) {
        env_override_bool(
            "CORVUS_WEB_SEARCH_ENABLED",
            Some("WEB_SEARCH_ENABLED"),
            &mut self.web_search.enabled,
        );
        env_override_web_search_provider(
            "CORVUS_WEB_SEARCH_PROVIDER",
            "WEB_SEARCH_PROVIDER",
            &mut self.web_search.provider,
        );

        if let Ok(api_key) =
            std::env::var("CORVUS_BRAVE_API_KEY").or_else(|_| std::env::var("BRAVE_API_KEY"))
        {
            let api_key = api_key.trim();
            if !api_key.is_empty() {
                self.web_search.brave_api_key = Some(api_key.to_string());
            }
        }

        env_override_usize_clamped(
            "CORVUS_WEB_SEARCH_MAX_RESULTS",
            "WEB_SEARCH_MAX_RESULTS",
            1,
            10,
            &mut self.web_search.max_results,
        );
        env_override_u64_positive(
            "CORVUS_WEB_SEARCH_TIMEOUT_SECS",
            "WEB_SEARCH_TIMEOUT_SECS",
            &mut self.web_search.timeout_secs,
        );
    }

    fn apply_cerebro_env_overrides(&mut self) {
        env_override_optional("CORVUS_CEREBRO_ENDPOINT", &mut self.memory.cerebro.endpoint);
        env_override_optional(
            "CORVUS_CEREBRO_AUTH_TOKEN",
            &mut self.memory.cerebro.auth_token,
        );
        env_override_u64_positive(
            "CORVUS_CEREBRO_TIMEOUT_MS",
            "CEREBRO_TIMEOUT_MS",
            &mut self.memory.cerebro.request_timeout_ms,
        );
        env_override_bool(
            "CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK",
            Some("CEREBRO_ALLOW_INSECURE_LOOPBACK"),
            &mut self.memory.cerebro.allow_insecure_loopback,
        );
    }

    fn apply_updates_env_overrides(&mut self) {
        env_override_bool("CORVUS_UPDATES_ENABLED", None, &mut self.updates.enabled);
        env_override_bool(
            "CORVUS_UPDATE_AUTO_INSTALL",
            None,
            &mut self.updates.auto_install_enabled,
        );
        env_override_bool(
            "CORVUS_UPDATE_CHANNEL_VISIBILITY",
            None,
            &mut self.updates.channel_visibility_enabled,
        );
        env_override_bool(
            "CORVUS_UPDATE_CLI_NOTICE",
            None,
            &mut self.updates.cli_startup_notice_enabled,
        );

        if let Ok(raw) = std::env::var("CORVUS_UPDATE_METHOD_OVERRIDE") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                if let Some(method) = normalize_install_method_override(trimmed) {
                    self.updates.install_method_override = Some(method);
                } else {
                    tracing::warn!(
                        "ignoring invalid CORVUS_UPDATE_METHOD_OVERRIDE value: {}",
                        trimmed
                    );
                }
            }
        }

        if let Ok(raw) = std::env::var("CORVUS_UPDATE_RESTART_POLICY") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                if let Some(policy) = normalize_restart_policy(trimmed) {
                    self.updates.restart_policy = policy;
                } else {
                    tracing::warn!(
                        "ignoring invalid CORVUS_UPDATE_RESTART_POLICY value: {}",
                        trimmed
                    );
                }
            }
        }
    }

    fn emit_deprecation_warnings(&self) {
        if let Some(message) = self.autonomy.action_rate_deprecation_warning() {
            tracing::warn!("{message}");
        }
    }

    pub fn validate_for_runtime(&self) -> Result<()> {
        self.validate_agent_profile()?;
        self.validate_mcp_servers()?;
        self.validate_memory_config()?;
        self.validate_cost_config()?;
        self.validate_delegate_overrides()?;
        self.validate_code_session_config()?;
        self.validate_account_pools()?;
        self.validate_skills_config()?;
        self.validate_multimodal_config()?;
        self.validate_audio_config()
    }

    fn validate_cost_config(&self) -> Result<()> {
        if !self.cost.session_limit_usd.is_finite() || self.cost.session_limit_usd < 0.0 {
            anyhow::bail!("cost.session_limit_usd must be a finite, non-negative value");
        }
        if !self.cost.daily_limit_usd.is_finite() || self.cost.daily_limit_usd < 0.0 {
            anyhow::bail!("cost.daily_limit_usd must be a finite, non-negative value");
        }
        if !self.cost.monthly_limit_usd.is_finite() || self.cost.monthly_limit_usd < 0.0 {
            anyhow::bail!("cost.monthly_limit_usd must be a finite, non-negative value");
        }
        for (model, pricing) in &self.cost.prices {
            if !pricing.input.is_finite() || pricing.input < 0.0 {
                anyhow::bail!("cost.prices.{model}.input must be a finite, non-negative value");
            }
            if !pricing.output.is_finite() || pricing.output < 0.0 {
                anyhow::bail!("cost.prices.{model}.output must be a finite, non-negative value");
            }
        }
        Ok(())
    }

    fn validate_agent_profile(&self) -> Result<()> {
        if is_supported_agent_profile(&self.agent.profile) {
            return Ok(());
        }

        anyhow::bail!(
            "unsupported agent.profile '{}'; supported values are: full, code, lite",
            self.agent.profile
        );
    }

    fn validate_mcp_servers(&self) -> Result<()> {
        if !self.mcp.enabled {
            return Ok(());
        }

        for (idx, server) in self.mcp.servers.iter().enumerate() {
            Self::validate_mcp_server(server, idx)?;
        }

        Ok(())
    }

    fn validate_memory_config(&self) -> Result<()> {
        match self.memory.backend.as_str() {
            "sqlite" | "lucid" | "markdown" | "none" => {}
            "surreal" | "surreal-graphs" => {
                anyhow::bail!(
                    "memory.backend '{}' is not supported; SurrealDB backend has been removed; valid options are: sqlite, lucid, markdown, none. For Cerebro, keep memory.backend to a supported local backend and configure [memory.cerebro] (see https://github.com/dallay/corvus/blob/main/clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md).",
                    self.memory.backend
                );
            }
            _ => {
                anyhow::bail!(
                    "memory.backend '{}' is not supported; valid options are: sqlite, lucid, markdown, none",
                    self.memory.backend
                );
            }
        }

        if let Some(endpoint) = self.memory.cerebro.endpoint.as_deref() {
            Self::validate_cerebro_endpoint(endpoint, &self.memory.cerebro)?;
        }

        Ok(())
    }

    fn validate_delegate_overrides(&self) -> Result<()> {
        for (name, agent) in &self.agents {
            let base = format!("agents.{name}");
            if let Some(max_iterations) = agent.max_iterations {
                if max_iterations == 0 {
                    anyhow::bail!("{base}.max_iterations must be greater than zero");
                }
            }
            if let Some(timeout_ms) = agent.timeout_ms {
                if timeout_ms == 0 {
                    anyhow::bail!("{base}.timeout_ms must be greater than zero");
                }
            }
        }

        Ok(())
    }

    fn validate_account_pools(&self) -> Result<()> {
        for (provider, pool) in &self.reliability.account_pools {
            let provider = provider.trim();
            if provider.is_empty() {
                anyhow::bail!("reliability.account_pools provider name must be non-empty");
            }
            if pool.accounts.is_empty() {
                anyhow::bail!("reliability.account_pools.{provider}.accounts must be non-empty");
            }

            let mut seen_ids = std::collections::HashSet::new();
            for (idx, account) in pool.accounts.iter().enumerate() {
                let base = format!("reliability.account_pools.{provider}.accounts[{idx}]");
                Self::validate_single_pool_account(account, &base, &mut seen_ids)?;
            }
        }

        Ok(())
    }

    fn validate_single_pool_account(
        account: &ProviderAccountConfig,
        base: &str,
        seen_ids: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if account.id.trim().is_empty() {
            anyhow::bail!("{base}.id must be non-empty");
        }
        if account.api_key.trim().is_empty() {
            anyhow::bail!("{base}.api_key must be non-empty");
        }
        if account.weight == 0 {
            anyhow::bail!("{base}.weight must be greater than zero");
        }
        if !seen_ids.insert(account.id.trim().to_string()) {
            anyhow::bail!("{base}.id must be unique within pool");
        }
        Ok(())
    }

    fn validate_skills_config(&self) -> Result<()> {
        if let Some(ref url) = self.skills.catalog_repo_url {
            let parsed = Url::parse(url)
                .map_err(|e| anyhow::anyhow!("invalid catalog_repo_url '{}': {}", url, e))?;
            if parsed.scheme() != "https" {
                anyhow::bail!("catalog_repo_url must use https:// scheme, got '{}'", url,);
            }
            let host = parsed.host_str().unwrap_or("");
            if host.is_empty() || Self::is_loopback_host(host) {
                anyhow::bail!("catalog_repo_url must not point to localhost: '{}'", url,);
            }
        }
        if self.skills.catalog_cache_ttl_hours == Some(0) {
            anyhow::bail!("catalog_cache_ttl_hours must be > 0 (got 0)");
        }
        Ok(())
    }

    fn validate_multimodal_config(&self) -> Result<()> {
        let mm = &self.multimodal;

        // Validate max_image_bytes bounds regardless of enabled state
        if let Some(max_bytes) = mm.max_image_bytes {
            if max_bytes == 0 {
                anyhow::bail!("multimodal.max_image_bytes must be greater than 0");
            }
            if max_bytes > crate::channels::media::MAX_IMAGE_BYTES_CEILING {
                anyhow::bail!(
                    "multimodal.max_image_bytes={} exceeds the 50 MiB ceiling ({})",
                    max_bytes,
                    crate::channels::media::MAX_IMAGE_BYTES_CEILING,
                );
            }
        }

        if !mm.enabled {
            return Ok(());
        }
        let hint = match mm.vision_model_hint {
            Some(ref h) => h,
            None => {
                anyhow::bail!(
                    "multimodal.enabled=true requires multimodal.vision_model_hint to be set"
                );
            }
        };
        // Cross-reference: a matching model_route must exist with
        // allow_image_input enabled.
        let has_image_route = self
            .model_routes
            .iter()
            .any(|r| r.hint == *hint && r.allow_image_input);
        if !has_image_route {
            anyhow::bail!(
                "multimodal.vision_model_hint='{}' does not match any \
                 [[model_routes]] entry with allow_image_input=true",
                hint,
            );
        }
        if mm.allowed_channels.is_empty() {
            anyhow::bail!(
                "multimodal.enabled=true requires multimodal.allowed_channels to be non-empty"
            );
        }
        for ch in &mm.allowed_channels {
            if !MVP_VALID_MULTIMODAL_CHANNELS.contains(&ch.as_str()) {
                tracing::warn!(
                    "multimodal.allowed_channels contains '{}' which is not a supported MVP channel \
                     (telegram, whatsapp, discord) — it will be fail-closed at runtime",
                    ch,
                );
            }
        }

        // Log effective max_image_bytes
        let effective = mm
            .max_image_bytes
            .unwrap_or(crate::channels::media::MAX_IMAGE_BYTES);
        if mm.max_image_bytes.is_some() {
            tracing::info!("Multimodal enabled: max_image_bytes={effective} (config override)");
        } else {
            tracing::info!("Multimodal enabled: max_image_bytes={effective} (default)");
        }

        Ok(())
    }

    fn validate_audio_config(&self) -> Result<()> {
        let ac = &self.audio;

        // Validate bounds regardless of enabled state
        if ac.max_audio_bytes == 0 {
            anyhow::bail!("audio.max_audio_bytes must be greater than 0");
        }
        if ac.max_audio_bytes > MAX_AUDIO_BYTES_CEILING {
            anyhow::bail!(
                "audio.max_audio_bytes={} exceeds the 100 MiB ceiling ({})",
                ac.max_audio_bytes,
                MAX_AUDIO_BYTES_CEILING,
            );
        }
        if ac.max_audio_duration_secs == 0 {
            anyhow::bail!("audio.max_audio_duration_secs must be greater than 0");
        }
        if ac.max_audio_duration_secs > MAX_AUDIO_DURATION_SECS_CEILING {
            anyhow::bail!(
                "audio.max_audio_duration_secs={} exceeds the 1 hour ceiling ({})",
                ac.max_audio_duration_secs,
                MAX_AUDIO_DURATION_SECS_CEILING,
            );
        }

        if ac.max_concurrent_transcriptions == 0 {
            anyhow::bail!("audio.max_concurrent_transcriptions must be greater than 0");
        }
        if ac.transcription_timeout_secs == 0 {
            anyhow::bail!("audio.transcription_timeout_secs must be greater than 0");
        }

        if !ac.enabled {
            return Ok(());
        }

        if ac.allowed_channels.is_empty() {
            anyhow::bail!("audio.allowed_channels must be non-empty when audio is enabled");
        }

        for ch in &ac.allowed_channels {
            if !VALID_AUDIO_CHANNELS.contains(&ch.as_str()) {
                tracing::warn!(
                    "audio.allowed_channels contains '{}' which is not a recognised audio \
                     channel (telegram, gateway, cli) — it will be fail-closed at runtime",
                    ch,
                );
            }
        }

        tracing::info!(
            "Audio enabled: allowed_channels={:?}, max_bytes={}, max_duration={}s, \
             model={}, language={}",
            ac.allowed_channels,
            ac.max_audio_bytes,
            ac.max_audio_duration_secs,
            ac.transcription_model,
            ac.transcription_language,
        );

        Ok(())
    }

    fn validate_code_session_config(&self) -> Result<()> {
        let code_session = &self.agent.code_session;
        if code_session.max_iterations == 0 {
            anyhow::bail!("agent.code_session.max_iterations must be greater than zero");
        }
        if code_session.timeout_ms == 0 {
            anyhow::bail!("agent.code_session.timeout_ms must be greater than zero");
        }
        if code_session.enabled && code_session.validation_commands.is_empty() {
            anyhow::bail!(
                "agent.code_session.validation_commands must be non-empty when code_session is enabled"
            );
        }

        for (idx, validation) in code_session.validation_commands.iter().enumerate() {
            if validation.command.trim().is_empty() {
                anyhow::bail!(
                    "agent.code_session.validation_commands[{idx}].command must be non-empty"
                );
            }
            if validation.timeout_ms == 0 {
                anyhow::bail!(
                    "agent.code_session.validation_commands[{idx}].timeout_ms must be greater than zero"
                );
            }
        }

        Ok(())
    }

    fn validate_mcp_server(server: &McpServerConfig, idx: usize) -> Result<()> {
        let base = format!("mcp.servers[{idx}]");

        if !is_valid_mcp_identifier(&server.name) {
            anyhow::bail!(
            "{base}.name must be a non-empty identifier using [a-zA-Z0-9_-] and cannot be 'mcp'"
        );
        }

        Self::validate_mcp_command(server.command.as_str(), &base)?;
        Self::validate_non_zero(server.startup_timeout_ms, &base, "startup_timeout_ms")?;
        Self::validate_non_zero(server.call_timeout_ms, &base, "call_timeout_ms")?;
        Self::validate_non_zero(server.output_limit_bytes, &base, "output_limit_bytes")?;

        if server.output_limit_bytes > 10 * 1024 * 1024 {
            anyhow::bail!("{base}.output_limit_bytes exceeds maximum allowed (10MB)");
        }

        Self::validate_mcp_capabilities(&server.capabilities, &base)?;
        Self::validate_mcp_capability_limits(server, &base)?;

        Self::validate_mcp_env(&server.env, &base)
    }

    fn validate_mcp_capabilities(capabilities: &[String], base: &str) -> Result<()> {
        const VALID_CAPABILITIES: &[&str] = &["tools", "resources", "prompts"];

        if capabilities.is_empty() {
            anyhow::bail!("{base}.capabilities must contain at least one capability type");
        }

        let mut seen = std::collections::HashSet::new();
        for cap in capabilities {
            if !VALID_CAPABILITIES.contains(&cap.as_str()) {
                anyhow::bail!(
                    "{base}.capabilities contains unrecognized capability type '{cap}'; \
                     valid values are: tools, resources, prompts"
                );
            }
            if !seen.insert(cap.as_str()) {
                anyhow::bail!("{base}.capabilities contains duplicate entry '{cap}'");
            }
        }

        Ok(())
    }

    fn validate_mcp_capability_limits(server: &McpServerConfig, base: &str) -> Result<()> {
        let max_limit = 10 * 1024 * 1024; // 10MB

        if let Some(limit) = server.resource_output_limit_bytes {
            if limit == 0 {
                anyhow::bail!("{base}.resource_output_limit_bytes must be greater than zero");
            }
            if limit > max_limit {
                anyhow::bail!("{base}.resource_output_limit_bytes exceeds maximum allowed (10MB)");
            }
        }

        if let Some(limit) = server.prompt_output_limit_bytes {
            if limit == 0 {
                anyhow::bail!("{base}.prompt_output_limit_bytes must be greater than zero");
            }
            if limit > max_limit {
                anyhow::bail!("{base}.prompt_output_limit_bytes exceeds maximum allowed (10MB)");
            }
        }

        Ok(())
    }

    fn validate_mcp_command(command: &str, base: &str) -> Result<()> {
        if command.trim().is_empty() {
            anyhow::bail!("{base}.command must be non-empty");
        }

        if command.contains('\0') {
            anyhow::bail!("{base}.command contains an invalid value");
        }

        Ok(())
    }

    fn validate_non_zero<T>(value: T, base: &str, field: &str) -> Result<()>
    where
        T: PartialEq + Default,
    {
        if value == T::default() {
            anyhow::bail!("{base}.{field} must be greater than zero");
        }

        Ok(())
    }

    fn validate_mcp_env(env: &BTreeMap<String, String>, base: &str) -> Result<()> {
        for (key, value) in env {
            if key.contains('\0') {
                anyhow::bail!("{base}.env contains an invalid key");
            }

            if value.contains('\0') {
                anyhow::bail!("{base}.env contains an invalid value");
            }
        }

        Ok(())
    }

    fn validate_cerebro_endpoint(endpoint: &str, config: &MemoryCerebroConfig) -> Result<()> {
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            anyhow::bail!("memory.cerebro.endpoint must be non-empty when configured");
        }

        if config.request_timeout_ms == 0 {
            anyhow::bail!("memory.cerebro.request_timeout_ms must be greater than zero");
        }

        let parsed = Url::parse(endpoint)
            .with_context(|| format!("memory.cerebro.endpoint is not a valid URL: {endpoint}"))?;

        let scheme = parsed.scheme();
        let is_insecure = matches!(scheme, "http" | "ws");
        let is_secure = matches!(scheme, "https" | "wss");

        if !is_insecure && !is_secure {
            anyhow::bail!("memory.cerebro.endpoint must use http, https, ws, or wss transport");
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("memory.cerebro.endpoint must include a host"))?;

        if is_insecure && !config.allow_insecure_loopback {
            anyhow::bail!(
                "memory.cerebro.endpoint requires https/wss or allow_insecure_loopback=true"
            );
        }

        if is_insecure && config.allow_insecure_loopback && !Self::is_loopback_host(host) {
            anyhow::bail!(
                "memory.cerebro.endpoint allows insecure transport only for loopback addresses"
            );
        }

        if config
            .auth_token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .is_none()
        {
            anyhow::bail!("memory.cerebro.auth_token is required when endpoint is configured");
        }

        Ok(())
    }

    fn is_loopback_host(host: &str) -> bool {
        if host.eq_ignore_ascii_case("localhost") {
            return true;
        }

        host.parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    }

    pub fn save(&self) -> Result<()> {
        // Encrypt secrets before serialization
        let mut config_to_save = self.clone();
        let corvus_dir = self
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        let store = crate::security::SecretStore::new(corvus_dir, self.secrets.encrypt);

        encrypt_optional_secret(&store, &mut config_to_save.api_key, "config.api_key")?;
        encrypt_optional_secret(
            &store,
            &mut config_to_save.composio.api_key,
            "config.composio.api_key",
        )?;

        encrypt_optional_secret(
            &store,
            &mut config_to_save.browser.computer_use.api_key,
            "config.browser.computer_use.api_key",
        )?;

        encrypt_optional_secret(
            &store,
            &mut config_to_save.web_search.brave_api_key,
            "config.web_search.brave_api_key",
        )?;
        encrypt_optional_secret(
            &store,
            &mut config_to_save.memory.cerebro.auth_token,
            "config.memory.cerebro.auth_token",
        )?;

        for agent in config_to_save.agents.values_mut() {
            encrypt_optional_secret(&store, &mut agent.api_key, "config.agents.*.api_key")?;
        }

        for (provider, pool) in &mut config_to_save.reliability.account_pools {
            for (idx, account) in pool.accounts.iter_mut().enumerate() {
                encrypt_required_secret(
                    &store,
                    &mut account.api_key,
                    &format!("config.reliability.account_pools.{provider}.accounts[{idx}].api_key"),
                )?;
            }
        }

        let toml_str =
            toml::to_string_pretty(&config_to_save).context("Failed to serialize config")?;

        let parent_dir = self
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                parent_dir.display()
            )
        })?;

        let file_name = self
            .config_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("config.toml");
        let temp_path = parent_dir.join(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()));
        let backup_path = parent_dir.join(format!("{file_name}.bak"));

        let mut open_options = OpenOptions::new();
        open_options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(0o600);
        }
        let mut temp_file = open_options.open(&temp_path).with_context(|| {
            format!(
                "Failed to create temporary config file: {}",
                temp_path.display()
            )
        })?;
        temp_file
            .write_all(toml_str.as_bytes())
            .context("Failed to write temporary config contents")?;
        temp_file
            .sync_all()
            .context("Failed to fsync temporary config file")?;
        drop(temp_file);

        let had_existing_config = self.config_path.exists();
        if had_existing_config {
            fs::copy(&self.config_path, &backup_path).with_context(|| {
                format!(
                    "Failed to create config backup before atomic replace: {}",
                    backup_path.display()
                )
            })?;
            enforce_secure_config_permissions(&backup_path)?;
        }

        if let Err(e) = fs::rename(&temp_path, &self.config_path) {
            let _ = fs::remove_file(&temp_path);
            if had_existing_config && backup_path.exists() {
                let _ = fs::copy(&backup_path, &self.config_path);
            }
            anyhow::bail!("Failed to atomically replace config file: {e}");
        }

        enforce_secure_config_permissions(&self.config_path)?;
        sync_directory(parent_dir)?;

        if had_existing_config {
            let _ = fs::remove_file(&backup_path);
        }

        Ok(())
    }
}

fn enforce_secure_config_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let meta = fs::metadata(path)
            .with_context(|| format!("Failed to read config file metadata: {}", path.display()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode != 0o600 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).with_context(|| {
                format!(
                    "Failed to restrict config file permissions to 600: {}",
                    path.display()
                )
            })?;
            tracing::warn!(
                "Config file {:?} had insecure permissions (mode {:o}); restricted to 600",
                path,
                mode,
            );
        }
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    let dir = File::open(path)
        .with_context(|| format!("Failed to open directory for fsync: {}", path.display()))?;
    dir.sync_all()
        .with_context(|| format!("Failed to fsync directory metadata: {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        acquire_gateway_webhook_dispatcher_lock_blocking, GatewayWebhookDispatcherEnvGuard,
    };
    use std::path::PathBuf;

    // ── Defaults ─────────────────────────────────────────────

    #[test]
    fn config_default_has_sane_values() {
        let c = Config::default();
        assert_eq!(c.default_provider.as_deref(), Some("openrouter"));
        assert!(c.default_model.as_deref().unwrap().contains("claude"));
        assert!((c.default_temperature - 0.7).abs() < f64::EPSILON);
        assert!(c.api_key.is_none());
        assert!(c.workspace_dir.to_string_lossy().contains("workspace"));
        assert!(c.config_path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    fn observability_config_default() {
        let o = ObservabilityConfig::default();
        assert_eq!(o.backend, "none");
    }

    #[test]
    fn autonomy_config_default() {
        let a = AutonomyConfig::default();
        assert_eq!(a.level, AutonomyLevel::Supervised);
        assert!(a.workspace_only);
        assert!(a.allowed_commands.contains(&"git".to_string()));
        assert!(a.allowed_commands.contains(&"cargo".to_string()));
        assert!(a.forbidden_paths.contains(&"/etc".to_string()));
        assert_eq!(a.max_actions_per_hour, 20);
        assert!(a.require_approval_for_medium_risk);
        assert!(a.block_high_risk_commands);
    }

    #[test]
    fn autonomy_config_normalizes_deprecated_action_rate_alias() {
        let parsed: AutonomyConfig = toml::from_str(
            r#"
level = "supervised"
workspace_only = true
allowed_commands = ["git"]
forbidden_paths = ["/etc"]
max_cost_per_day_cents = 42
require_approval_for_medium_risk = true
block_high_risk_commands = true
auto_approve = []
always_ask = []
"#,
        )
        .unwrap();

        assert_eq!(parsed.max_actions_per_hour, 42);
        assert_eq!(
            parsed.deprecated_fields(),
            &["autonomy.max_cost_per_day_cents".to_string()]
        );
    }

    #[test]
    fn autonomy_config_serializes_canonical_action_rate_field_only() {
        let mut config = AutonomyConfig::default();
        config.max_actions_per_hour = 33;

        let toml = toml::to_string(&config).unwrap();

        assert!(toml.contains("max_actions_per_hour = 33"));
        assert!(!toml.contains("max_cost_per_day_cents"));
    }

    #[test]
    fn runtime_config_default() {
        let r = RuntimeConfig::default();
        assert_eq!(r.kind, "native");
        assert_eq!(r.docker.image, "alpine:3.20");
        assert_eq!(r.docker.network, "none");
        assert_eq!(r.docker.memory_limit_mb, Some(512));
        assert_eq!(r.docker.cpu_limit, Some(1.0));
        assert!(r.docker.read_only_rootfs);
        assert!(r.docker.mount_workspace);
    }

    #[test]
    fn mission_config_defaults_fail_closed() {
        let mission = MissionConfig::default();
        assert!(!mission.enabled);
        assert_eq!(mission.max_runtime_ms, 300_000);
        assert_eq!(mission.max_steps, 10);
        assert_eq!(mission.max_estimated_cost_cents, 100);
    }

    #[test]
    fn config_defaults_mission_when_section_missing() {
        let toml_str = r#"
default_temperature = 0.7
"#;

        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(!parsed.mission.enabled);
        assert_eq!(parsed.mission.max_runtime_ms, 300_000);
        assert_eq!(parsed.mission.max_steps, 10);
        assert_eq!(parsed.mission.max_estimated_cost_cents, 100);
    }

    #[test]
    fn heartbeat_config_default() {
        let h = HeartbeatConfig::default();
        assert!(!h.enabled);
        assert_eq!(h.interval_minutes, 30);
    }

    #[test]
    fn cron_config_default() {
        let c = CronConfig::default();
        assert!(c.enabled);
        assert_eq!(c.max_run_history, 50);
    }

    #[test]
    fn cron_config_serde_roundtrip() {
        let c = CronConfig {
            enabled: false,
            max_run_history: 100,
        };
        let json = serde_json::to_string(&c).unwrap();
        let parsed: CronConfig = serde_json::from_str(&json).unwrap();
        assert!(!parsed.enabled);
        assert_eq!(parsed.max_run_history, 100);
    }

    #[test]
    fn config_defaults_cron_when_section_missing() {
        let toml_str = r#"
default_temperature = 0.7
"#;

        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(parsed.cron.enabled);
        assert_eq!(parsed.cron.max_run_history, 50);
    }

    #[test]
    fn memory_config_default_hygiene_settings() {
        let m = MemoryConfig::default();
        assert_eq!(m.backend, "sqlite");
        assert!(m.auto_save);
        assert!(m.hygiene_enabled);
        assert_eq!(m.archive_after_days, 7);
        assert_eq!(m.purge_after_days, 30);
        assert_eq!(m.conversation_retention_days, 30);
        assert!(m.sqlite_open_timeout_secs.is_none());
        assert!(m.cerebro.endpoint.is_none());
        assert_eq!(m.cerebro.request_timeout_ms, 30_000);
        assert!(!m.cerebro.allow_insecure_loopback);
    }

    #[test]
    fn cerebro_memory_config_debug_redacts_sensitive_fields() {
        let cfg = MemoryCerebroConfig {
            endpoint: Some("https://cerebro.example.com/mcp".into()),
            auth_token: Some("secret-token".into()),
            request_timeout_ms: 30_000,
            allow_insecure_loopback: false,
        };
        let rendered = format!("{cfg:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("secret-token"));
    }

    #[test]
    fn channels_config_default() {
        let c = ChannelsConfig::default();
        assert!(c.cli);
        assert!(c.telegram.is_none());
        assert!(c.discord.is_none());
    }

    #[test]
    fn updates_config_defaults_are_safe_and_enabled() {
        let updates = UpdateConfig::default();
        assert!(updates.enabled);
        assert!(!updates.auto_install_enabled);
        assert!(updates.channel_visibility_enabled);
        assert!(updates.cli_startup_notice_enabled);
        assert!(updates.install_method_override.is_none());
        assert_eq!(updates.restart_policy, "prompt");
        assert_eq!(updates.history_max_entries, 200);
        assert_eq!(updates.check_interval_minutes, 30);
        assert_eq!(updates.confirmation_ttl_minutes, 30);
        assert!(updates.notify_destinations.is_empty());
    }

    // ── Serde round-trip ─────────────────────────────────────

    #[test]
    fn config_toml_roundtrip() {
        let config = Config {
            workspace_dir: PathBuf::from("/tmp/test/workspace"),
            config_path: PathBuf::from("/tmp/test/config.toml"),
            api_key: Some("sk-test-key".into()),
            api_url: None,
            default_provider: Some("openrouter".into()),
            default_model: Some("gpt-4o".into()),
            default_temperature: 0.5,
            observability: ObservabilityConfig {
                backend: "log".into(),
                ..ObservabilityConfig::default()
            },
            autonomy: AutonomyConfig {
                level: AutonomyLevel::Full,
                workspace_only: false,
                allowed_commands: vec!["docker".into()],
                forbidden_paths: vec!["/secret".into()],
                max_actions_per_hour: 50,
                require_approval_for_medium_risk: false,
                block_high_risk_commands: true,
                auto_approve: vec!["file_read".into()],
                always_ask: vec![],
                deprecated_fields: vec![],
            },
            security: SecurityConfig::default(),
            runtime: RuntimeConfig {
                kind: "docker".into(),
                ..RuntimeConfig::default()
            },
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            mission: MissionConfig::default(),
            model_routes: Vec::new(),
            query_classification: QueryClassificationConfig::default(),
            heartbeat: HeartbeatConfig {
                enabled: true,
                interval_minutes: 15,
            },
            cron: CronConfig::default(),
            channels_config: ChannelsConfig {
                cli: true,
                telegram: Some(TelegramConfig {
                    bot_token: "123:ABC".into(),
                    allowed_users: vec!["user1".into()],
                    stream_mode: StreamMode::default(),
                    draft_update_interval_ms: default_draft_update_interval_ms(),
                }),
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
            },
            updates: UpdateConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            web_search: WebSearchConfig::default(),
            mcp: McpConfig::default(),
            agent: AgentConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            skills: SkillsConfig::default(),
            multimodal: MultimodalConfig::default(),
            audio: AudioConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.default_provider, config.default_provider);
        assert_eq!(parsed.default_model, config.default_model);
        assert!((parsed.default_temperature - config.default_temperature).abs() < f64::EPSILON);
        assert_eq!(parsed.observability.backend, "log");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Full);
        assert!(!parsed.autonomy.workspace_only);
        assert_eq!(parsed.runtime.kind, "docker");
        assert!(parsed.heartbeat.enabled);
        assert_eq!(parsed.heartbeat.interval_minutes, 15);
        assert!(parsed.channels_config.telegram.is_some());
        assert_eq!(
            parsed.channels_config.telegram.unwrap().bot_token,
            "123:ABC"
        );
    }

    #[test]
    fn config_minimal_toml_uses_defaults() {
        let minimal = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(parsed.api_key.is_none());
        assert!(parsed.default_provider.is_none());
        assert_eq!(parsed.observability.backend, "none");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Supervised);
        assert_eq!(parsed.runtime.kind, "native");
        assert!(!parsed.heartbeat.enabled);
        assert!(parsed.channels_config.cli);
        assert!(parsed.memory.hygiene_enabled);
        assert_eq!(parsed.memory.archive_after_days, 7);
        assert_eq!(parsed.memory.purge_after_days, 30);
        assert_eq!(parsed.memory.conversation_retention_days, 30);
    }

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert!(!cfg.compact_context);
        assert_eq!(cfg.profile, "full");
        assert_eq!(cfg.max_tool_iterations, 10);
        assert_eq!(cfg.max_history_messages, 50);
        assert!(!cfg.parallel_tools);
        assert_eq!(cfg.tool_dispatcher, "auto");
    }

    #[test]
    fn agent_config_deserializes() {
        let raw = r#"
default_temperature = 0.7
[agent]
compact_context = true
profile = "code"
max_tool_iterations = 20
max_history_messages = 80
parallel_tools = true
tool_dispatcher = "xml"
"#;
        let parsed: Config = toml::from_str(raw).unwrap();
        assert!(parsed.agent.compact_context);
        assert_eq!(parsed.agent.profile, "code");
        assert_eq!(parsed.agent.max_tool_iterations, 20);
        assert_eq!(parsed.agent.max_history_messages, 80);
        assert!(parsed.agent.parallel_tools);
        assert_eq!(parsed.agent.tool_dispatcher, "xml");
    }

    fn sample_delegate_config() -> DelegateAgentConfig {
        DelegateAgentConfig {
            provider: "openrouter".into(),
            model: "gpt-4o".into(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            max_depth: 1,
            execution_mode: DelegateExecutionMode::default(),
            max_iterations: None,
            timeout_ms: None,
        }
    }

    fn sample_pool_account(id: &str, api_key: &str, weight: u32) -> ProviderAccountConfig {
        ProviderAccountConfig {
            id: id.to_string(),
            api_key: api_key.to_string(),
            api_url: None,
            weight,
            enabled: true,
        }
    }

    #[test]
    fn validate_for_runtime_rejects_unknown_agent_profile() {
        let mut config = Config::default();
        config.agent.profile = "unknown".to_string();

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err.to_string().contains("unsupported agent.profile"));
    }

    #[test]
    fn validate_for_runtime_rejects_delegate_max_iterations_zero() {
        let mut config = Config::default();
        let mut delegate = sample_delegate_config();
        delegate.max_iterations = Some(0);
        config.agents.insert("child".into(), delegate);

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("agents.child.max_iterations must be greater than zero"));
    }

    #[test]
    fn validate_for_runtime_rejects_delegate_timeout_zero() {
        let mut config = Config::default();
        let mut delegate = sample_delegate_config();
        delegate.timeout_ms = Some(0);
        config.agents.insert("child".into(), delegate);

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("agents.child.timeout_ms must be greater than zero"));
    }

    #[test]
    fn validate_for_runtime_rejects_code_session_without_validations() {
        let mut config = Config::default();
        config.agent.code_session.enabled = true;

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("agent.code_session.validation_commands must be non-empty"));
    }

    #[test]
    fn validate_for_runtime_rejects_empty_validation_command() {
        let mut config = Config::default();
        config.agent.code_session.validation_commands = vec![ValidationCommandConfig {
            command: "   ".into(),
            required: true,
            timeout_ms: 1_000,
        }];

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("agent.code_session.validation_commands[0].command must be non-empty"));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_account_missing_id() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_pool_account("", "sk-test", 1)],
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("reliability.account_pools.openrouter.accounts[0].id must be non-empty"));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_provider_name_empty() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "  ".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_pool_account("acct-1", "sk-test", 1)],
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("reliability.account_pools provider name must be non-empty"));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_account_missing_api_key() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_pool_account("acct-1", "  ", 1)],
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err.to_string().contains(
            "reliability.account_pools.openrouter.accounts[0].api_key must be non-empty"
        ));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_with_no_accounts() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: Vec::new(),
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("reliability.account_pools.openrouter.accounts must be non-empty"));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_account_duplicate_ids() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![
                    sample_pool_account("acct-1", "sk-test-1", 1),
                    sample_pool_account("acct-1", "sk-test-2", 1),
                ],
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("reliability.account_pools.openrouter.accounts[1].id must be unique"));
    }

    #[test]
    fn validate_for_runtime_rejects_pool_account_zero_weight() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_pool_account("acct-1", "sk-test-1", 0)],
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err.to_string().contains(
            "reliability.account_pools.openrouter.accounts[0].weight must be greater than zero"
        ));
    }

    #[test]
    fn validate_for_runtime_accepts_valid_pool_config() {
        let mut config = Config::default();
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::WeightedRoundRobin,
                accounts: vec![
                    sample_pool_account("acct-1", "sk-test-1", 1),
                    sample_pool_account("acct-2", "sk-test-2", 2),
                ],
            },
        );

        assert!(config.validate_for_runtime().is_ok());
    }

    #[test]
    fn validate_for_runtime_rejects_negative_cost_limits() {
        let mut config = Config::default();
        config.cost.session_limit_usd = -1.0;

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("cost.session_limit_usd must be a finite, non-negative value"));
    }

    #[test]
    fn validate_for_runtime_rejects_negative_model_pricing() {
        let mut config = Config::default();
        config.cost.prices.insert(
            "bad-model".to_string(),
            ModelPricing {
                input: -0.1,
                output: 1.0,
            },
        );

        let err = config.validate_for_runtime().unwrap_err();
        assert!(err
            .to_string()
            .contains("cost.prices.bad-model.input must be a finite, non-negative value"));
    }

    #[test]
    fn config_save_and_load_tmpdir() {
        let dir = std::env::temp_dir().join("corvus_test_config");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let config = Config {
            workspace_dir: dir.join("workspace"),
            config_path: config_path.clone(),
            api_key: Some("sk-roundtrip".into()),
            api_url: None,
            default_provider: Some("openrouter".into()),
            default_model: Some("test-model".into()),
            default_temperature: 0.9,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            security: SecurityConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            mission: MissionConfig::default(),
            model_routes: Vec::new(),
            query_classification: QueryClassificationConfig::default(),
            heartbeat: HeartbeatConfig::default(),
            cron: CronConfig::default(),
            channels_config: ChannelsConfig::default(),
            updates: UpdateConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            composio: ComposioConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            web_search: WebSearchConfig::default(),
            mcp: McpConfig::default(),
            agent: AgentConfig::default(),
            identity: IdentityConfig::default(),
            cost: CostConfig::default(),
            peripherals: PeripheralsConfig::default(),
            agents: HashMap::new(),
            hardware: HardwareConfig::default(),
            skills: SkillsConfig::default(),
            multimodal: MultimodalConfig::default(),
            audio: AudioConfig::default(),
        };

        config.save().unwrap();
        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = toml::from_str(&contents).unwrap();
        assert!(loaded
            .api_key
            .as_deref()
            .is_some_and(crate::security::SecretStore::is_encrypted));
        let store = crate::security::SecretStore::new(&dir, true);
        let decrypted = store.decrypt(loaded.api_key.as_deref().unwrap()).unwrap();
        assert_eq!(decrypted, "sk-roundtrip");
        assert_eq!(loaded.default_model.as_deref(), Some("test-model"));
        assert!((loaded.default_temperature - 0.9).abs() < f64::EPSILON);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_save_encrypts_nested_credentials() {
        let dir = std::env::temp_dir().join(format!(
            "corvus_test_nested_credentials_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();

        let mut config = Config::default();
        config.workspace_dir = dir.join("workspace");
        config.config_path = dir.join("config.toml");
        config.api_key = Some("root-credential".into());
        config.composio.api_key = Some("composio-credential".into());
        config.browser.computer_use.api_key = Some("browser-credential".into());
        config.web_search.brave_api_key = Some("brave-credential".into());
        config.memory.cerebro.endpoint = Some("https://cerebro.example.com/mcp".into());
        config.memory.cerebro.auth_token = Some("test-token".into());

        config.agents.insert(
            "worker".into(),
            DelegateAgentConfig {
                provider: "openrouter".into(),
                model: "model-test".into(),
                system_prompt: None,
                api_key: Some("agent-credential".into()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );

        config.save().unwrap();

        let contents = fs::read_to_string(config.config_path.clone()).unwrap();
        let stored: Config = toml::from_str(&contents).unwrap();
        let store = crate::security::SecretStore::new(&dir, true);

        let root_encrypted = stored.api_key.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(root_encrypted));
        assert_eq!(store.decrypt(root_encrypted).unwrap(), "root-credential");

        let composio_encrypted = stored.composio.api_key.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(
            composio_encrypted
        ));
        assert_eq!(
            store.decrypt(composio_encrypted).unwrap(),
            "composio-credential"
        );

        let browser_encrypted = stored.browser.computer_use.api_key.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(
            browser_encrypted
        ));
        assert_eq!(
            store.decrypt(browser_encrypted).unwrap(),
            "browser-credential"
        );

        let web_search_encrypted = stored.web_search.brave_api_key.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(
            web_search_encrypted
        ));
        assert_eq!(
            store.decrypt(web_search_encrypted).unwrap(),
            "brave-credential"
        );

        let cerebro_endpoint = stored.memory.cerebro.endpoint.as_deref().unwrap();
        assert!(!crate::security::SecretStore::is_encrypted(
            cerebro_endpoint
        ));
        assert_eq!(cerebro_endpoint, "https://cerebro.example.com/mcp");

        let cerebro_token_encrypted = stored.memory.cerebro.auth_token.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(
            cerebro_token_encrypted
        ));
        assert_eq!(
            store.decrypt(cerebro_token_encrypted).unwrap(),
            "test-token"
        );

        let worker = stored.agents.get("worker").unwrap();
        let worker_encrypted = worker.api_key.as_deref().unwrap();
        assert!(crate::security::SecretStore::is_encrypted(worker_encrypted));
        assert_eq!(store.decrypt(worker_encrypted).unwrap(), "agent-credential");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn provider_account_debug_redacts_api_key() {
        let account = ProviderAccountConfig {
            id: "acct-1".into(),
            api_key: "sk-secret".into(),
            api_url: None,
            weight: 1,
            enabled: true,
        };

        let rendered = format!("{account:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("sk-secret"));
    }

    #[test]
    fn config_save_encrypts_pool_api_keys() {
        let dir = std::env::temp_dir().join(format!(
            "corvus_test_pool_credentials_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();

        let mut config = Config::default();
        config.workspace_dir = dir.join("workspace");
        config.config_path = dir.join("config.toml");
        config.reliability.account_pools.insert(
            "openrouter".into(),
            ProviderAccountPoolConfig {
                strategy: AccountPoolStrategy::RoundRobin,
                accounts: vec![sample_pool_account("acct-1", "pool-key", 1)],
            },
        );

        config.save().unwrap();

        let contents = fs::read_to_string(config.config_path.clone()).unwrap();
        let stored: Config = toml::from_str(&contents).unwrap();
        let store = crate::security::SecretStore::new(&dir, true);

        let pool = stored.reliability.account_pools.get("openrouter").unwrap();
        let encrypted = pool.accounts[0].api_key.as_str();
        assert!(crate::security::SecretStore::is_encrypted(encrypted));
        assert_eq!(store.decrypt(encrypted).unwrap(), "pool-key");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_save_atomic_cleanup() {
        let dir = std::env::temp_dir().join(format!("corvus_test_config_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let mut config = Config::default();
        config.workspace_dir = dir.join("workspace");
        config.config_path = config_path.clone();
        config.default_model = Some("model-a".into());

        config.save().unwrap();
        assert!(config_path.exists());

        config.default_model = Some("model-b".into());
        config.save().unwrap();

        let contents = fs::read_to_string(&config_path).unwrap();
        assert!(contents.contains("model-b"));

        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        assert!(!names.iter().any(|name| name.ends_with(".bak")));

        let _ = fs::remove_dir_all(&dir);
    }

    // ── Telegram / Discord config ────────────────────────────

    #[test]
    fn telegram_config_serde() {
        let tc = TelegramConfig {
            bot_token: "123:XYZ".into(),
            allowed_users: vec!["alice".into(), "bob".into()],
            stream_mode: StreamMode::Partial,
            draft_update_interval_ms: 500,
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: TelegramConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "123:XYZ");
        assert_eq!(parsed.allowed_users.len(), 2);
        assert_eq!(parsed.stream_mode, StreamMode::Partial);
        assert_eq!(parsed.draft_update_interval_ms, 500);
    }

    #[test]
    fn telegram_config_defaults_stream_off() {
        let json = r#"{"bot_token":"tok","allowed_users":[]}"#;
        let parsed: TelegramConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.stream_mode, StreamMode::Off);
        assert_eq!(parsed.draft_update_interval_ms, 1000);
    }

    #[test]
    fn discord_config_serde() {
        let dc = DiscordConfig {
            bot_token: "discord-token".into(),
            guild_id: Some("12345".into()),
            allowed_users: vec![],
            listen_to_bots: false,
            mention_only: false,
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "discord-token");
        assert_eq!(parsed.guild_id.as_deref(), Some("12345"));
    }

    #[test]
    fn discord_config_optional_guild() {
        let dc = DiscordConfig {
            bot_token: "tok".into(),
            guild_id: None,
            allowed_users: vec![],
            listen_to_bots: false,
            mention_only: false,
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.guild_id.is_none());
    }

    // ── iMessage / Matrix config ────────────────────────────

    #[test]
    fn imessage_config_serde() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["+1234567890".into(), "user@icloud.com".into()],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.allowed_contacts.len(), 2);
        assert_eq!(parsed.allowed_contacts[0], "+1234567890");
    }

    #[test]
    fn imessage_config_empty_contacts() {
        let ic = IMessageConfig {
            allowed_contacts: vec![],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.allowed_contacts.is_empty());
    }

    #[test]
    fn imessage_config_wildcard() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["*".into()],
        };
        let toml_str = toml::to_string(&ic).unwrap();
        let parsed: IMessageConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_contacts, vec!["*"]);
    }

    #[test]
    fn matrix_config_serde() {
        let mc = MatrixConfig {
            homeserver: "https://matrix.org".into(),
            access_token: "syt_token_abc".into(),
            room_id: "!room123:matrix.org".into(),
            allowed_users: vec!["@user:matrix.org".into()],
        };
        let json = serde_json::to_string(&mc).unwrap();
        let parsed: MatrixConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.homeserver, "https://matrix.org");
        assert_eq!(parsed.access_token, "syt_token_abc");
        assert_eq!(parsed.room_id, "!room123:matrix.org");
        assert_eq!(parsed.allowed_users.len(), 1);
    }

    #[test]
    fn matrix_config_toml_roundtrip() {
        let mc = MatrixConfig {
            homeserver: "https://synapse.local:8448".into(),
            access_token: "tok".into(),
            room_id: "!abc:synapse.local".into(),
            allowed_users: vec!["@admin:synapse.local".into(), "*".into()],
        };
        let toml_str = toml::to_string(&mc).unwrap();
        let parsed: MatrixConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.homeserver, "https://synapse.local:8448");
        assert_eq!(parsed.allowed_users.len(), 2);
    }

    #[test]
    fn signal_config_serde() {
        let sc = SignalConfig {
            http_url: "http://127.0.0.1:8686".into(),
            account: "+1234567890".into(),
            group_id: Some("group123".into()),
            allowed_from: vec!["+1111111111".into()],
            ignore_attachments: true,
            ignore_stories: false,
        };
        let json = serde_json::to_string(&sc).unwrap();
        let parsed: SignalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.http_url, "http://127.0.0.1:8686");
        assert_eq!(parsed.account, "+1234567890");
        assert_eq!(parsed.group_id.as_deref(), Some("group123"));
        assert_eq!(parsed.allowed_from.len(), 1);
        assert!(parsed.ignore_attachments);
        assert!(!parsed.ignore_stories);
    }

    #[test]
    fn signal_config_toml_roundtrip() {
        let sc = SignalConfig {
            http_url: "http://localhost:8080".into(),
            account: "+9876543210".into(),
            group_id: None,
            allowed_from: vec!["*".into()],
            ignore_attachments: false,
            ignore_stories: true,
        };
        let toml_str = toml::to_string(&sc).unwrap();
        let parsed: SignalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.http_url, "http://localhost:8080");
        assert_eq!(parsed.account, "+9876543210");
        assert!(parsed.group_id.is_none());
        assert!(parsed.ignore_stories);
    }

    #[test]
    fn signal_config_defaults() {
        let json = r#"{"http_url":"http://127.0.0.1:8686","account":"+1234567890"}"#;
        let parsed: SignalConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.group_id.is_none());
        assert!(parsed.allowed_from.is_empty());
        assert!(!parsed.ignore_attachments);
        assert!(!parsed.ignore_stories);
    }

    #[test]
    fn channels_config_with_imessage_and_matrix() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            mattermost: None,
            webhook: None,
            imessage: Some(IMessageConfig {
                allowed_contacts: vec!["+1".into()],
            }),
            matrix: Some(MatrixConfig {
                homeserver: "https://m.org".into(),
                access_token: "tok".into(),
                room_id: "!r:m".into(),
                allowed_users: vec!["@u:m".into()],
            }),
            signal: None,
            whatsapp: None,
            email: None,
            irc: None,
            lark: None,
            dingtalk: None,
            qq: None,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.imessage.is_some());
        assert!(parsed.matrix.is_some());
        assert_eq!(parsed.imessage.unwrap().allowed_contacts, vec!["+1"]);
        assert_eq!(parsed.matrix.unwrap().homeserver, "https://m.org");
    }

    #[test]
    fn channels_config_default_has_no_imessage_matrix() {
        let c = ChannelsConfig::default();
        assert!(c.imessage.is_none());
        assert!(c.matrix.is_none());
    }

    // ── Edge cases: serde(default) for allowed_users ─────────

    #[test]
    fn discord_config_deserializes_without_allowed_users() {
        // Old configs won't have allowed_users — serde(default) should fill vec![]
        let json = r#"{"bot_token":"tok","guild_id":"123"}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn discord_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"tok","guild_id":"123","allowed_users":["111","222"]}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["111", "222"]);
    }

    #[test]
    fn slack_config_deserializes_without_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok"}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn slack_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok","allowed_users":["U111"]}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["U111"]);
    }

    #[test]
    fn discord_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "tok"
guild_id = "123"
"#;
        let parsed: DiscordConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.bot_token, "tok");
    }

    #[test]
    fn slack_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "xoxb-tok"
channel_id = "C123"
"#;
        let parsed: SlackConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.channel_id.as_deref(), Some("C123"));
    }

    #[test]
    fn webhook_config_with_secret() {
        let json = r#"{"port":8080,"secret":"my-secret-key"}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.secret.as_deref(), Some("my-secret-key"));
    }

    #[test]
    fn webhook_config_without_secret() {
        let json = r#"{"port":8080}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.secret.is_none());
        assert_eq!(parsed.port, 8080);
    }

    // ── WhatsApp config ──────────────────────────────────────

    #[test]
    fn whatsapp_config_serde() {
        let wc = WhatsAppConfig {
            access_token: "EAABx...".into(),
            phone_number_id: "123456789".into(),
            verify_token: "my-verify-token".into(),
            app_secret: None,
            allowed_numbers: vec!["+1234567890".into(), "+9876543210".into()],
        };
        let json = serde_json::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "EAABx...");
        assert_eq!(parsed.phone_number_id, "123456789");
        assert_eq!(parsed.verify_token, "my-verify-token");
        assert_eq!(parsed.allowed_numbers.len(), 2);
    }

    #[test]
    fn whatsapp_config_toml_roundtrip() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "12345".into(),
            verify_token: "verify".into(),
            app_secret: Some("secret123".into()),
            allowed_numbers: vec!["+1".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.phone_number_id, "12345");
        assert_eq!(parsed.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn whatsapp_config_deserializes_without_allowed_numbers() {
        let json = r#"{"access_token":"tok","phone_number_id":"123","verify_token":"ver"}"#;
        let parsed: WhatsAppConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_numbers.is_empty());
    }

    #[test]
    fn whatsapp_config_wildcard_allowed() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "123".into(),
            verify_token: "ver".into(),
            app_secret: None,
            allowed_numbers: vec!["*".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_numbers, vec!["*"]);
    }

    #[test]
    fn channels_config_with_whatsapp() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            mattermost: None,
            webhook: None,
            imessage: None,
            matrix: None,
            signal: None,
            whatsapp: Some(WhatsAppConfig {
                access_token: "tok".into(),
                phone_number_id: "123".into(),
                verify_token: "ver".into(),
                app_secret: None,
                allowed_numbers: vec!["+1".into()],
            }),
            email: None,
            irc: None,
            lark: None,
            dingtalk: None,
            qq: None,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.whatsapp.is_some());
        let wa = parsed.whatsapp.unwrap();
        assert_eq!(wa.phone_number_id, "123");
        assert_eq!(wa.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn channels_config_default_has_no_whatsapp() {
        let c = ChannelsConfig::default();
        assert!(c.whatsapp.is_none());
    }

    // ══════════════════════════════════════════════════════════
    // SECURITY CHECKLIST TESTS — Gateway config
    // ══════════════════════════════════════════════════════════

    #[test]
    fn checklist_gateway_default_requires_pairing() {
        let g = GatewayConfig::default();
        assert!(g.require_pairing, "Pairing must be required by default");
    }

    #[test]
    fn checklist_gateway_default_blocks_public_bind() {
        let g = GatewayConfig::default();
        assert!(
            !g.allow_public_bind,
            "Public bind must be blocked by default"
        );
    }

    #[test]
    fn checklist_gateway_default_no_tokens() {
        let g = GatewayConfig::default();
        assert!(
            g.paired_tokens.is_empty(),
            "No pre-paired tokens by default"
        );
        assert!(!g.allow_unpaired_session_scopes);
        assert_eq!(g.pair_rate_limit_per_minute, 10);
        assert_eq!(g.webhook_rate_limit_per_minute, 60);
        assert!(!g.trust_forwarded_headers);
        assert_eq!(g.rate_limit_max_keys, 10_000);
        assert_eq!(g.idempotency_ttl_secs, 300);
        assert_eq!(g.idempotency_max_keys, 10_000);
    }

    #[test]
    fn checklist_gateway_cli_default_host_is_localhost() {
        // The CLI default for --host is 127.0.0.1 (checked in main.rs)
        // Here we verify the config default matches
        let c = Config::default();
        assert!(
            c.gateway.require_pairing,
            "Config default must require pairing"
        );
        assert!(
            !c.gateway.allow_public_bind,
            "Config default must block public bind"
        );
    }

    #[test]
    fn checklist_gateway_serde_roundtrip() {
        let g = GatewayConfig {
            port: 3000,
            host: "127.0.0.1".into(),
            admin_expose_provider_pools: false,
            require_pairing: true,
            allow_public_bind: false,
            allow_unpaired_session_scopes: true,
            paired_tokens: vec!["zc_test_token".into()],
            pair_rate_limit_per_minute: 12,
            webhook_rate_limit_per_minute: 80,
            trust_forwarded_headers: true,
            rate_limit_max_keys: 2048,
            idempotency_ttl_secs: 600,
            idempotency_max_keys: 4096,
            webhook_dispatcher_enabled: true,
        };
        let toml_str = toml::to_string(&g).unwrap();
        let parsed: GatewayConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.require_pairing);
        assert!(!parsed.allow_public_bind);
        assert!(parsed.allow_unpaired_session_scopes);
        assert_eq!(parsed.paired_tokens, vec!["zc_test_token"]);
        assert_eq!(parsed.pair_rate_limit_per_minute, 12);
        assert_eq!(parsed.webhook_rate_limit_per_minute, 80);
        assert!(parsed.trust_forwarded_headers);
        assert_eq!(parsed.rate_limit_max_keys, 2048);
        assert_eq!(parsed.idempotency_ttl_secs, 600);
        assert_eq!(parsed.idempotency_max_keys, 4096);
        assert!(parsed.webhook_dispatcher_enabled);
    }

    #[test]
    fn checklist_gateway_backward_compat_no_gateway_section() {
        // Old configs without [gateway] should get secure defaults
        let minimal = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.gateway.require_pairing,
            "Missing [gateway] must default to require_pairing=true"
        );
        assert!(
            !parsed.gateway.allow_public_bind,
            "Missing [gateway] must default to allow_public_bind=false"
        );
    }

    #[test]
    fn checklist_autonomy_default_is_workspace_scoped() {
        let a = AutonomyConfig::default();
        assert!(a.workspace_only, "Default autonomy must be workspace_only");
        assert!(
            a.forbidden_paths.contains(&"/etc".to_string()),
            "Must block /etc"
        );
        assert!(
            a.forbidden_paths.contains(&"/proc".to_string()),
            "Must block /proc"
        );
        assert!(
            a.forbidden_paths.contains(&"~/.ssh".to_string()),
            "Must block ~/.ssh"
        );
    }

    // ══════════════════════════════════════════════════════════
    // COMPOSIO CONFIG TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn composio_config_default_disabled() {
        let c = ComposioConfig::default();
        assert!(!c.enabled, "Composio must be disabled by default");
        assert!(c.api_key.is_none(), "No API key by default");
        assert_eq!(c.entity_id, "default");
    }

    #[test]
    fn composio_config_serde_roundtrip() {
        let c = ComposioConfig {
            enabled: true,
            api_key: Some("comp-key-123".into()),
            entity_id: "user42".into(),
        };
        let toml_str = toml::to_string(&c).unwrap();
        let parsed: ComposioConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.api_key.as_deref(), Some("comp-key-123"));
        assert_eq!(parsed.entity_id, "user42");
    }

    #[test]
    fn composio_config_backward_compat_missing_section() {
        let minimal = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            !parsed.composio.enabled,
            "Missing [composio] must default to disabled"
        );
        assert!(parsed.composio.api_key.is_none());
    }

    #[test]
    fn composio_config_partial_toml() {
        let toml_str = r"
enabled = true
";
        let parsed: ComposioConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.enabled);
        assert!(parsed.api_key.is_none());
        assert_eq!(parsed.entity_id, "default");
    }

    // ══════════════════════════════════════════════════════════
    // SECRETS CONFIG TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn secrets_config_default_encrypts() {
        let s = SecretsConfig::default();
        assert!(s.encrypt, "Encryption must be enabled by default");
    }

    #[test]
    fn secrets_config_serde_roundtrip() {
        let s = SecretsConfig { encrypt: false };
        let toml_str = toml::to_string(&s).unwrap();
        let parsed: SecretsConfig = toml::from_str(&toml_str).unwrap();
        assert!(!parsed.encrypt);
    }

    #[test]
    fn secrets_config_backward_compat_missing_section() {
        let minimal = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.secrets.encrypt,
            "Missing [secrets] must default to encrypt=true"
        );
    }

    #[test]
    fn config_default_has_composio_and_secrets() {
        let c = Config::default();
        assert!(!c.composio.enabled);
        assert!(c.composio.api_key.is_none());
        assert!(c.secrets.encrypt);
        assert!(!c.browser.enabled);
        assert!(c.browser.allowed_domains.is_empty());
    }

    #[test]
    fn browser_config_default_disabled() {
        let b = BrowserConfig::default();
        assert!(!b.enabled);
        assert!(b.allowed_domains.is_empty());
        assert_eq!(b.backend, "agent_browser");
        assert!(b.native_headless);
        assert_eq!(b.native_webdriver_url, "http://127.0.0.1:9515");
        assert!(b.native_chrome_path.is_none());
        assert_eq!(b.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");
        assert_eq!(b.computer_use.timeout_ms, 15_000);
        assert!(!b.computer_use.allow_remote_endpoint);
        assert!(b.computer_use.window_allowlist.is_empty());
        assert!(b.computer_use.max_coordinate_x.is_none());
        assert!(b.computer_use.max_coordinate_y.is_none());
    }

    #[test]
    fn browser_config_serde_roundtrip() {
        let b = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into(), "docs.example.com".into()],
            session_name: None,
            backend: "auto".into(),
            native_headless: false,
            native_webdriver_url: "http://localhost:4444".into(),
            native_chrome_path: Some("/usr/bin/chromium".into()),
            computer_use: BrowserComputerUseConfig {
                endpoint: "https://computer-use.example.com/v1/actions".into(),
                api_key: Some("test-token".into()),
                timeout_ms: 8_000,
                allow_remote_endpoint: true,
                window_allowlist: vec!["Chrome".into(), "Visual Studio Code".into()],
                max_coordinate_x: Some(3840),
                max_coordinate_y: Some(2160),
            },
        };
        let toml_str = toml::to_string(&b).unwrap();
        let parsed: BrowserConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.allowed_domains.len(), 2);
        assert_eq!(parsed.allowed_domains[0], "example.com");
        assert_eq!(parsed.backend, "auto");
        assert!(!parsed.native_headless);
        assert_eq!(parsed.native_webdriver_url, "http://localhost:4444");
        assert_eq!(
            parsed.native_chrome_path.as_deref(),
            Some("/usr/bin/chromium")
        );
        assert_eq!(
            parsed.computer_use.endpoint,
            "https://computer-use.example.com/v1/actions"
        );
        assert_eq!(parsed.computer_use.api_key.as_deref(), Some("test-token"));
        assert_eq!(parsed.computer_use.timeout_ms, 8_000);
        assert!(parsed.computer_use.allow_remote_endpoint);
        assert_eq!(parsed.computer_use.window_allowlist.len(), 2);
        assert_eq!(parsed.computer_use.max_coordinate_x, Some(3840));
        assert_eq!(parsed.computer_use.max_coordinate_y, Some(2160));
    }

    #[test]
    fn browser_config_backward_compat_missing_section() {
        let minimal = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(!parsed.browser.enabled);
        assert!(parsed.browser.allowed_domains.is_empty());
    }

    #[test]
    fn config_rejects_unknown_top_level_sections() {
        let raw = r#"
default_temperature = 0.7
[conductor]
enabled = true
"#;

        let err = toml::from_str::<Config>(raw).unwrap_err().to_string();
        assert!(err.contains("unknown field") || err.contains("conductor"));
    }

    // ── Environment variable overrides (Docker support) ─────────

    fn env_override_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_OVERRIDE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        ENV_OVERRIDE_TEST_LOCK
            .lock()
            .expect("env override test lock poisoned")
    }

    #[test]
    fn env_override_api_key() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert!(config.api_key.is_none());

        std::env::set_var("CORVUS_API_KEY", "sk-test-env-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-test-env-key"));

        std::env::remove_var("CORVUS_API_KEY");
    }

    #[test]
    fn env_override_api_key_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("CORVUS_API_KEY");
        std::env::set_var("API_KEY", "sk-fallback-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-fallback-key"));

        std::env::remove_var("API_KEY");
    }

    #[test]
    fn env_override_provider() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("CORVUS_PROVIDER", "anthropic");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("anthropic"));

        std::env::remove_var("CORVUS_PROVIDER");
    }

    #[test]
    fn env_override_provider_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("CORVUS_PROVIDER");
        std::env::set_var("PROVIDER", "openai");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("openai"));

        std::env::remove_var("PROVIDER");
    }

    #[test]
    fn env_override_glm_api_key_for_regional_aliases() {
        let _env_guard = env_override_test_guard();
        let mut config = Config {
            default_provider: Some("glm-cn".to_string()),
            ..Config::default()
        };

        std::env::set_var("GLM_API_KEY", "glm-regional-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("glm-regional-key"));

        std::env::remove_var("GLM_API_KEY");
    }

    #[test]
    fn env_override_zai_api_key_for_regional_aliases() {
        let _env_guard = env_override_test_guard();
        let mut config = Config {
            default_provider: Some("zai-cn".to_string()),
            ..Config::default()
        };

        std::env::set_var("ZAI_API_KEY", "zai-regional-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("zai-regional-key"));

        std::env::remove_var("ZAI_API_KEY");
    }

    #[test]
    fn env_override_model() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("CORVUS_MODEL", "gpt-4o");
        config.apply_env_overrides();
        assert_eq!(config.default_model.as_deref(), Some("gpt-4o"));

        std::env::remove_var("CORVUS_MODEL");
    }

    #[test]
    fn env_override_model_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("CORVUS_MODEL");
        std::env::set_var("MODEL", "anthropic/claude-3.5-sonnet");
        config.apply_env_overrides();
        assert_eq!(
            config.default_model.as_deref(),
            Some("anthropic/claude-3.5-sonnet")
        );

        std::env::remove_var("MODEL");
    }

    #[test]
    fn env_override_memory_backend() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.memory.backend, "sqlite");

        std::env::set_var("CORVUS_MEMORY_BACKEND", "markdown");
        config.apply_env_overrides();
        assert_eq!(config.memory.backend, "markdown");

        std::env::remove_var("CORVUS_MEMORY_BACKEND");
    }

    #[test]
    fn env_override_memory_backend_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.memory.backend, "sqlite");

        std::env::remove_var("CORVUS_MEMORY_BACKEND");
        std::env::set_var("MEMORY_BACKEND", "markdown");
        config.apply_env_overrides();
        assert_eq!(config.memory.backend, "markdown");

        std::env::remove_var("MEMORY_BACKEND");
    }

    #[test]
    fn env_override_memory_backend_invalid_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.memory.backend, "sqlite");

        std::env::set_var("CORVUS_MEMORY_BACKEND", "unsupported");
        config.apply_env_overrides();
        assert_eq!(config.memory.backend, "sqlite");

        std::env::remove_var("CORVUS_MEMORY_BACKEND");
    }

    #[test]
    fn env_override_workspace() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let expected_workspace = PathBuf::from("/custom/workspace");
        let (expected_config_dir, expected_workspace_dir) =
            resolve_config_dir_for_workspace(&expected_workspace);

        std::env::set_var("CORVUS_WORKSPACE", "/custom/workspace");
        config.apply_env_overrides();
        assert_eq!(config.workspace_dir, expected_workspace_dir);
        assert_eq!(config.config_path, expected_config_dir.join("config.toml"));

        std::env::remove_var("CORVUS_WORKSPACE");
    }

    #[test]
    fn load_or_init_workspace_override_uses_workspace_root_for_config() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let workspace_dir = temp_home.join("profile-a");

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);
        std::env::set_var("CORVUS_WORKSPACE", &workspace_dir);

        let config = Config::load_or_init().unwrap();

        assert_eq!(config.workspace_dir, workspace_dir.join("workspace"));
        assert_eq!(config.config_path, workspace_dir.join("config.toml"));
        assert!(workspace_dir.join("config.toml").exists());

        std::env::remove_var("CORVUS_WORKSPACE");
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn load_or_init_workspace_suffix_uses_legacy_config_layout() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let workspace_dir = temp_home.join("workspace");
        let legacy_config_path = temp_home.join(".corvus").join("config.toml");

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);
        std::env::set_var("CORVUS_WORKSPACE", &workspace_dir);

        let config = Config::load_or_init().unwrap();

        assert_eq!(config.workspace_dir, workspace_dir);
        assert_eq!(config.config_path, legacy_config_path);
        assert!(config.config_path.exists());

        std::env::remove_var("CORVUS_WORKSPACE");
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn load_or_init_workspace_override_keeps_existing_legacy_config() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let workspace_dir = temp_home.join("custom-workspace");
        let legacy_config_dir = temp_home.join(".corvus");
        let legacy_config_path = legacy_config_dir.join("config.toml");

        fs::create_dir_all(&legacy_config_dir).unwrap();
        fs::write(
            &legacy_config_path,
            r#"default_temperature = 0.7
default_model = "legacy-model"
"#,
        )
        .unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);
        std::env::set_var("CORVUS_WORKSPACE", &workspace_dir);

        let config = Config::load_or_init().unwrap();

        assert_eq!(config.workspace_dir, workspace_dir);
        assert_eq!(config.config_path, legacy_config_path);
        assert_eq!(config.default_model.as_deref(), Some("legacy-model"));

        std::env::remove_var("CORVUS_WORKSPACE");
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn load_or_init_uses_persisted_active_workspace_marker() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let custom_config_dir = temp_home.join("profiles").join("agent-alpha");

        fs::create_dir_all(&custom_config_dir).unwrap();
        fs::write(
            custom_config_dir.join("config.toml"),
            "default_temperature = 0.7\ndefault_model = \"persisted-profile\"\n",
        )
        .unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);
        std::env::remove_var("CORVUS_WORKSPACE");

        persist_active_workspace_config_dir(&custom_config_dir).unwrap();

        let config = Config::load_or_init().unwrap();

        assert_eq!(config.config_path, custom_config_dir.join("config.toml"));
        assert_eq!(config.workspace_dir, custom_config_dir.join("workspace"));
        assert_eq!(config.default_model.as_deref(), Some("persisted-profile"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn load_or_init_env_workspace_override_takes_priority_over_marker() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let marker_config_dir = temp_home.join("profiles").join("persisted-profile");
        let env_workspace_dir = temp_home.join("env-workspace");

        fs::create_dir_all(&marker_config_dir).unwrap();
        fs::write(
            marker_config_dir.join("config.toml"),
            "default_temperature = 0.7\ndefault_model = \"marker-model\"\n",
        )
        .unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);
        persist_active_workspace_config_dir(&marker_config_dir).unwrap();
        std::env::set_var("CORVUS_WORKSPACE", &env_workspace_dir);

        let config = Config::load_or_init().unwrap();

        assert_eq!(config.workspace_dir, env_workspace_dir.join("workspace"));
        assert_eq!(config.config_path, env_workspace_dir.join("config.toml"));

        std::env::remove_var("CORVUS_WORKSPACE");
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn persist_active_workspace_marker_is_cleared_for_default_config_dir() {
        let _env_guard = env_override_test_guard();
        let temp_home =
            std::env::temp_dir().join(format!("corvus_test_home_{}", uuid::Uuid::new_v4()));
        let default_config_dir = temp_home.join(".corvus");
        let custom_config_dir = temp_home.join("profiles").join("custom-profile");
        let marker_path = default_config_dir.join(ACTIVE_WORKSPACE_STATE_FILE);

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);

        persist_active_workspace_config_dir(&custom_config_dir).unwrap();
        assert!(marker_path.exists());

        persist_active_workspace_config_dir(&default_config_dir).unwrap();
        assert!(!marker_path.exists());

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn env_override_empty_values_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let original_provider = config.default_provider.clone();

        std::env::set_var("CORVUS_PROVIDER", "");
        config.apply_env_overrides();
        assert_eq!(config.default_provider, original_provider);

        std::env::remove_var("CORVUS_PROVIDER");
    }

    #[test]
    fn env_override_gateway_port() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.gateway.port, 3000);

        std::env::set_var("CORVUS_GATEWAY_PORT", "8080");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 8080);

        std::env::remove_var("CORVUS_GATEWAY_PORT");
    }

    #[test]
    fn env_override_port_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("CORVUS_GATEWAY_PORT");
        std::env::set_var("PORT", "9000");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 9000);

        std::env::remove_var("PORT");
    }

    #[test]
    fn env_override_gateway_host() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        assert_eq!(config.gateway.host, "127.0.0.1");

        std::env::set_var("CORVUS_GATEWAY_HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("CORVUS_GATEWAY_HOST");
    }

    #[test]
    fn env_override_gateway_webhook_dispatcher() {
        let _env_guard = env_override_test_guard();
        {
            let _dispatcher_lock = acquire_gateway_webhook_dispatcher_lock_blocking();
            std::env::set_var("CORVUS_GATEWAY_WEBHOOK_DISPATCHER", "0");
        }

        let mut config = Config::default();
        assert!(!config.gateway.webhook_dispatcher_enabled);

        {
            let _dispatcher_env = GatewayWebhookDispatcherEnvGuard::set_blocking("1");
            config.apply_env_overrides();
            assert!(config.gateway.webhook_dispatcher_enabled);
        }

        assert_eq!(
            std::env::var("CORVUS_GATEWAY_WEBHOOK_DISPATCHER").as_deref(),
            Ok("0")
        );

        {
            let _dispatcher_lock = acquire_gateway_webhook_dispatcher_lock_blocking();
            std::env::remove_var("CORVUS_GATEWAY_WEBHOOK_DISPATCHER");
        }
    }

    #[test]
    fn env_override_host_fallback() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::remove_var("CORVUS_GATEWAY_HOST");
        std::env::set_var("HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("HOST");
    }

    #[test]
    fn env_override_temperature() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("CORVUS_TEMPERATURE", "0.5");
        config.apply_env_overrides();
        assert!((config.default_temperature - 0.5).abs() < f64::EPSILON);

        std::env::remove_var("CORVUS_TEMPERATURE");
    }

    #[test]
    fn env_override_temperature_out_of_range_ignored() {
        let _env_guard = env_override_test_guard();
        // Clean up any leftover env vars from other tests
        std::env::remove_var("CORVUS_TEMPERATURE");

        let mut config = Config::default();
        let original_temp = config.default_temperature;

        // Temperature > 2.0 should be ignored
        std::env::set_var("CORVUS_TEMPERATURE", "3.0");
        config.apply_env_overrides();
        assert!(
            (config.default_temperature - original_temp).abs() < f64::EPSILON,
            "Temperature 3.0 should be ignored (out of range)"
        );

        std::env::remove_var("CORVUS_TEMPERATURE");
    }

    #[test]
    fn env_override_invalid_port_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let original_port = config.gateway.port;

        std::env::set_var("PORT", "not_a_number");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, original_port);

        std::env::remove_var("PORT");
    }

    #[test]
    fn env_override_web_search_config() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("WEB_SEARCH_ENABLED", "false");
        std::env::set_var("WEB_SEARCH_PROVIDER", "brave");
        std::env::set_var("WEB_SEARCH_MAX_RESULTS", "7");
        std::env::set_var("WEB_SEARCH_TIMEOUT_SECS", "20");
        std::env::set_var("BRAVE_API_KEY", "brave-test-key");

        config.apply_env_overrides();

        assert!(!config.web_search.enabled);
        assert_eq!(config.web_search.provider, "brave");
        assert_eq!(config.web_search.max_results, 7);
        assert_eq!(config.web_search.timeout_secs, 20);
        assert_eq!(
            config.web_search.brave_api_key.as_deref(),
            Some("brave-test-key")
        );

        std::env::remove_var("WEB_SEARCH_ENABLED");
        std::env::remove_var("WEB_SEARCH_PROVIDER");
        std::env::remove_var("WEB_SEARCH_MAX_RESULTS");
        std::env::remove_var("WEB_SEARCH_TIMEOUT_SECS");
        std::env::remove_var("BRAVE_API_KEY");
    }

    #[test]
    fn env_override_web_search_invalid_values_ignored() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        let original_provider = config.web_search.provider.clone();
        let original_max_results = config.web_search.max_results;
        let original_timeout = config.web_search.timeout_secs;

        std::env::set_var("WEB_SEARCH_PROVIDER", "DuckDuckGo");
        config.apply_env_overrides();
        assert_eq!(config.web_search.provider, "duckduckgo");

        std::env::set_var("WEB_SEARCH_PROVIDER", "bing");
        std::env::set_var("WEB_SEARCH_MAX_RESULTS", "99");
        std::env::set_var("WEB_SEARCH_TIMEOUT_SECS", "0");

        config.apply_env_overrides();

        assert_eq!(config.web_search.provider, "duckduckgo");
        assert_eq!(config.web_search.max_results, original_max_results);
        assert_eq!(config.web_search.timeout_secs, original_timeout);

        config.web_search.provider = original_provider;
        std::env::remove_var("WEB_SEARCH_MAX_RESULTS");
        std::env::remove_var("WEB_SEARCH_TIMEOUT_SECS");
        std::env::remove_var("WEB_SEARCH_PROVIDER");
    }

    #[test]
    fn env_override_cerebro_memory_config() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("CORVUS_CEREBRO_ENDPOINT", "https://cerebro.example.com/mcp");
        std::env::set_var("CORVUS_CEREBRO_AUTH_TOKEN", "svc-token");
        std::env::set_var("CORVUS_CEREBRO_TIMEOUT_MS", "45000");
        std::env::set_var("CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK", "true");

        config.apply_env_overrides();

        assert_eq!(
            config.memory.cerebro.endpoint.as_deref(),
            Some("https://cerebro.example.com/mcp")
        );
        assert_eq!(
            config.memory.cerebro.auth_token.as_deref(),
            Some("svc-token")
        );
        assert_eq!(config.memory.cerebro.request_timeout_ms, 45_000);
        assert!(config.memory.cerebro.allow_insecure_loopback);

        std::env::remove_var("CORVUS_CEREBRO_ENDPOINT");
        std::env::remove_var("CORVUS_CEREBRO_AUTH_TOKEN");
        std::env::remove_var("CORVUS_CEREBRO_TIMEOUT_MS");
        std::env::remove_var("CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK");
    }

    #[test]
    fn env_override_updates_policy_fields() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();

        std::env::set_var("CORVUS_UPDATES_ENABLED", "false");
        std::env::set_var("CORVUS_UPDATE_AUTO_INSTALL", "true");
        std::env::set_var("CORVUS_UPDATE_CHANNEL_VISIBILITY", "false");
        std::env::set_var("CORVUS_UPDATE_CLI_NOTICE", "false");
        std::env::set_var("CORVUS_UPDATE_METHOD_OVERRIDE", "cargo");
        std::env::set_var("CORVUS_UPDATE_RESTART_POLICY", "never");

        config.apply_env_overrides();

        assert!(!config.updates.enabled);
        assert!(config.updates.auto_install_enabled);
        assert!(!config.updates.channel_visibility_enabled);
        assert!(!config.updates.cli_startup_notice_enabled);
        assert_eq!(
            config.updates.install_method_override.as_deref(),
            Some("cargo")
        );
        assert_eq!(config.updates.restart_policy, "never");

        std::env::remove_var("CORVUS_UPDATES_ENABLED");
        std::env::remove_var("CORVUS_UPDATE_AUTO_INSTALL");
        std::env::remove_var("CORVUS_UPDATE_CHANNEL_VISIBILITY");
        std::env::remove_var("CORVUS_UPDATE_CLI_NOTICE");
        std::env::remove_var("CORVUS_UPDATE_METHOD_OVERRIDE");
        std::env::remove_var("CORVUS_UPDATE_RESTART_POLICY");
    }

    #[test]
    fn env_override_updates_invalid_values_fail_safe() {
        let _env_guard = env_override_test_guard();
        let mut config = Config::default();
        config.updates.install_method_override = Some("npm".to_string());
        config.updates.restart_policy = "prompt".to_string();

        std::env::set_var("CORVUS_UPDATE_METHOD_OVERRIDE", "unknown");
        std::env::set_var("CORVUS_UPDATE_RESTART_POLICY", "invalid");

        config.apply_env_overrides();

        assert_eq!(
            config.updates.install_method_override.as_deref(),
            Some("npm")
        );
        assert_eq!(config.updates.restart_policy, "prompt");

        std::env::remove_var("CORVUS_UPDATE_METHOD_OVERRIDE");
        std::env::remove_var("CORVUS_UPDATE_RESTART_POLICY");
    }

    #[test]
    fn gateway_config_default_values() {
        let g = GatewayConfig::default();
        assert_eq!(g.port, 3000);
        assert_eq!(g.host, "127.0.0.1");
        assert!(g.require_pairing);
        assert!(!g.allow_public_bind);
        assert!(g.paired_tokens.is_empty());
        assert!(!g.trust_forwarded_headers);
        assert_eq!(g.rate_limit_max_keys, 10_000);
        assert_eq!(g.idempotency_max_keys, 10_000);
        assert!(!g.webhook_dispatcher_enabled);
    }

    // ── Peripherals config ───────────────────────────────────────

    #[test]
    fn peripherals_config_default_disabled() {
        let p = PeripheralsConfig::default();
        assert!(!p.enabled);
        assert!(p.boards.is_empty());
    }

    #[test]
    fn peripheral_board_config_defaults() {
        let b = PeripheralBoardConfig::default();
        assert!(b.board.is_empty());
        assert_eq!(b.transport, "serial");
        assert!(b.path.is_none());
        assert_eq!(b.baud, 115_200);
    }

    #[test]
    fn peripherals_config_toml_roundtrip() {
        let p = PeripheralsConfig {
            enabled: true,
            boards: vec![PeripheralBoardConfig {
                board: "nucleo-f401re".into(),
                transport: "serial".into(),
                path: Some("/dev/ttyACM0".into()),
                baud: 115_200,
            }],
            datasheet_dir: None,
        };
        let toml_str = toml::to_string(&p).unwrap();
        let parsed: PeripheralsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.boards.len(), 1);
        assert_eq!(parsed.boards[0].board, "nucleo-f401re");
        assert_eq!(parsed.boards[0].path.as_deref(), Some("/dev/ttyACM0"));
    }

    #[test]
    fn lark_config_serde() {
        let lc = LarkConfig {
            app_id: "cli_123456".into(),
            app_secret: "secret_abc".into(),
            encrypt_key: Some("encrypt_key".into()),
            verification_token: Some("verify_token".into()),
            allowed_users: vec!["user_123".into(), "user_456".into()],
            use_feishu: true,
            receive_mode: LarkReceiveMode::Websocket,
            port: None,
        };
        let json = serde_json::to_string(&lc).unwrap();
        let parsed: LarkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.app_id, "cli_123456");
        assert_eq!(parsed.app_secret, "secret_abc");
        assert_eq!(parsed.encrypt_key.as_deref(), Some("encrypt_key"));
        assert_eq!(parsed.verification_token.as_deref(), Some("verify_token"));
        assert_eq!(parsed.allowed_users.len(), 2);
        assert!(parsed.use_feishu);
    }

    #[test]
    fn lark_config_toml_roundtrip() {
        let lc = LarkConfig {
            app_id: "cli_123456".into(),
            app_secret: "secret_abc".into(),
            encrypt_key: Some("encrypt_key".into()),
            verification_token: Some("verify_token".into()),
            allowed_users: vec!["*".into()],
            use_feishu: false,
            receive_mode: LarkReceiveMode::Webhook,
            port: Some(9898),
        };
        let toml_str = toml::to_string(&lc).unwrap();
        let parsed: LarkConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.app_id, "cli_123456");
        assert_eq!(parsed.app_secret, "secret_abc");
        assert!(!parsed.use_feishu);
    }

    #[test]
    fn lark_config_deserializes_without_optional_fields() {
        let json = r#"{"app_id":"cli_123","app_secret":"secret"}"#;
        let parsed: LarkConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.encrypt_key.is_none());
        assert!(parsed.verification_token.is_none());
        assert!(parsed.allowed_users.is_empty());
        assert!(!parsed.use_feishu);
    }

    #[test]
    fn lark_config_defaults_to_lark_endpoint() {
        let json = r#"{"app_id":"cli_123","app_secret":"secret"}"#;
        let parsed: LarkConfig = serde_json::from_str(json).unwrap();
        assert!(
            !parsed.use_feishu,
            "use_feishu should default to false (Lark)"
        );
    }

    #[test]
    fn lark_config_with_wildcard_allowed_users() {
        let json = r#"{"app_id":"cli_123","app_secret":"secret","allowed_users":["*"]}"#;
        let parsed: LarkConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["*"]);
    }

    // ── Config file permission hardening (Unix only) ───────────────

    #[cfg(unix)]
    #[test]
    fn new_config_file_has_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        // Create a config and save it
        let mut config = Config::default();
        config.config_path = config_path.clone();
        config.save().unwrap();

        // Apply the same permission logic as load_or_init
        let _ = std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600));

        let meta = std::fs::metadata(&config_path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "New config file should be owner-only (0600), got {mode:o}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn world_readable_config_is_detectable() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        // Create a config file with intentionally loose permissions
        std::fs::write(&config_path, "# test config").unwrap();
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let meta = std::fs::metadata(&config_path).unwrap();
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o004 != 0,
            "Test setup: file should be world-readable (mode {mode:o})"
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_restricts_permissions_even_if_config_became_insecure() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let mut config = Config::default();
        config.config_path = config_path.clone();
        config.save().unwrap();

        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        config.save().unwrap();

        let mode = std::fs::metadata(&config_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn hardware_config_default_values() {
        let hw = HardwareConfig::default();
        assert!(!hw.enabled);
        assert_eq!(hw.transport, HardwareTransport::None);
        assert_eq!(hw.baud_rate, 115_200);
        assert!(hw.serial_port.is_none());
        assert!(hw.probe_target.is_none());
        assert!(!hw.workspace_datasheets);
    }

    #[test]
    fn hardware_transport_display() {
        assert_eq!(HardwareTransport::None.to_string(), "none");
        assert_eq!(HardwareTransport::Native.to_string(), "native");
        assert_eq!(HardwareTransport::Serial.to_string(), "serial");
        assert_eq!(HardwareTransport::Probe.to_string(), "probe");
    }

    #[test]
    fn hardware_config_transport_mode() {
        let mut hw = HardwareConfig::default();
        hw.transport = HardwareTransport::Serial;
        assert_eq!(hw.transport_mode(), HardwareTransport::Serial);
    }

    #[test]
    fn delegate_agent_config_default_max_depth() {
        let delegate = DelegateAgentConfig {
            provider: "openrouter".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            max_depth: default_max_depth(),
            execution_mode: DelegateExecutionMode::default(),
            max_iterations: None,
            timeout_ms: None,
        };
        assert_eq!(delegate.max_depth, 3);
    }

    #[test]
    fn identity_config_default_format() {
        let identity = IdentityConfig::default();
        assert_eq!(identity.format, "openclaw");
        assert!(identity.aieos_path.is_none());
        assert!(identity.aieos_inline.is_none());
    }

    #[test]
    fn agent_config_default_values() {
        let agent = AgentConfig::default();
        assert!(!agent.compact_context);
        assert_eq!(agent.profile, "full");
        assert_eq!(agent.max_tool_iterations, 10);
        assert_eq!(agent.max_history_messages, 50);
        assert!(!agent.parallel_tools);
        assert_eq!(agent.tool_dispatcher, "auto");
    }

    #[test]
    fn model_route_config_can_override_api_key() {
        let route = ModelRouteConfig {
            hint: "reasoning".to_string(),
            provider: "openrouter".to_string(),
            model: "claude-opus-4".to_string(),
            api_key: Some("sk-override".to_string()),
            allow_image_input: false,
        };
        assert_eq!(route.api_key, Some("sk-override".to_string()));
    }

    #[test]
    fn query_classification_config_default_disabled() {
        let qc = QueryClassificationConfig::default();
        assert!(!qc.enabled);
        assert!(qc.rules.is_empty());
    }

    #[test]
    fn classification_rule_length_constraints() {
        let rule = ClassificationRule {
            hint: "code".to_string(),
            keywords: vec!["rust".to_string(), "cargo".to_string()],
            patterns: vec!["fn ".to_string(), "impl ".to_string()],
            min_length: Some(10),
            max_length: Some(1000),
            priority: 10,
        };
        assert_eq!(rule.min_length, Some(10));
        assert_eq!(rule.max_length, Some(1000));
        assert_eq!(rule.priority, 10);
    }

    #[test]
    fn hardware_transport_serde_roundtrip() {
        for transport in [
            HardwareTransport::None,
            HardwareTransport::Native,
            HardwareTransport::Serial,
            HardwareTransport::Probe,
        ] {
            let json = serde_json::to_string(&transport).unwrap();
            let parsed: HardwareTransport = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, transport);
        }
    }

    #[test]
    fn hardware_config_serde_with_serial_port() {
        let hw = HardwareConfig {
            enabled: true,
            transport: HardwareTransport::Serial,
            serial_port: Some("/dev/ttyACM0".to_string()),
            baud_rate: 115_200,
            probe_target: None,
            workspace_datasheets: true,
        };
        let json = serde_json::to_string(&hw).unwrap();
        let parsed: HardwareConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.transport, HardwareTransport::Serial);
        assert_eq!(parsed.serial_port, Some("/dev/ttyACM0".to_string()));
        assert!(parsed.workspace_datasheets);
    }

    #[test]
    fn delegate_agent_config_serde_roundtrip() {
        let delegate = DelegateAgentConfig {
            provider: "openrouter".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            system_prompt: Some("You are a helpful assistant".to_string()),
            api_key: Some("sk-test".to_string()),
            temperature: Some(0.5),
            max_depth: 2,
            execution_mode: DelegateExecutionMode::default(),
            max_iterations: None,
            timeout_ms: None,
        };
        let json = serde_json::to_string(&delegate).unwrap();
        let parsed: DelegateAgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, "openrouter");
        assert_eq!(parsed.model, "claude-3-5-sonnet");
        assert_eq!(
            parsed.system_prompt,
            Some("You are a helpful assistant".to_string())
        );
        assert_eq!(parsed.max_depth, 2);
    }

    #[test]
    fn identity_config_supports_both_path_and_inline() {
        let identity = IdentityConfig {
            format: "aieos".to_string(),
            aieos_path: Some("identity.json".to_string()),
            aieos_inline: Some(r#"{"name":"Agent"}"#.to_string()),
        };
        assert_eq!(identity.format, "aieos");
        assert!(identity.aieos_path.is_some());
        assert!(identity.aieos_inline.is_some());
    }

    #[test]
    fn classification_rule_default_values() {
        let rule = ClassificationRule::default();
        assert!(rule.hint.is_empty());
        assert!(rule.keywords.is_empty());
        assert!(rule.patterns.is_empty());
        assert_eq!(rule.min_length, None);
        assert_eq!(rule.max_length, None);
        assert_eq!(rule.priority, 0);
    }

    // ── Phase 1.1: CodeSessionConfig / ValidationCommandConfig / DelegateExecutionMode ──

    #[test]
    fn code_session_config_default_values() {
        let cfg = CodeSessionConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.validation_commands.is_empty());
        assert_eq!(cfg.max_iterations, 50);
        assert_eq!(cfg.timeout_ms, 600_000);
    }

    #[test]
    fn validation_command_config_required_defaults_to_true() {
        let cmd = ValidationCommandConfig {
            command: "cargo test".into(),
            ..ValidationCommandConfig::default()
        };
        assert!(cmd.required, "required must default to true");
        assert_eq!(cmd.timeout_ms, 60_000);
    }

    #[test]
    fn validation_command_config_toml_roundtrip() {
        let toml_str = r#"
            command = "cargo clippy"
            required = false
            timeout_ms = 30000
        "#;
        let parsed: ValidationCommandConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.command, "cargo clippy");
        assert!(!parsed.required);
        assert_eq!(parsed.timeout_ms, 30_000);

        let back = toml::to_string(&parsed).unwrap();
        let reparsed: ValidationCommandConfig = toml::from_str(&back).unwrap();
        assert_eq!(reparsed.command, parsed.command);
        assert_eq!(reparsed.required, parsed.required);
        assert_eq!(reparsed.timeout_ms, parsed.timeout_ms);
    }

    #[test]
    fn delegate_execution_mode_defaults_to_one_shot() {
        let mode = DelegateExecutionMode::default();
        assert_eq!(mode, DelegateExecutionMode::OneShot);
    }

    #[test]
    fn delegate_execution_mode_toml_roundtrip() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Wrapper {
            mode: DelegateExecutionMode,
        }

        let session: Wrapper = toml::from_str(r#"mode = "session""#).unwrap();
        assert_eq!(session.mode, DelegateExecutionMode::Session);

        let one_shot: Wrapper = toml::from_str(r#"mode = "one_shot""#).unwrap();
        assert_eq!(one_shot.mode, DelegateExecutionMode::OneShot);

        let back = toml::to_string(&session).unwrap();
        let reparsed: Wrapper = toml::from_str(&back).unwrap();
        assert_eq!(reparsed.mode, DelegateExecutionMode::Session);
    }

    #[test]
    fn delegate_agent_config_new_fields_have_safe_defaults() {
        let toml_str = r#"
            provider = "anthropic"
            model = "claude-3-5-haiku"
        "#;
        let cfg: DelegateAgentConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.execution_mode, DelegateExecutionMode::OneShot);
        assert_eq!(cfg.max_iterations, None);
        assert_eq!(cfg.timeout_ms, None);
        assert_eq!(cfg.max_depth, 3);
    }

    #[test]
    fn agent_config_includes_code_session_with_defaults() {
        let cfg = AgentConfig::default();
        assert!(!cfg.code_session.enabled);
        assert!(cfg.code_session.validation_commands.is_empty());
        assert_eq!(cfg.code_session.max_iterations, 50);
        assert_eq!(cfg.code_session.timeout_ms, 600_000);
    }

    #[test]
    fn agent_config_code_session_deserializes_from_toml() {
        let toml_str = r#"
            [code_session]
            enabled = true
            max_iterations = 30
            timeout_ms = 120000

            [[code_session.validation_commands]]
            command = "cargo test"
            required = true
        "#;
        let cfg: AgentConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.code_session.enabled);
        assert_eq!(cfg.code_session.max_iterations, 30);
        assert_eq!(cfg.code_session.timeout_ms, 120_000);
        assert_eq!(cfg.code_session.validation_commands.len(), 1);
        assert_eq!(
            cfg.code_session.validation_commands[0].command,
            "cargo test"
        );
        assert!(cfg.code_session.validation_commands[0].required);
    }

    // ── Multimodal config (Task 4.4) ─────────────────────────

    #[test]
    fn multimodal_config_defaults_are_deny_all() {
        let mm = MultimodalConfig::default();
        assert!(!mm.enabled);
        assert!(mm.allowed_channels.is_empty());
        assert!(mm.vision_model_hint.is_none());
        assert!(mm.max_image_bytes.is_none());
    }

    #[test]
    fn config_defaults_multimodal_when_section_missing() {
        let toml_str = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(!parsed.multimodal.enabled);
        assert!(parsed.multimodal.allowed_channels.is_empty());
        assert!(parsed.multimodal.vision_model_hint.is_none());
        assert!(parsed.multimodal.max_image_bytes.is_none());
    }

    #[test]
    fn multimodal_config_deserializes_full_section() {
        let toml_str = r#"
default_temperature = 0.7

[multimodal]
enabled = true
allowed_channels = ["telegram", "whatsapp", "discord"]
vision_model_hint = "vision"
max_image_bytes = 5242880
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(parsed.multimodal.enabled);
        assert_eq!(
            parsed.multimodal.allowed_channels,
            vec!["telegram", "whatsapp", "discord"]
        );
        assert_eq!(
            parsed.multimodal.vision_model_hint.as_deref(),
            Some("vision")
        );
        assert_eq!(parsed.multimodal.max_image_bytes, Some(5_242_880));
    }

    #[test]
    fn multimodal_config_partial_section_uses_defaults() {
        let toml_str = r#"
default_temperature = 0.7

[multimodal]
enabled = true
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(parsed.multimodal.enabled);
        assert!(parsed.multimodal.allowed_channels.is_empty());
        assert!(parsed.multimodal.vision_model_hint.is_none());
        assert!(parsed.multimodal.max_image_bytes.is_none());
    }

    #[test]
    fn model_route_config_allow_image_input_defaults_false() {
        let toml_str = r#"
hint = "fast"
provider = "openrouter"
model = "gpt-4o"
"#;
        let parsed: ModelRouteConfig = toml::from_str(toml_str).unwrap();
        assert!(!parsed.allow_image_input);
    }

    #[test]
    fn model_route_config_allow_image_input_opt_in() {
        let toml_str = r#"
hint = "vision"
provider = "openrouter"
model = "gpt-4o"
allow_image_input = true
"#;
        let parsed: ModelRouteConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allow_image_input);
    }

    #[test]
    fn multimodal_validation_passes_when_disabled() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                ..MultimodalConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_multimodal_config().is_ok());
    }

    fn make_vision_route() -> ModelRouteConfig {
        ModelRouteConfig {
            hint: "vision".into(),
            provider: "test-provider".into(),
            model: "test-model".into(),
            api_key: None,
            allow_image_input: true,
        }
    }

    #[test]
    fn multimodal_validation_passes_when_fully_configured() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into()],
                vision_model_hint: Some("vision".into()),
                max_image_bytes: None,
            },
            model_routes: vec![make_vision_route()],
            ..Config::default()
        };
        // Warnings emitted but no error
        assert!(config.validate_multimodal_config().is_ok());
    }

    #[test]
    fn multimodal_validation_rejects_empty_channels_when_enabled() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: true,
                allowed_channels: Vec::new(),
                vision_model_hint: Some("vision".into()),
                max_image_bytes: None,
            },
            model_routes: vec![make_vision_route()],
            ..Config::default()
        };
        let error = config
            .validate_multimodal_config()
            .expect_err("should fail");
        assert!(error.to_string().contains("allowed_channels"));
    }

    #[test]
    fn multimodal_validation_rejects_missing_vision_hint_when_enabled() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into()],
                vision_model_hint: None,
                max_image_bytes: None,
            },
            ..Config::default()
        };
        let error = config
            .validate_multimodal_config()
            .expect_err("should fail");
        assert!(error.to_string().contains("vision_model_hint"));
    }

    #[test]
    fn multimodal_validation_passes_with_discord_channel() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: true,
                allowed_channels: vec!["discord".into()],
                vision_model_hint: Some("vision".into()),
                max_image_bytes: None,
            },
            model_routes: vec![make_vision_route()],
            ..Config::default()
        };
        assert!(config.validate_multimodal_config().is_ok());
    }

    #[test]
    fn multimodal_validation_warns_but_passes_for_non_mvp_channels() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: true,
                allowed_channels: vec!["slack".into()],
                vision_model_hint: Some("vision".into()),
                max_image_bytes: None,
            },
            model_routes: vec![make_vision_route()],
            ..Config::default()
        };
        // Non-MVP channels warn but don't reject (fail-closed at runtime per ADR-4)
        assert!(config.validate_multimodal_config().is_ok());
    }

    // ── max_image_bytes config validation (task 2.10) ────────

    #[test]
    fn multimodal_max_image_bytes_zero_rejected() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                max_image_bytes: Some(0),
                ..Default::default()
            },
            ..Config::default()
        };
        let err = config.validate_multimodal_config().unwrap_err();
        assert!(
            err.to_string().contains("greater than 0"),
            "expected 'greater than 0' error, got: {err}"
        );
    }

    #[test]
    fn multimodal_max_image_bytes_exceeds_ceiling_rejected() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                max_image_bytes: Some(104_857_600), // 100 MiB
                ..Default::default()
            },
            ..Config::default()
        };
        let err = config.validate_multimodal_config().unwrap_err();
        assert!(
            err.to_string().contains("50 MiB ceiling"),
            "expected '50 MiB ceiling' error, got: {err}"
        );
    }

    #[test]
    fn multimodal_max_image_bytes_valid_value_accepted() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                max_image_bytes: Some(5_242_880), // 5 MiB
                ..Default::default()
            },
            ..Config::default()
        };
        assert!(config.validate_multimodal_config().is_ok());
    }

    #[test]
    fn multimodal_max_image_bytes_none_accepted() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                max_image_bytes: None,
                ..Default::default()
            },
            ..Config::default()
        };
        assert!(config.validate_multimodal_config().is_ok());
    }

    #[test]
    fn multimodal_max_image_bytes_at_ceiling_accepted() {
        let config = Config {
            multimodal: MultimodalConfig {
                enabled: false,
                max_image_bytes: Some(52_428_800), // exactly 50 MiB
                ..Default::default()
            },
            ..Config::default()
        };
        assert!(config.validate_multimodal_config().is_ok());
    }

    // ── MCP capabilities config ─────────────────────────────

    #[test]
    fn default_mcp_capabilities_returns_tools() {
        let caps = default_mcp_capabilities();
        assert_eq!(caps, vec!["tools".to_string()]);
    }

    #[test]
    fn mcp_server_config_default_has_capabilities_tools() {
        let server = McpServerConfig::default();
        assert_eq!(server.capabilities, vec!["tools".to_string()]);
        assert!(server.resource_output_limit_bytes.is_none());
        assert!(server.prompt_output_limit_bytes.is_none());
    }

    #[test]
    fn mcp_config_deser_missing_capabilities_defaults_to_tools() {
        let json = r#"{
            "name": "docs",
            "command": "__mcp_mock__"
        }"#;
        let server: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(server.capabilities, vec!["tools".to_string()]);
    }

    #[test]
    fn mcp_config_deser_explicit_capabilities_preserved() {
        let json = r#"{
            "name": "docs",
            "command": "__mcp_mock__",
            "capabilities": ["tools", "resources"]
        }"#;
        let server: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(server.capabilities, vec!["tools", "resources"]);
    }

    #[test]
    fn mcp_validation_rejects_unknown_capability() {
        let result = Config::validate_mcp_capabilities(
            &["tools".to_string(), "subscriptions".to_string()],
            "mcp.servers[0]",
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("subscriptions"));
        assert!(err.contains("unrecognized"));
    }

    #[test]
    fn mcp_validation_rejects_empty_capabilities() {
        let result = Config::validate_mcp_capabilities(&[], "mcp.servers[0]");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least one"));
    }

    #[test]
    fn mcp_validation_rejects_duplicate_capabilities() {
        let result = Config::validate_mcp_capabilities(
            &["tools".to_string(), "tools".to_string()],
            "mcp.servers[0]",
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn mcp_validation_accepts_tools_only() {
        let result = Config::validate_mcp_capabilities(&["tools".to_string()], "mcp.servers[0]");
        assert!(result.is_ok());
    }

    #[test]
    fn mcp_validation_accepts_tools_and_resources() {
        let result = Config::validate_mcp_capabilities(
            &["tools".to_string(), "resources".to_string()],
            "mcp.servers[0]",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn mcp_validation_accepts_all_three_capabilities() {
        let result = Config::validate_mcp_capabilities(
            &[
                "tools".to_string(),
                "resources".to_string(),
                "prompts".to_string(),
            ],
            "mcp.servers[0]",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn mcp_validation_rejects_zero_resource_output_limit() {
        let server = McpServerConfig {
            name: "test".to_string(),
            command: "__mcp_mock__".to_string(),
            resource_output_limit_bytes: Some(0),
            ..McpServerConfig::default()
        };
        let result = Config::validate_mcp_capability_limits(&server, "mcp.servers[0]");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("resource_output_limit_bytes"));
        assert!(err.contains("greater than zero"));
    }

    #[test]
    fn mcp_validation_rejects_excessive_prompt_output_limit() {
        let server = McpServerConfig {
            name: "test".to_string(),
            command: "__mcp_mock__".to_string(),
            prompt_output_limit_bytes: Some(20 * 1024 * 1024), // 20MB > 10MB max
            ..McpServerConfig::default()
        };
        let result = Config::validate_mcp_capability_limits(&server, "mcp.servers[0]");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("prompt_output_limit_bytes"));
        assert!(err.contains("10MB"));
    }

    #[test]
    fn mcp_validation_accepts_valid_capability_limits() {
        let server = McpServerConfig {
            name: "test".to_string(),
            command: "__mcp_mock__".to_string(),
            resource_output_limit_bytes: Some(128 * 1024),
            prompt_output_limit_bytes: Some(64 * 1024),
            ..McpServerConfig::default()
        };
        let result = Config::validate_mcp_capability_limits(&server, "mcp.servers[0]");
        assert!(result.is_ok());
    }

    // ── SandboxConfig.require field (T1) ────────────────────

    #[test]
    fn sandbox_config_default_require_is_false() {
        let config = SandboxConfig::default();
        assert!(!config.require, "require must default to false");
    }

    #[test]
    fn sandbox_config_require_serde_roundtrip() {
        let config = SandboxConfig {
            enabled: Some(true),
            backend: SandboxBackend::Auto,
            require: true,
            firejail_args: Vec::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SandboxConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.require, "require=true must survive serde roundtrip");
    }

    #[test]
    fn sandbox_config_missing_require_defaults_to_false() {
        let json = r#"{"enabled":true,"backend":"auto"}"#;
        let parsed: SandboxConfig = serde_json::from_str(json).unwrap();
        assert!(
            !parsed.require,
            "missing require field must default to false"
        );
    }

    // ── AudioConfig tests (Task 1.2 — audio-input-support) ──

    #[test]
    fn audio_config_empty_toml_section_uses_defaults() {
        let toml_str = "";
        let parsed: AudioConfig = toml::from_str(toml_str).unwrap();
        assert!(!parsed.enabled);
        assert!(parsed.allowed_channels.is_empty());
        assert_eq!(parsed.max_audio_bytes, 26_214_400);
        assert_eq!(parsed.max_audio_duration_secs, 600);
        assert_eq!(parsed.transcription_model, "base");
        assert_eq!(parsed.transcription_language, "es");
        assert_eq!(parsed.whisper_binary, "whisper-cli");
        assert_eq!(parsed.max_concurrent_transcriptions, 1);
        assert_eq!(parsed.transcription_timeout_secs, 120);
    }

    #[test]
    fn audio_config_full_toml_roundtrip() {
        let toml_str = r#"
enabled = true
allowed_channels = ["telegram"]
max_audio_bytes = 5242880
max_audio_duration_secs = 300
transcription_model = "small"
transcription_language = "en"
whisper_binary = "/usr/local/bin/whisper-cli"
max_concurrent_transcriptions = 2
transcription_timeout_secs = 60
"#;
        let parsed: AudioConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.allowed_channels, vec!["telegram".to_string()]);
        assert_eq!(parsed.max_audio_bytes, 5_242_880);
        assert_eq!(parsed.max_audio_duration_secs, 300);
        assert_eq!(parsed.transcription_model, "small");
        assert_eq!(parsed.transcription_language, "en");
        assert_eq!(parsed.whisper_binary, "/usr/local/bin/whisper-cli");
        assert_eq!(parsed.max_concurrent_transcriptions, 2);
        assert_eq!(parsed.transcription_timeout_secs, 60);
    }

    #[test]
    fn audio_config_default_impl_matches_documented_defaults() {
        let cfg = AudioConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.allowed_channels.is_empty());
        assert_eq!(cfg.max_audio_bytes, 26_214_400);
        assert_eq!(cfg.max_audio_duration_secs, 600);
        assert_eq!(cfg.transcription_model, "base");
        assert_eq!(cfg.transcription_language, "es");
        assert_eq!(cfg.whisper_binary, "whisper-cli");
        assert_eq!(cfg.max_concurrent_transcriptions, 1);
        assert_eq!(cfg.transcription_timeout_secs, 120);
    }

    #[test]
    fn config_with_no_audio_section_gets_default_audio() {
        let config = Config::default();
        assert!(!config.audio.enabled);
        assert!(config.audio.allowed_channels.is_empty());
    }

    // ── Audio config validation tests (Task 1.3 — audio-input-support) ──

    #[test]
    fn audio_validation_passes_when_disabled() {
        let config = Config {
            audio: AudioConfig {
                enabled: false,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_audio_config().is_ok());
    }

    #[test]
    fn audio_validation_rejects_enabled_with_empty_channels() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: Vec::new(),
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("allowed_channels"),
            "expected 'allowed_channels' error, got: {err}"
        );
        assert_eq!(
            err.to_string(),
            "audio.allowed_channels must be non-empty when audio is enabled"
        );
    }

    #[test]
    fn audio_validation_passes_with_valid_config() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into()],
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_audio_config().is_ok());
    }

    #[test]
    fn audio_validation_accepts_minimum_concurrency_and_timeout_boundaries() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into()],
                max_concurrent_transcriptions: 1,
                transcription_timeout_secs: 1,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_audio_config().is_ok());
    }

    #[test]
    fn audio_validation_rejects_zero_max_audio_bytes() {
        let config = Config {
            audio: AudioConfig {
                max_audio_bytes: 0,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("greater than 0"),
            "expected 'greater than 0' error, got: {err}"
        );
    }

    #[test]
    fn audio_validation_rejects_bytes_exceeding_ceiling() {
        let config = Config {
            audio: AudioConfig {
                max_audio_bytes: 200 * 1024 * 1024, // 200 MiB
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("100 MiB ceiling"),
            "expected '100 MiB ceiling' error, got: {err}"
        );
    }

    #[test]
    fn audio_validation_rejects_zero_duration() {
        let config = Config {
            audio: AudioConfig {
                max_audio_duration_secs: 0,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("greater than 0"),
            "expected 'greater than 0' error, got: {err}"
        );
    }

    #[test]
    fn audio_validation_rejects_duration_exceeding_ceiling() {
        let config = Config {
            audio: AudioConfig {
                max_audio_duration_secs: 7200, // 2 hours
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("1 hour ceiling"),
            "expected '1 hour ceiling' error, got: {err}"
        );
    }

    #[test]
    fn audio_validation_warns_but_passes_for_non_phase1_channels() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into(), "discord".into()],
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        // Non-Phase-1 channels warn but don't reject
        assert!(config.validate_audio_config().is_ok());
    }

    #[test]
    fn audio_validation_accepts_bytes_at_ceiling() {
        let config = Config {
            audio: AudioConfig {
                max_audio_bytes: MAX_AUDIO_BYTES_CEILING, // exactly 100 MiB
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_audio_config().is_ok());
    }

    #[test]
    fn audio_validation_accepts_duration_at_ceiling() {
        let config = Config {
            audio: AudioConfig {
                max_audio_duration_secs: MAX_AUDIO_DURATION_SECS_CEILING, // exactly 1 hour
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(config.validate_audio_config().is_ok());
    }

    // ── AudioConfig default values (coverage) ────────────────

    #[test]
    fn audio_config_default_values_are_correct() {
        let ac = AudioConfig::default();
        assert!(!ac.enabled);
        assert!(ac.allowed_channels.is_empty());
        assert_eq!(ac.max_audio_bytes, 26_214_400); // 25 MiB
        assert_eq!(ac.max_audio_duration_secs, 600); // 10 min
        assert_eq!(ac.transcription_model, "base");
        assert_eq!(ac.transcription_language, "es");
        assert_eq!(ac.whisper_binary, "whisper-cli");
        assert_eq!(ac.max_concurrent_transcriptions, 1);
        assert_eq!(ac.transcription_timeout_secs, 120);
    }

    // ── AudioConfig serde deserialization ─────────────────────

    #[test]
    fn audio_config_toml_deserialization_with_all_fields() {
        let toml_str = r#"
default_temperature = 0.7

[audio]
enabled = true
allowed_channels = ["telegram"]
max_audio_bytes = 52428800
max_audio_duration_secs = 300
transcription_model = "large-v3"
transcription_language = "en"
whisper_binary = "/usr/local/bin/whisper-cli"
max_concurrent_transcriptions = 4
transcription_timeout_secs = 60
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(parsed.audio.enabled);
        assert_eq!(parsed.audio.allowed_channels, vec!["telegram"]);
        assert_eq!(parsed.audio.max_audio_bytes, 52_428_800);
        assert_eq!(parsed.audio.max_audio_duration_secs, 300);
        assert_eq!(parsed.audio.transcription_model, "large-v3");
        assert_eq!(parsed.audio.transcription_language, "en");
        assert_eq!(parsed.audio.whisper_binary, "/usr/local/bin/whisper-cli");
        assert_eq!(parsed.audio.max_concurrent_transcriptions, 4);
        assert_eq!(parsed.audio.transcription_timeout_secs, 60);
    }

    #[test]
    fn audio_config_toml_missing_optional_fields_use_defaults() {
        let toml_str = r#"
default_temperature = 0.7

[audio]
enabled = true
allowed_channels = ["telegram"]
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(parsed.audio.enabled);
        assert_eq!(parsed.audio.allowed_channels, vec!["telegram"]);
        // All other fields should fall back to defaults
        assert_eq!(parsed.audio.max_audio_bytes, 26_214_400);
        assert_eq!(parsed.audio.max_audio_duration_secs, 600);
        assert_eq!(parsed.audio.transcription_model, "base");
        assert_eq!(parsed.audio.transcription_language, "es");
        assert_eq!(parsed.audio.whisper_binary, "whisper-cli");
        assert_eq!(parsed.audio.max_concurrent_transcriptions, 1);
        assert_eq!(parsed.audio.transcription_timeout_secs, 120);
    }

    #[test]
    fn audio_config_toml_no_section_gets_defaults() {
        let toml_str = r#"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(toml_str).unwrap();
        assert!(!parsed.audio.enabled);
        assert!(parsed.audio.allowed_channels.is_empty());
        assert_eq!(parsed.audio.max_audio_bytes, 26_214_400);
        assert_eq!(parsed.audio.transcription_model, "base");
    }

    #[test]
    fn audio_config_serde_roundtrip() {
        let ac = AudioConfig {
            enabled: true,
            allowed_channels: vec!["telegram".into(), "discord".into()],
            max_audio_bytes: 10_000_000,
            max_audio_duration_secs: 120,
            transcription_model: "small".into(),
            transcription_language: "fr".into(),
            whisper_binary: "/opt/whisper".into(),
            max_concurrent_transcriptions: 2,
            transcription_timeout_secs: 90,
        };
        let toml_str = toml::to_string(&ac).unwrap();
        let parsed: AudioConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.allowed_channels, vec!["telegram", "discord"]);
        assert_eq!(parsed.max_audio_bytes, 10_000_000);
        assert_eq!(parsed.max_audio_duration_secs, 120);
        assert_eq!(parsed.transcription_model, "small");
        assert_eq!(parsed.transcription_language, "fr");
        assert_eq!(parsed.whisper_binary, "/opt/whisper");
        assert_eq!(parsed.max_concurrent_transcriptions, 2);
        assert_eq!(parsed.transcription_timeout_secs, 90);
    }

    // ── AudioConfig validation — concurrency/timeout zero ────

    #[test]
    fn audio_validation_rejects_zero_concurrent_transcriptions() {
        let config = Config {
            audio: AudioConfig {
                max_concurrent_transcriptions: 0,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("max_concurrent_transcriptions")
                && err.to_string().contains("greater than 0"),
            "expected concurrent transcriptions error, got: {err}"
        );
    }

    #[test]
    fn audio_validation_rejects_zero_transcription_timeout() {
        let config = Config {
            audio: AudioConfig {
                transcription_timeout_secs: 0,
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        let err = config.validate_audio_config().expect_err("should fail");
        assert!(
            err.to_string().contains("transcription_timeout_secs")
                && err.to_string().contains("greater than 0"),
            "expected transcription timeout error, got: {err}"
        );
    }

    // ── VALID_AUDIO_CHANNELS expansion (T1.6) ────────────────

    #[test]
    fn audio_validation_accepts_gateway_channel() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["gateway".into()],
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        // "gateway" is now a recognised channel — validation must pass without warning
        assert!(
            config.validate_audio_config().is_ok(),
            "gateway should be a recognised audio channel"
        );
    }

    #[test]
    fn audio_validation_accepts_cli_channel() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["cli".into()],
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        // "cli" is now a recognised channel — validation must pass without warning
        assert!(
            config.validate_audio_config().is_ok(),
            "cli should be a recognised audio channel"
        );
    }

    #[test]
    fn audio_validation_accepts_all_known_channels_together() {
        let config = Config {
            audio: AudioConfig {
                enabled: true,
                allowed_channels: vec!["telegram".into(), "gateway".into(), "cli".into()],
                ..AudioConfig::default()
            },
            ..Config::default()
        };
        assert!(
            config.validate_audio_config().is_ok(),
            "all three recognised channels together must pass validation"
        );
    }
}
