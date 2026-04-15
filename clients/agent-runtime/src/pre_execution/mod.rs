use crate::agent::unified_entrypoint::{self, CanonicalOutcome};
use crate::memory::Memory;
use crate::session_commands::{
    default_registry, CommandContext, SessionCommandResult, SessionCommandService,
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
    SessionCommand {
        result: SessionCommandResult,
        success: bool,
    },
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
    session_id: &str,
    prompt: &str,
    caller_token_hash: Option<&str>,
) -> IngressDecision {
    let service = SessionCommandService::new(memory);
    if let Some(result) = default_registry()
        .dispatch(
            &service,
            CommandContext {
                session_id,
                caller_token_hash,
            },
            prompt,
        )
        .await
    {
        return match result {
            Ok(result) => IngressDecision::SessionCommand {
                result,
                success: true,
            },
            Err(error) => IngressDecision::SessionCommand {
                result: SessionCommandResult {
                    command: "slash-session-error",
                    session_id: session_id.to_string(),
                    message: error.message(),
                    resumed_session_id: None,
                    resumable_sessions: Vec::new(),
                },
                success: false,
            },
        };
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
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
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
        let decision = evaluate_ingress(&IngressMemory, "session-1", "/tldr", None).await;

        match decision {
            IngressDecision::SessionCommand { result, success } => {
                assert!(!success);
                assert_eq!(
                    result.message,
                    "slash-session commands require sqlite memory backend (backend=none)"
                );
            }
            other => panic!("expected session command interception, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ingress_preserves_unknown_slash_like_input() {
        let decision = evaluate_ingress(&IngressMemory, "session-1", "/resume-later", None).await;

        assert!(matches!(decision, IngressDecision::Continue));
    }

    #[tokio::test]
    async fn ingress_reports_invalid_argument_shape_for_recognized_command() {
        let decision =
            evaluate_ingress(&IngressMemory, "session-1", "/tldr extra args", None).await;

        match decision {
            IngressDecision::SessionCommand { result, success } => {
                assert!(!success);
                assert!(result
                    .message
                    .contains("invalid slash command usage for /tldr"));
            }
            other => panic!("expected session command error, got {other:?}"),
        }
    }
}
