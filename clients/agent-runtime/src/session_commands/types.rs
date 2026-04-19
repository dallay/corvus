use crate::config::ExecutionMode;
use crate::memory::{
    ResumableSessionEntry, SessionListEntry, SessionSnapshotKind, SessionStatus,
    SlashSessionLifecycle,
};
use std::sync::Arc;

/// Sanitize storage errors for user-facing messages.
/// Strips sensitive information like file paths and connection strings.
/// The detailed error is always logged internally; callers receive a fixed
/// public message to prevent leaking backend internals.
pub(crate) fn sanitize_storage_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string();

    // Always log the full error detail internally for debugging
    tracing::debug!(error = %error_str, "storage error (internal log)");

    // List of patterns that may contain sensitive information
    let sensitive_patterns = [
        // File paths that may leak system info
        r"/[a-zA-Z0-9_/.-]+",
        // Connection strings that may contain credentials
        r"sqlite://[^\s]+",
        r"postgresql://[^\s]+",
        r"mysql://[^\s]+",
    ];

    let mut sanitized = error_str.clone();
    for pattern in sensitive_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            sanitized = re.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }

    // Check for common error types and map to user-friendly messages
    let lower = sanitized.to_lowercase();
    if lower.contains("no such file") || lower.contains("not found") {
        return "storage not found".to_string();
    }
    if lower.contains("permission") || lower.contains("access denied") {
        return "storage access denied".to_string();
    }
    if lower.contains("locked") || lower.contains("busy") {
        return "storage is busy".to_string();
    }

    // Final fallback: never return raw sanitized text to callers
    "storage unavailable".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandArgumentShape {
    None,
    OptionalText,
    OptionalTargetThenText,
    RequiredText,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlashCommandRequirements {
    pub capabilities: &'static [CommandCapability],
    pub permissions: &'static [CommandPermission],
    pub backends: &'static [CommandBackend],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCapability {
    SessionLifecycle,
    SessionSummary,
    SessionRead,
    SettingsRead,
    SettingsWrite,
    McpManagement,
    ToolManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPermission {
    RequiresCallerScope,
    RequiresResumableSessionVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBackend {
    SqliteSlashSessions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandDescriptor {
    pub canonical_name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub argument_shape: SlashCommandArgumentShape,
    pub requirements: SlashCommandRequirements,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSlashInvocation {
    pub invoked_name: String,
    pub raw_args: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashInvocation {
    pub invoked_name: String,
    pub canonical_name: &'static str,
    pub raw_args: String,
    pub primary_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContext {
    pub session: CommandSessionContext,
    pub caller: CommandCaller,
    pub ingress: CommandIngressContext,
    pub facts: CommandContextFacts,
}

impl CommandContext {
    pub fn new(
        session: CommandSessionContext,
        caller: CommandCaller,
        ingress: CommandIngressContext,
    ) -> Self {
        let facts = CommandContextFacts {
            has_caller_scope: caller.scope_key().is_some(),
        };

        Self {
            session,
            caller,
            ingress,
            facts,
        }
    }

    pub fn for_cli(
        session_id: impl Into<String>,
        session_source: CommandSessionSource,
        execution_mode: ExecutionMode,
        scope_key: Option<String>,
    ) -> Self {
        let caller = scope_key
            .map(|scope_key| CommandCaller::DerivedCliScope { scope_key })
            .unwrap_or(CommandCaller::Unavailable);

        Self::new(
            CommandSessionContext {
                session_id: session_id.into(),
                source: session_source,
            },
            caller,
            CommandIngressContext {
                source: CommandIngressSource::Cli,
                execution_mode,
            },
        )
    }

    pub fn for_gateway_http(
        session_id: impl Into<String>,
        session_source: CommandSessionSource,
        execution_mode: ExecutionMode,
        caller_scope_key: Option<String>,
    ) -> Self {
        Self::new(
            CommandSessionContext {
                session_id: session_id.into(),
                source: session_source,
            },
            verified_or_unavailable(caller_scope_key),
            CommandIngressContext {
                source: CommandIngressSource::GatewayHttp,
                execution_mode,
            },
        )
    }

    pub fn for_gateway_stream(
        session_id: impl Into<String>,
        session_source: CommandSessionSource,
        execution_mode: ExecutionMode,
        caller_scope_key: Option<String>,
    ) -> Self {
        Self::new(
            CommandSessionContext {
                session_id: session_id.into(),
                source: session_source,
            },
            verified_or_unavailable(caller_scope_key),
            CommandIngressContext {
                source: CommandIngressSource::GatewayStream,
                execution_mode,
            },
        )
    }

    pub fn for_webhook(
        session_id: impl Into<String>,
        session_source: CommandSessionSource,
        execution_mode: ExecutionMode,
        caller_scope_key: Option<String>,
    ) -> Self {
        Self::new(
            CommandSessionContext {
                session_id: session_id.into(),
                source: session_source,
            },
            verified_or_unavailable(caller_scope_key),
            CommandIngressContext {
                source: CommandIngressSource::Webhook,
                execution_mode,
            },
        )
    }

    pub fn for_channel(
        session_id: impl Into<String>,
        session_source: CommandSessionSource,
        execution_mode: ExecutionMode,
        channel: impl Into<String>,
        caller_scope_key: Option<String>,
    ) -> Self {
        let channel = channel.into();
        let caller = caller_scope_key
            .map(|scope_key| CommandCaller::DerivedChannelScope {
                channel: channel.clone(),
                scope_key,
            })
            .unwrap_or(CommandCaller::Unavailable);

        Self::new(
            CommandSessionContext {
                session_id: session_id.into(),
                source: session_source,
            },
            caller,
            CommandIngressContext {
                source: CommandIngressSource::Channel { name: channel },
                execution_mode,
            },
        )
    }
}

fn verified_or_unavailable(caller_scope_key: Option<String>) -> CommandCaller {
    caller_scope_key
        .map(|scope_key| CommandCaller::VerifiedTokenHash { scope_key })
        .unwrap_or(CommandCaller::Unavailable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSessionContext {
    pub session_id: String,
    pub source: CommandSessionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSessionSource {
    Existing,
    Explicit,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIngressContext {
    pub source: CommandIngressSource,
    pub execution_mode: ExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandIngressSource {
    Cli,
    GatewayHttp,
    GatewayStream,
    Webhook,
    Channel { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandCaller {
    VerifiedTokenHash { scope_key: String },
    DerivedCliScope { scope_key: String },
    DerivedChannelScope { channel: String, scope_key: String },
    Unavailable,
}

impl CommandCaller {
    pub fn scope_key(&self) -> Option<&str> {
        match self {
            Self::VerifiedTokenHash { scope_key }
            | Self::DerivedCliScope { scope_key }
            | Self::DerivedChannelScope { scope_key, .. } => Some(scope_key.as_str()),
            Self::Unavailable => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContextFacts {
    pub has_caller_scope: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashRegistryError {
    InvalidName {
        name: String,
    },
    EmptyDescription {
        canonical_name: String,
    },
    DuplicateCanonicalName {
        canonical_name: String,
    },
    DuplicateAlias {
        alias: String,
        existing_canonical_name: String,
    },
    AliasCollidesWithCanonical {
        alias: String,
        canonical_name: String,
    },
}

#[async_trait::async_trait]
pub trait SlashCommandHandler: Send + Sync {
    async fn handle(
        &self,
        service: &super::service::SessionCommandService<'_>,
        context: CommandContext,
        invocation: SlashInvocation,
    ) -> SessionCommandOutcome;
}

#[derive(Clone)]
pub struct SlashCommandRegistration {
    pub descriptor: SlashCommandDescriptor,
    pub handler: Arc<dyn SlashCommandHandler>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionCommandOutcome {
    Success(SessionCommandSuccess),
    Failure(SessionCommandFailure),
}

impl SessionCommandOutcome {
    pub fn message(&self) -> &str {
        match self {
            Self::Success(success) => success.message.as_str(),
            Self::Failure(failure) => failure.message.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionCommandSuccess {
    pub command: &'static str,
    pub session_id: String,
    pub message: String,
    pub data: SessionCommandSuccessData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandToolEntry {
    pub name: String,
    pub description: String,
    pub source_kind: SessionCommandToolSourceKind,
    pub source_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandHelpEntry {
    pub name: &'static str,
    pub usage: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandSessionStatus {
    pub session_id: String,
    pub current_session_known: bool,
    pub session_status: Option<crate::memory::SessionStatus>,
    pub slash_lifecycle: Option<crate::memory::SlashSessionLifecycle>,
    pub started_at: Option<String>,
    pub last_activity: Option<String>,
    pub ended_at: Option<String>,
    pub message_count: Option<u32>,
    pub has_tldr_snapshot: Option<bool>,
    pub has_compact_snapshot: Option<bool>,
    pub resume_hydration_pending: Option<bool>,
    pub suspended_at: Option<String>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionCommandSessionInspect {
    pub session_id: String,
    pub current_session_known: bool,
    pub session: Option<SessionCommandInspectSessionRecord>,
    pub state: Option<SessionCommandInspectStateRecord>,
    pub snapshots: SessionCommandInspectSnapshots,
    pub gaps: Vec<SessionCommandInspectGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandInspectSessionRecord {
    pub status: SessionStatus,
    pub started_at: String,
    pub last_activity: String,
    pub ended_at: Option<String>,
    pub message_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandInspectStateRecord {
    pub lifecycle: SlashSessionLifecycle,
    pub latest_tldr_snapshot_id: Option<String>,
    pub latest_compact_snapshot_id: Option<String>,
    pub pending_hydration_snapshot_id: Option<String>,
    pub suspended_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionCommandInspectSnapshots {
    pub latest_tldr: SessionCommandInspectSnapshotSlot,
    pub latest_compact: SessionCommandInspectSnapshotSlot,
    pub pending_hydration: SessionCommandInspectSnapshotSlot,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionCommandInspectSnapshotSlot {
    pub reference_id: Option<String>,
    pub snapshot: Option<SessionCommandInspectSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionCommandInspectSnapshot {
    pub snapshot_id: String,
    pub kind: SessionSnapshotKind,
    pub created_at: String,
    pub resume_capable: bool,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandInspectGap {
    pub code: SessionCommandInspectGapCode,
    pub reference_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommandInspectGapCode {
    SlashSessionStateMissing,
    SnapshotUnavailableWithoutState,
    ReferencedSnapshotMissing,
    ReferencedSnapshotOwnershipMismatch,
    ReferencedSnapshotKindMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommandToolSourceKind {
    Native,
    McpTool,
    McpResource,
    McpPrompt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionCommandSuccessData {
    None,
    SessionHelp {
        entries: Vec<SessionCommandHelpEntry>,
    },
    SessionStatus {
        status: SessionCommandSessionStatus,
    },
    SessionInspect {
        inspect: Box<SessionCommandSessionInspect>,
    },
    SessionList {
        sessions: Vec<SessionListEntry>,
    },
    Resumed {
        resumed_session_id: String,
    },
    ResumableSessions {
        sessions: Vec<ResumableSessionEntry>,
    },
    ToolListing {
        tools: Vec<SessionCommandToolEntry>,
    },
    ModelInfo {
        current: String,
        available: Vec<String>,
    },
    ProviderInfo {
        current: String,
        available: Vec<String>,
    },
    TemperatureInfo {
        current: f32,
    },
    McpList {
        servers: Vec<String>,
    },
    McpAdded {
        server: String,
    },
    McpRemoved {
        server: String,
    },
    ToolEnabled {
        name: String,
    },
    ToolDisabled {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCommandFailure {
    pub command: &'static str,
    pub kind: SessionCommandFailureKind,
    pub session_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommandFailureKind {
    UnsupportedBackend,
    UnknownSession,
    InvalidState,
    MissingSnapshot,
    InvalidResumeTarget,
    InvalidArguments,
    MissingCallerScope,
    PermissionDenied,
    StorageFailure,
}
