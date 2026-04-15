use crate::memory::ResumableSessionEntry;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlashCommandRequirements {
    pub capability_tags: Vec<&'static str>,
    pub permission_tags: Vec<&'static str>,
    pub backend_tags: Vec<&'static str>,
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

#[derive(Debug, Clone)]
pub struct CommandContext<'a> {
    pub session_id: &'a str,
    pub caller_token_hash: Option<&'a str>,
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
        context: CommandContext<'_>,
        invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError>;
}

#[derive(Clone)]
pub struct SlashCommandRegistration {
    pub descriptor: SlashCommandDescriptor,
    pub handler: Arc<dyn SlashCommandHandler>,
}

#[derive(Debug, Clone)]
pub struct SessionCommandResult {
    pub command: &'static str,
    pub session_id: String,
    pub message: String,
    pub resumed_session_id: Option<String>,
    pub resumable_sessions: Vec<ResumableSessionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommandError {
    UnsupportedBackend {
        backend: String,
    },
    UnknownSession {
        session_id: String,
    },
    InvalidState {
        session_id: String,
        detail: &'static str,
    },
    MissingSnapshot {
        session_id: String,
    },
    InvalidResumeTarget {
        session_id: String,
    },
    InvalidArguments {
        command: &'static str,
        detail: String,
    },
    Unauthorized,
    StorageFailure {
        detail: String,
    },
}

impl SessionCommandError {
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedBackend { backend } => {
                format!("slash-session commands require sqlite memory backend (backend={backend})")
            }
            Self::UnknownSession { session_id } => {
                format!("[session:{session_id}] unknown session")
            }
            Self::InvalidState { session_id, detail } => {
                format!("[session:{session_id}] {detail}")
            }
            Self::MissingSnapshot { session_id } => {
                format!("[session:{session_id}] missing resume-capable compact snapshot")
            }
            Self::InvalidResumeTarget { session_id } => {
                format!("[session:{session_id}] invalid resume target")
            }
            Self::InvalidArguments { command, detail } => {
                format!("invalid slash command usage for {command}: {detail}")
            }
            Self::Unauthorized => "unauthorized: verifiable caller identity required".to_string(),
            Self::StorageFailure { detail } => {
                format!("slash-session storage failure: {detail}")
            }
        }
    }
}
