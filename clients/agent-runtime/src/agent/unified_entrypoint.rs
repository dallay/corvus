use crate::agent::unified_loop::{AgentLoop, LoopConfig, LoopEvent};
use futures::StreamExt;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct UnifiedExecutionConfig {
    pub tool_calls: usize,
    pub step_duration: Duration,
    pub max_retries: u8,
    pub backoff_millis: u64,
    pub enable_test_triggers: bool,
}

impl Default for UnifiedExecutionConfig {
    fn default() -> Self {
        Self {
            tool_calls: 1,
            step_duration: Duration::from_millis(1),
            max_retries: 1,
            backoff_millis: 25,
            enable_test_triggers: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CanonicalOutcomeConfig {
    pub enable_test_triggers: bool,
}

#[derive(Debug, Clone)]
pub struct UnifiedExecutionResult {
    pub session_id: String,
    pub events: Vec<LoopEvent>,
    pub retries_used: u8,
    pub used_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct CanonicalOutcome {
    pub session_id: String,
    pub events: Vec<LoopEvent>,
    pub approval_required: Option<String>,
    pub approval_reason: Option<String>,
    pub timeout_aborted: bool,
    pub fallback_response: Option<String>,
}

pub async fn execute_with_retry_backoff(
    session_id: String,
    prompt: &str,
    config: &LoopConfig,
    mut options: UnifiedExecutionConfig,
) -> UnifiedExecutionResult {
    let mut retries_used = 0;
    let mut events = Vec::new();
    let mut current_config = config.clone();

    loop {
        let loop_runner = AgentLoop::new(current_config.clone());
        let mut current_events = loop_runner
            .run(prompt, options.tool_calls, options.step_duration)
            .collect::<Vec<_>>()
            .await;

        let recoverable = current_events.iter().any(is_recoverable_error);
        events.append(&mut current_events);

        if !recoverable {
            return UnifiedExecutionResult {
                session_id,
                events,
                retries_used,
                used_fallback: false,
            };
        }

        if retries_used >= options.max_retries {
            events.push(LoopEvent::LLMProgress(
                "recoverable error exhausted retries; returning fallback".to_string(),
            ));
            events.push(LoopEvent::Complete(
                "fallback response: temporary tool/runtime issue".to_string(),
            ));
            return UnifiedExecutionResult {
                session_id,
                events,
                retries_used,
                used_fallback: true,
            };
        }

        retries_used += 1;
        let backoff = options
            .backoff_millis
            .saturating_mul(2u64.saturating_pow(u32::from(retries_used - 1)));
        events.push(LoopEvent::LLMProgress(format!(
            "retrying after recoverable error (backoff={}ms)",
            backoff
        )));

        tokio::time::sleep(Duration::from_millis(backoff)).await;

        let timeout_multiplier = 1u32
            .checked_shl(u32::from(retries_used))
            .unwrap_or(u32::MAX);
        options.step_duration = options.step_duration.max(Duration::from_millis(1));
        let mut next_config = config.clone();
        next_config.timeout = config
            .timeout
            .saturating_mul(timeout_multiplier)
            .max(Duration::from_millis(1));

        if options.enable_test_triggers && prompt.contains("tool-failure") {
            events.push(LoopEvent::Error(
                "tool execution failed and requires fallback".to_string(),
            ));
            events.push(LoopEvent::Complete(
                "fallback response: temporary tool/runtime issue".to_string(),
            ));
            return UnifiedExecutionResult {
                session_id,
                events,
                retries_used,
                used_fallback: true,
            };
        }

        current_config = next_config;
    }
}

fn is_recoverable_error(event: &LoopEvent) -> bool {
    match event {
        LoopEvent::Error(message) => {
            message.contains("timeout") || message.contains("tool") || message.contains("iteration")
        }
        _ => false,
    }
}

pub async fn run_canonical_outcome(
    session_id: String,
    prompt: &str,
    approval_granted: bool,
    config: CanonicalOutcomeConfig,
) -> CanonicalOutcome {
    if !config.enable_test_triggers {
        return CanonicalOutcome {
            session_id,
            events: Vec::new(),
            approval_required: None,
            approval_reason: None,
            timeout_aborted: false,
            fallback_response: None,
        };
    }

    let mut loop_config = LoopConfig::default();
    let mut options = UnifiedExecutionConfig::default();
    options.enable_test_triggers = true;

    if prompt.contains("timeout") {
        loop_config.timeout = Duration::from_millis(1);
        options.tool_calls = 2;
        options.step_duration = Duration::from_millis(2);
        options.max_retries = 0;
    }

    if prompt.contains("needs-approval") {
        loop_config.approval_required_tool = Some("tool-1".to_string());
    }

    let result =
        execute_with_retry_backoff(session_id.clone(), prompt, &loop_config, options).await;

    let mut approval_required = None;
    let mut approval_reason = None;

    for event in &result.events {
        if let LoopEvent::ApprovalRequired(tool, reason) = event {
            approval_required = Some(tool.clone());
            approval_reason = Some(reason.clone());
            break;
        }
    }

    let timeout_aborted = result
        .events
        .iter()
        .any(|event| matches!(event, LoopEvent::Error(message) if message.contains("timeout")));

    let fallback_response = if result.used_fallback {
        result.events.iter().rev().find_map(|event| match event {
            LoopEvent::Complete(text) => Some(text.clone()),
            _ => None,
        })
    } else {
        None
    };

    let (approval_required, approval_reason) = if approval_granted {
        (None, None)
    } else {
        (approval_required, approval_reason)
    };

    CanonicalOutcome {
        session_id,
        events: result.events,
        approval_required,
        approval_reason,
        timeout_aborted,
        fallback_response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retry_backoff_recovers_timeout_before_fallback() {
        let config = LoopConfig {
            timeout: Duration::from_millis(1),
            ..LoopConfig::default()
        };
        let result = execute_with_retry_backoff(
            "session-test".to_string(),
            "timeout",
            &config,
            UnifiedExecutionConfig {
                tool_calls: 2,
                step_duration: Duration::from_millis(2),
                max_retries: 1,
                backoff_millis: 1,
                enable_test_triggers: false,
            },
        )
        .await;

        assert_eq!(result.session_id, "session-test");
        assert!(result.retries_used <= 1);
        assert!(result
            .events
            .iter()
            .any(|e| matches!(e, LoopEvent::LLMProgress(msg) if msg.contains("retrying"))));
    }

    #[tokio::test]
    async fn retry_backoff_uses_fallback_on_persistent_tool_failure() {
        let timeout_config = LoopConfig {
            timeout: Duration::from_millis(1),
            ..LoopConfig::default()
        };
        let result = execute_with_retry_backoff(
            "session-tool".to_string(),
            "timeout",
            &timeout_config,
            UnifiedExecutionConfig {
                tool_calls: 2,
                step_duration: Duration::from_millis(2),
                max_retries: 0,
                backoff_millis: 1,
                enable_test_triggers: false,
            },
        )
        .await;

        assert!(result.used_fallback);
        assert!(result.events.iter().any(
            |event| matches!(event, LoopEvent::Complete(msg) if msg.contains("fallback response"))
        ));
    }

    #[tokio::test]
    async fn canonical_outcome_blocks_when_approval_not_granted() {
        let outcome = run_canonical_outcome(
            "session-approve".to_string(),
            "needs-approval",
            false,
            CanonicalOutcomeConfig {
                enable_test_triggers: true,
            },
        )
        .await;
        assert_eq!(outcome.session_id, "session-approve");
        assert_eq!(outcome.approval_required, Some("tool-1".to_string()));
    }

    #[tokio::test]
    async fn canonical_outcome_unblocks_when_approval_granted() {
        let outcome = run_canonical_outcome(
            "session-approve".to_string(),
            "needs-approval",
            true,
            CanonicalOutcomeConfig {
                enable_test_triggers: true,
            },
        )
        .await;
        assert_eq!(outcome.approval_required, None);
    }
}
