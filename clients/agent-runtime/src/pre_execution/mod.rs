use crate::agent::unified_entrypoint::{self, CanonicalOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockingOutcome {
    ApprovalRequired { tool: String },
    TimeoutAborted,
    Fallback { response: String },
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

pub fn classify_blocking(outcome: &CanonicalOutcome) -> Option<BlockingOutcome> {
    if let Some(tool) = &outcome.approval_required {
        return Some(BlockingOutcome::ApprovalRequired { tool: tool.clone() });
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

    #[test]
    fn classify_prefers_approval_required() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: Some("tool-1".to_string()),
            timeout_aborted: true,
            fallback_response: Some("fallback".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::ApprovalRequired {
                tool: "tool-1".to_string(),
            })
        );
    }

    #[test]
    fn classify_identifies_timeout() {
        let outcome = CanonicalOutcome {
            session_id: "s1".to_string(),
            events: Vec::new(),
            approval_required: None,
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
            timeout_aborted: true,
            fallback_response: Some("fallback response".to_string()),
        };

        assert_eq!(
            classify_blocking(&outcome),
            Some(BlockingOutcome::TimeoutAborted)
        );
    }
}
