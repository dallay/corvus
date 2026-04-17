use crate::agent::unified_entrypoint::{self, CanonicalOutcome};
use crate::memory::Memory;
use crate::session_commands::{
    default_registry, CommandContext, SessionCommandOutcome, SessionCommandService,
};

mod session_command_adapter;

pub use session_command_adapter::{
    adapt_handled_ingress, HandledIngress, HandledIngressOutcome, SessionCommandFailureClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockingOutcome {
    ApprovalRequired { tool: String, reason: String },
    TimeoutAborted,
    Fallback { response: String },
}

#[derive(Debug, Clone)]
pub enum IngressDecision {
    Continue,
    Blocking(BlockingOutcome),
    SessionCommand { outcome: SessionCommandOutcome },
}

pub fn approval_granted_from_env() -> bool {
    std::env::var("CORVUS_UNIFIED_APPROVE").as_deref() == Ok("1")
}

pub fn test_triggers_enabled() -> bool {
    cfg!(test) || std::env::var("CORVUS_UNIFIED_TEST_TRIGGERS").as_deref() == Ok("1")
}

pub async fn evaluate(session_id: String, prompt: &str) -> CanonicalOutcome {
    unified_entrypoint::run_canonical_outcome(
        session_id,
        prompt,
        approval_granted_from_env(),
        unified_entrypoint::CanonicalOutcomeConfig {
            enable_test_triggers: test_triggers_enabled(),
        },
    )
    .await
}

pub async fn evaluate_ingress(
    memory: &dyn Memory,
    context: CommandContext,
    prompt: &str,
    include_blocking_fallback: bool,
) -> IngressDecision {
    let service = SessionCommandService::new(memory);
    let session_id = context.session.session_id.clone();
    if let Some(outcome) = default_registry().dispatch(&service, context, prompt).await {
        return IngressDecision::SessionCommand { outcome };
    }

    if !include_blocking_fallback {
        return IngressDecision::Continue;
    }

    let canonical = evaluate(session_id.to_string(), prompt).await;
    match classify_blocking(&canonical) {
        Some(blocking) => IngressDecision::Blocking(blocking),
        None => IngressDecision::Continue,
    }
}

pub fn classify_blocking(outcome: &CanonicalOutcome) -> Option<BlockingOutcome> {
    if let Some(tool) = &outcome.approval_required {
        return Some(BlockingOutcome::ApprovalRequired {
            tool: tool.clone(),
            reason: outcome
                .approval_reason
                .clone()
                .unwrap_or_else(|| format!("approval required for `{tool}`")),
        });
    }

    if outcome.timeout_aborted {
        return Some(BlockingOutcome::TimeoutAborted);
    }

    outcome
        .fallback_response
        .as_ref()
        .map(|response| BlockingOutcome::Fallback {
            response: response.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionMode;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::session_commands::{
        CommandCaller, CommandIngressSource, CommandSessionSource, SessionCommandFailure,
        SessionCommandFailureKind, SessionCommandOutcome, SessionCommandSuccess,
        SessionCommandSuccessData,
    };
    use async_trait::async_trait;

    struct IngressMemory;

    #[async_trait]
    impl Memory for IngressMemory {
        fn name(&self) -> &str {
            "none"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[test]
    fn classify_prefers_approval_required() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: Some("tool-1".to_string()),
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: Some("fallback".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::ApprovalRequired {
                tool: "tool-1".to_string(),
                reason: "approval required for `tool-1`".to_string(),
            })
        );
    }

    #[test]
    fn classify_identifies_timeout() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: None,
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::TimeoutAborted)
        );
    }

    #[test]
    fn classify_identifies_fallback() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: false,
            fallback_response: Some("fallback response".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::Fallback {
                response: "fallback response".to_string(),
            })
        );
    }

    #[test]
    fn classify_returns_none_when_nothing_blocks() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: false,
            fallback_response: None,
        };

        assert_eq!(classify_blocking(&outcome), None);
    }

    #[test]
    fn classify_prefers_timeout_over_fallback_when_no_approval() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: true,
            fallback_response: Some("fallback response".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::TimeoutAborted)
        );
    }

    #[tokio::test]
    async fn ingress_classifies_supported_slash_commands_before_pre_execution() {
        let decision = evaluate_ingress(
            &IngressMemory,
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/tldr",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert_eq!(
                    outcome,
                    SessionCommandOutcome::Failure(crate::session_commands::SessionCommandFailure {
                        command: "/tldr",
                        kind: SessionCommandFailureKind::UnsupportedBackend,
                        session_id: Some("session-1".to_string()),
                        message:
                            "slash-session commands require sqlite memory backend (backend=none)"
                                .to_string(),
                    })
                );
            }
            other => panic!("expected session command interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_preserves_unknown_slash_like_input() {
        let decision = evaluate_ingress(
            &IngressMemory,
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/resume-later",
            true,
        )
        .await;

        assert!(matches!(decision, IngressDecision::Continue));
    }

    #[tokio::test]
    async fn ingress_reports_invalid_argument_shape_for_recognized_command() {
        let decision = evaluate_ingress(
            &IngressMemory,
            CommandContext::for_cli(
                "session-1",
                CommandSessionSource::Existing,
                ExecutionMode::Standard,
                None,
            ),
            "/tldr extra args",
            true,
        )
        .await;

        match decision {
            IngressDecision::SessionCommand { outcome } => {
                assert!(matches!(
                    outcome,
                    SessionCommandOutcome::Failure(
                        crate::session_commands::SessionCommandFailure {
                            kind: SessionCommandFailureKind::InvalidArguments,
                            ..
                        }
                    )
                ));
            }
            other => panic!("expected session command error, got {other:?}"),
        }
    }

    #[test]
    fn adapt_handled_ingress_returns_not_handled_for_continue() {
        assert!(matches!(
            adapt_handled_ingress(IngressDecision::Continue),
            HandledIngress::NotHandled
        ));
    }

    #[test]
    fn adapt_handled_ingress_preserves_success_outcome() {
        let success = SessionCommandSuccess {
            command: "/tldr",
            session_id: "session-1".to_string(),
            message: "summary".to_string(),
            data: SessionCommandSuccessData::None,
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Success(success.clone()),
            }),
            HandledIngress::Handled(HandledIngressOutcome::SessionCommandSuccess(actual))
                if actual == success
        ));
    }

    #[test]
    fn adapt_handled_ingress_classifies_permission_failures() {
        for kind in [
            SessionCommandFailureKind::MissingCallerScope,
            SessionCommandFailureKind::PermissionDenied,
        ] {
            let failure = SessionCommandFailure {
                command: "/resume",
                kind: kind.clone(),
                session_id: Some("session-1".to_string()),
                message: "denied".to_string(),
            };

            assert!(matches!(
                adapt_handled_ingress(IngressDecision::SessionCommand {
                    outcome: SessionCommandOutcome::Failure(failure.clone()),
                }),
                HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
                    class: SessionCommandFailureClass::PermissionDenied,
                    failure: actual,
                }) if actual == failure
            ));
        }
    }

    #[test]
    fn adapt_handled_ingress_classifies_generic_failures() {
        let failure = SessionCommandFailure {
            command: "/tldr",
            kind: SessionCommandFailureKind::InvalidArguments,
            session_id: Some("session-1".to_string()),
            message: "bad args".to_string(),
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::SessionCommand {
                outcome: SessionCommandOutcome::Failure(failure.clone()),
            }),
            HandledIngress::Handled(HandledIngressOutcome::SessionCommandFailure {
                class: SessionCommandFailureClass::Failed,
                failure: actual,
            }) if actual == failure
        ));
    }

    #[test]
    fn adapt_handled_ingress_preserves_blocking_outcomes() {
        let blocking = BlockingOutcome::Fallback {
            response: "fallback response".to_string(),
        };

        assert!(matches!(
            adapt_handled_ingress(IngressDecision::Blocking(blocking.clone())),
            HandledIngress::Handled(HandledIngressOutcome::Blocking(actual)) if actual == blocking
        ));
    }

    #[test]
    fn typed_context_builders_preserve_transport_specific_caller_semantics() {
        let cli = CommandContext::for_cli(
            "session-cli",
            CommandSessionSource::Existing,
            ExecutionMode::Standard,
            Some("cli-scope".to_string()),
        );
        let gateway = CommandContext::for_gateway_http(
            "session-http",
            CommandSessionSource::Explicit,
            ExecutionMode::Plan,
            Some("verified-scope".to_string()),
        );
        let channel = CommandContext::for_channel(
            "session-channel",
            CommandSessionSource::Generated,
            ExecutionMode::Standard,
            "discord",
            Some("channel-scope".to_string()),
        );

        assert!(matches!(cli.ingress.source, CommandIngressSource::Cli));
        assert!(matches!(
            cli.caller,
            CommandCaller::DerivedCliScope { ref scope_key } if scope_key == "cli-scope"
        ));

        assert!(matches!(
            gateway.ingress.source,
            CommandIngressSource::GatewayHttp
        ));
        assert!(matches!(
            gateway.ingress.execution_mode,
            ExecutionMode::Plan
        ));
        assert!(matches!(
            gateway.caller,
            CommandCaller::VerifiedTokenHash { ref scope_key } if scope_key == "verified-scope"
        ));

        assert!(matches!(
            channel.ingress.source,
            CommandIngressSource::Channel { ref name } if name == "discord"
        ));
        assert!(matches!(
            channel.caller,
            CommandCaller::DerivedChannelScope {
                ref channel,
                ref scope_key,
            } if channel == "discord" && scope_key == "channel-scope"
        ));
    }
}
