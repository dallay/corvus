use crate::memory::ResumableSessionEntry;

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
        }
    }
}
