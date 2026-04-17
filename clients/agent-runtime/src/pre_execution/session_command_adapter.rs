use crate::pre_execution::{BlockingOutcome, IngressDecision};
use crate::session_commands::{
    SessionCommandFailure, SessionCommandFailureKind, SessionCommandOutcome, SessionCommandSuccess,
};

#[derive(Debug, Clone, PartialEq)]
pub enum HandledIngress {
    NotHandled,
    Handled(HandledIngressOutcome),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HandledIngressOutcome {
    SessionCommandSuccess(SessionCommandSuccess),
    SessionCommandFailure {
        class: SessionCommandFailureClass,
        failure: SessionCommandFailure,
    },
    Blocking(BlockingOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommandFailureClass {
    PermissionDenied,
    Failed,
}

pub fn adapt_handled_ingress(decision: IngressDecision) -> HandledIngress {
    match decision {
        IngressDecision::Continue => HandledIngress::NotHandled,
        IngressDecision::Blocking(blocking) => {
            HandledIngress::Handled(HandledIngressOutcome::Blocking(blocking))
        }
        IngressDecision::SessionCommand { outcome } => match outcome {
            SessionCommandOutcome::Success(success) => {
                HandledIngress::Handled(HandledIngressOutcome::SessionCommandSuccess(success))
            }
            SessionCommandOutcome::Failure(failure) => {
                HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
                    class: classify_session_command_failure(failure.kind.clone()),
                    failure,
                })
            }
        },
    }
}

fn classify_session_command_failure(kind: SessionCommandFailureKind) -> SessionCommandFailureClass {
    match kind {
        SessionCommandFailureKind::MissingCallerScope
        | SessionCommandFailureKind::PermissionDenied => {
            SessionCommandFailureClass::PermissionDenied
        }
        SessionCommandFailureKind::UnsupportedBackend
        | SessionCommandFailureKind::UnknownSession
        | SessionCommandFailureKind::InvalidState
        | SessionCommandFailureKind::MissingSnapshot
        | SessionCommandFailureKind::InvalidResumeTarget
        | SessionCommandFailureKind::InvalidArguments
        | SessionCommandFailureKind::StorageFailure => SessionCommandFailureClass::Failed,
    }
}
