use crate::memory::ResumableSessionEntry;

/// Sanitize storage errors for user-facing messages.
/// Strips sensitive information like file paths and connection strings.
pub(crate) fn sanitize_storage_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string();

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

    // If the sanitized message is too different, return a generic message
    if sanitized.len() > 100 && sanitized != error_str {
        return "storage unavailable".to_string();
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

    sanitized
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSlashCommand {
    Resume {
        target: Option<String>,
        args: String,
    },
    Suspend,
    Tldr,
    Compact {
        args: String,
    },
}

impl SessionSlashCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Resume { .. } => "/resume",
            Self::Suspend => "/suspend",
            Self::Tldr => "/tldr",
            Self::Compact { .. } => "/compact",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandContext<'a> {
    pub session_id: &'a str,
    pub caller_token_hash: Option<&'a str>,
    pub command: SessionSlashCommand,
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
            Self::StorageFailure { detail } => {
                format!("slash-session storage failure: {detail}")
            }
        }
    }
}
