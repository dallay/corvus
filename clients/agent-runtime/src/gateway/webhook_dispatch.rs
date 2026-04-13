use crate::agent::dispatcher::evaluate_tool_risk;
use crate::agent::dispatcher::DispatchAction;
use crate::agent::{Agent, AgentTurnEvent, AgentTurnOutcome, AgentTurnResult, TurnContext};
use crate::bootstrap;
use crate::config::{Config, ExecutionMode};
use crate::cost::UsagePeriod;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::pre_execution::BlockingOutcome;
use crate::providers::traits::{
    ProviderCapabilities, StreamChunk, StreamOptions, StreamResult, ToolsPayload,
};
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider};
use futures_util::stream;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookSessionSource {
    Explicit,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookTurnRequest {
    pub session_id: String,
    pub session_source: WebhookSessionSource,
    pub message: String,
    pub execution_mode: ExecutionMode,
    pub include_sse_frames: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookTerminalOutcome {
    Completed,
    BudgetExceeded {
        current_usd: f64,
        limit_usd: f64,
        period: UsagePeriod,
    },
    ApprovalRequired {
        tool: String,
        reason: String,
    },
    PlanModeBlocked {
        tool: String,
        reason: String,
        execution_mode: ExecutionMode,
    },
    Timeout,
    Fallback,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WebhookTurnResult {
    pub session_id: String,
    pub model: String,
    pub outcome: WebhookTerminalOutcome,
    pub response_text: Option<String>,
    pub event_frames: Vec<String>,
    pub tools_called: Vec<String>,
}

pub(crate) enum CanonicalWebhookResult {
    Agent(AgentTurnResult),
    Blocking(BlockingOutcome),
    BudgetExceeded {
        current_usd: f64,
        limit_usd: f64,
        period: UsagePeriod,
    },
    ApprovalRequired {
        tool: String,
        reason: String,
    },
    PlanModeBlocked {
        tool: String,
        reason: String,
        execution_mode: ExecutionMode,
    },
    Error,
}

struct SharedProvider {
    inner: Arc<dyn Provider>,
}

#[async_trait::async_trait]
impl Provider for SharedProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        self.inner.capabilities()
    }

    fn convert_tools(&self, tools: &[crate::tools::ToolSpec]) -> ToolsPayload {
        self.inner.convert_tools(tools)
    }

    async fn simple_chat(
        &self,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.inner.simple_chat(message, model, temperature).await
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.inner
            .chat_with_system(system_prompt, message, model, temperature)
            .await
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.inner
            .chat_with_history(messages, model, temperature)
            .await
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.inner.chat(request, model, temperature).await
    }

    fn supports_native_tools(&self) -> bool {
        self.inner.supports_native_tools()
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        self.inner.warmup().await
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    fn stream_chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        self.inner
            .stream_chat_with_system(system_prompt, message, model, temperature, options)
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        self.inner
            .stream_chat_with_history(messages, model, temperature, options)
    }
}

pub(crate) fn turn_context_for_request(request: &WebhookTurnRequest) -> TurnContext {
    // Generated webhook session ids are still propagated into the canonical turn so memory,
    // observer correlation, and audit continuity stay isolated per request instead of falling back
    // to a shared unscoped webhook bucket.
    TurnContext::with_session(request.session_id.clone())
}

pub(crate) fn map_canonical_result(
    request: &WebhookTurnRequest,
    model: &str,
    result: CanonicalWebhookResult,
) -> WebhookTurnResult {
    match result {
        CanonicalWebhookResult::Agent(agent_result) => {
            let outcome = match agent_result.terminal_outcome {
                AgentTurnOutcome::Completed => WebhookTerminalOutcome::Completed,
            };
            let response_text = agent_result.final_text.clone();
            let event_frames = event_frames_for_agent_result(request, &agent_result);
            let tools_called = agent_result.tools_called.clone();
            WebhookTurnResult {
                session_id: request.session_id.clone(),
                model: model.to_string(),
                outcome,
                response_text,
                event_frames,
                tools_called,
            }
        }
        CanonicalWebhookResult::ApprovalRequired { tool, reason } => WebhookTurnResult {
            session_id: request.session_id.clone(),
            model: model.to_string(),
            outcome: WebhookTerminalOutcome::ApprovalRequired {
                tool,
                reason: reason.clone(),
            },
            response_text: None,
            event_frames: event_frames_for_blocking_result(
                request,
                "approval_required",
                Some(reason.as_str()),
            ),
            tools_called: vec![],
        },
        CanonicalWebhookResult::PlanModeBlocked {
            tool,
            reason,
            execution_mode,
        } => WebhookTurnResult {
            session_id: request.session_id.clone(),
            model: model.to_string(),
            outcome: WebhookTerminalOutcome::PlanModeBlocked {
                tool,
                reason: reason.clone(),
                execution_mode,
            },
            response_text: None,
            event_frames: event_frames_for_blocking_result(
                request,
                crate::security::PLAN_MODE_BLOCKED_CODE,
                Some(reason.as_str()),
            ),
            tools_called: vec![],
        },
        CanonicalWebhookResult::BudgetExceeded {
            current_usd,
            limit_usd,
            period,
        } => WebhookTurnResult {
            session_id: request.session_id.clone(),
            model: model.to_string(),
            outcome: WebhookTerminalOutcome::BudgetExceeded {
                current_usd,
                limit_usd,
                period,
            },
            response_text: None,
            event_frames: event_frames_for_blocking_result(
                request,
                "budget_exceeded",
                Some("budget_exceeded"),
            ),
            tools_called: vec![],
        },
        CanonicalWebhookResult::Blocking(BlockingOutcome::ApprovalRequired { tool }) => {
            let reason = approval_reason_for_tool(&tool);
            WebhookTurnResult {
                session_id: request.session_id.clone(),
                model: model.to_string(),
                outcome: WebhookTerminalOutcome::ApprovalRequired {
                    tool,
                    reason: reason.clone(),
                },
                response_text: None,
                event_frames: event_frames_for_blocking_result(
                    request,
                    "approval_required",
                    Some(reason.as_str()),
                ),
                tools_called: vec![],
            }
        }
        CanonicalWebhookResult::Blocking(BlockingOutcome::TimeoutAborted) => WebhookTurnResult {
            session_id: request.session_id.clone(),
            model: model.to_string(),
            outcome: WebhookTerminalOutcome::Timeout,
            response_text: None,
            event_frames: event_frames_for_blocking_result(
                request,
                "error",
                Some("request aborted due to timeout semantics"),
            ),
            tools_called: vec![],
        },
        CanonicalWebhookResult::Blocking(BlockingOutcome::Fallback { response }) => {
            WebhookTurnResult {
                session_id: request.session_id.clone(),
                model: model.to_string(),
                outcome: WebhookTerminalOutcome::Fallback,
                response_text: Some(response),
                event_frames: event_frames_for_blocking_result(request, "complete", None),
                tools_called: vec![],
            }
        }
        CanonicalWebhookResult::Error => WebhookTurnResult {
            session_id: request.session_id.clone(),
            model: model.to_string(),
            outcome: WebhookTerminalOutcome::Error,
            response_text: None,
            event_frames: event_frames_for_blocking_result(request, "error", Some("runtime_error")),
            tools_called: vec![],
        },
    }
}

fn approval_reason_for_tool(tool: &str) -> String {
    match evaluate_tool_risk(tool) {
        DispatchAction::ApprovalRequired(reason) if !reason.trim().is_empty() => reason,
        DispatchAction::ApprovalRequired(_)
        | DispatchAction::Execute
        | DispatchAction::Blocked { .. } => {
            format!("approval required before executing `{tool}`")
        }
    }
}

fn event_frames_for_agent_result(
    request: &WebhookTurnRequest,
    result: &AgentTurnResult,
) -> Vec<String> {
    if !request.include_sse_frames {
        return Vec::new();
    }

    result
        .event_log
        .iter()
        .map(|event| map_agent_event_to_sse_frame(&request.session_id, event))
        .collect()
}

fn event_frames_for_blocking_result(
    request: &WebhookTurnRequest,
    event_name: &str,
    payload: Option<&str>,
) -> Vec<String> {
    if !request.include_sse_frames {
        return Vec::new();
    }

    vec![
        render_sse_frame(&request.session_id, "start", Some("started")),
        render_sse_frame(&request.session_id, event_name, payload),
    ]
}

fn map_agent_event_to_sse_frame(session_id: &str, event: &AgentTurnEvent) -> String {
    match event {
        AgentTurnEvent::Prepared => render_sse_frame(session_id, "start", Some("prepared")),
        AgentTurnEvent::Completed => render_sse_frame(session_id, "complete", Some("completed")),
    }
}

fn render_sse_frame(session_id: &str, event_name: &str, payload: Option<&str>) -> String {
    let data_lines = match payload {
        Some(payload) if !payload.is_empty() => {
            payload.lines().fold(String::new(), |mut acc, line| {
                use std::fmt::Write;
                writeln!(acc, "data: {line}").unwrap();
                acc
            })
        }
        _ => "data:\n".to_string(),
    };
    format!(
        "id: {}\nevent: {event_name}\n{data_lines}\n",
        sanitize_sse_id(session_id)
    )
}

fn sanitize_sse_id(session_id: &str) -> String {
    session_id
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .replace(['\r', '\n'], "")
}

fn approval_denial_from_history(history: &[ConversationMessage]) -> Option<(String, String)> {
    history.iter().rev().find_map(|message| {
        let ConversationMessage::ToolResults(results) = message else {
            return None;
        };

        results.iter().find_map(|result| {
            let parsed = serde_json::from_str::<serde_json::Value>(&result.content).ok()?;
            if parsed.get("code")?.as_str()? != "approval_required" {
                return None;
            }

            Some((
                parsed.get("tool")?.as_str()?.to_string(),
                parsed
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("approval_required")
                    .to_string(),
            ))
        })
    })
}

pub(crate) async fn execute(
    config: &Config,
    provider: Arc<dyn Provider>,
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
    cost_tracker: Option<Arc<crate::cost::CostTracker>>,
    model: &str,
    request: WebhookTurnRequest,
) -> WebhookTurnResult {
    let canonical =
        crate::pre_execution::evaluate(request.session_id.clone(), &request.message).await;
    if let Some(blocking) = crate::pre_execution::classify_blocking(&canonical) {
        match blocking {
            BlockingOutcome::ApprovalRequired { tool } => {
                return map_canonical_result(
                    &request,
                    model,
                    CanonicalWebhookResult::Blocking(BlockingOutcome::ApprovalRequired { tool }),
                );
            }
            other => {
                return map_canonical_result(
                    &request,
                    model,
                    CanonicalWebhookResult::Blocking(other),
                );
            }
        }
    }

    let mut effective_config = config.clone();
    effective_config.agent.execution_mode = request.execution_mode;

    let bootstrap = match bootstrap::BootstrapContext::for_gateway(
        &effective_config,
        memory,
        observer,
        cost_tracker,
    ) {
        Ok(bootstrap) => bootstrap,
        Err(_) => return map_canonical_result(&request, model, CanonicalWebhookResult::Error),
    };

    let provider: Box<dyn Provider> = Box::new(SharedProvider { inner: provider });
    let mut agent =
        match Agent::from_bootstrap_with_provider(&effective_config, bootstrap, provider) {
            Ok(agent) => agent,
            Err(_) => return map_canonical_result(&request, model, CanonicalWebhookResult::Error),
        };

    match agent
        .turn_with_context(&request.message, turn_context_for_request(&request))
        .await
    {
        Ok(result) => {
            if let Some(policy_blocked) = result.policy_blocked.as_ref() {
                match policy_denial_from_value(policy_blocked) {
                    Some((code, tool, reason))
                        if code == crate::security::PLAN_MODE_BLOCKED_CODE =>
                    {
                        map_canonical_result(
                            &request,
                            model,
                            CanonicalWebhookResult::PlanModeBlocked {
                                tool,
                                reason,
                                execution_mode: result.execution_mode,
                            },
                        )
                    }
                    _ => map_canonical_result(&request, model, CanonicalWebhookResult::Error),
                }
            } else if let Some(approval_required) = result.approval_required.as_ref() {
                match policy_denial_from_value(approval_required) {
                    Some((_code, tool, reason)) => map_canonical_result(
                        &request,
                        model,
                        CanonicalWebhookResult::ApprovalRequired { tool, reason },
                    ),
                    None => map_canonical_result(&request, model, CanonicalWebhookResult::Error),
                }
            } else {
                map_canonical_result(&request, model, CanonicalWebhookResult::Agent(result))
            }
        }
        Err(error) => {
            if let Some(crate::agent::AgentExecutionError::CostBudgetExceeded {
                current_usd,
                limit_usd,
                period,
            }) = error.downcast_ref::<crate::agent::AgentExecutionError>()
            {
                map_canonical_result(
                    &request,
                    model,
                    CanonicalWebhookResult::BudgetExceeded {
                        current_usd: *current_usd,
                        limit_usd: *limit_usd,
                        period: *period,
                    },
                )
            } else {
                map_canonical_result(&request, model, CanonicalWebhookResult::Error)
            }
        }
    }
}

fn policy_denial_from_value(value: &serde_json::Value) -> Option<(String, String, String)> {
    let code = value.get("code")?.as_str()?.to_string();
    let tool = value.get("tool")?.as_str()?.to_string();
    Some((
        code,
        tool.clone(),
        value
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| approval_reason_for_tool(&tool)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentTurnEvent, AgentTurnOutcome};
    use crate::config::Config;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::observability::{NoopObserver, Observer};
    use crate::providers::ToolCall;
    use crate::providers::ToolResultMessage;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn sample_request(session_source: WebhookSessionSource) -> WebhookTurnRequest {
        WebhookTurnRequest {
            session_id: "webhook-123".into(),
            session_source,
            message: "hello".into(),
            execution_mode: ExecutionMode::Standard,
            include_sse_frames: false,
        }
    }

    #[test]
    fn explicit_session_keeps_same_turn_context() {
        let context = turn_context_for_request(&sample_request(WebhookSessionSource::Explicit));

        assert_eq!(context.session_id.as_deref(), Some("webhook-123"));
    }

    #[test]
    fn generated_session_is_still_passed_into_canonical_turn_context() {
        let context = turn_context_for_request(&sample_request(WebhookSessionSource::Generated));

        assert_eq!(context.session_id.as_deref(), Some("webhook-123"));
    }

    #[test]
    fn render_sse_frame_sanitizes_session_id_to_single_line() {
        let frame = render_sse_frame(" session\nmalicious\r\nid ", "complete", Some("ok"));

        assert!(frame.starts_with("id: session\n"));
        assert!(!frame.contains("malicious"));
        assert!(!frame.contains("id "));
        assert_eq!(frame.matches("id:").count(), 1);
    }

    #[test]
    fn maps_completed_agent_turn_into_completed_webhook_result() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::Agent(AgentTurnResult {
                session_id: Some("webhook-123".into()),
                execution_mode: ExecutionMode::Standard,
                final_text: Some("done".into()),
                terminal_outcome: AgentTurnOutcome::Completed,
                approval_required: None,
                policy_blocked: None,
                event_log: vec![AgentTurnEvent::Prepared, AgentTurnEvent::Completed],
                tools_called: vec![],
            }),
        );

        assert_eq!(result.session_id, "webhook-123");
        assert_eq!(result.model, "test-model");
        assert_eq!(result.outcome, WebhookTerminalOutcome::Completed);
        assert_eq!(result.response_text.as_deref(), Some("done"));
    }

    #[test]
    fn maps_approval_required_block_into_webhook_denial() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::Blocking(BlockingOutcome::ApprovalRequired {
                tool: "shell".into(),
            }),
        );

        assert_eq!(
            result.outcome,
            WebhookTerminalOutcome::ApprovalRequired {
                tool: "shell".into(),
                reason: "shell".into(),
            }
        );
        assert_eq!(result.response_text, None);
    }

    #[test]
    fn maps_plan_mode_block_into_webhook_denial() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::PlanModeBlocked {
                tool: "file_write".into(),
                reason: "Plan Mode allows analysis-only capabilities and blocks `file_write`"
                    .into(),
                execution_mode: ExecutionMode::Plan,
            },
        );

        assert!(matches!(
            result.outcome,
            WebhookTerminalOutcome::PlanModeBlocked {
                ref tool,
                execution_mode: ExecutionMode::Plan,
                ..
            } if tool == "file_write"
        ));
    }

    #[test]
    fn maps_timeout_block_into_timeout_outcome() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::Blocking(BlockingOutcome::TimeoutAborted),
        );

        assert_eq!(result.outcome, WebhookTerminalOutcome::Timeout);
        assert_eq!(result.response_text, None);
    }

    #[test]
    fn maps_fallback_block_into_fallback_outcome() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::Blocking(BlockingOutcome::Fallback {
                response: "fallback response".into(),
            }),
        );

        assert_eq!(result.outcome, WebhookTerminalOutcome::Fallback);
        assert_eq!(result.response_text.as_deref(), Some("fallback response"));
    }

    #[test]
    fn maps_runtime_error_into_error_outcome() {
        let result = map_canonical_result(
            &sample_request(WebhookSessionSource::Explicit),
            "test-model",
            CanonicalWebhookResult::Error,
        );

        assert_eq!(result.outcome, WebhookTerminalOutcome::Error);
        assert_eq!(result.response_text, None);
    }

    #[tokio::test]
    async fn execute_maps_cost_budget_exceeded_into_machine_readable_outcome() {
        let (_temp, mut config) = test_config();
        config.cost.enabled = true;
        config.cost.daily_limit_usd = 1.0;
        config.cost.monthly_limit_usd = 10.0;

        let tracker = Arc::new(
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap(),
        );
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        let provider_impl = Arc::new(ScriptedProvider::new(vec![ChatResponse {
            text: Some("should not be called".into()),
            tool_calls: Vec::new(),
        }]));
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let result = execute(
            &config,
            provider,
            Arc::new(TestMemory),
            Arc::new(NoopObserver) as Arc<dyn Observer>,
            Some(tracker),
            "test-model",
            WebhookTurnRequest {
                session_id: "session-budget".into(),
                session_source: WebhookSessionSource::Explicit,
                message: "hello".into(),
                execution_mode: ExecutionMode::Standard,
                include_sse_frames: false,
            },
        )
        .await;

        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            result.outcome,
            WebhookTerminalOutcome::BudgetExceeded { .. }
        ));
    }

    #[test]
    fn approval_denial_is_detected_from_tool_results_history() {
        let history = vec![ConversationMessage::ToolResults(vec![ToolResultMessage {
            tool_call_id: "tc-1".into(),
            content: r#"{"code":"approval_required","tool":"shell","reason":"approval required"}"#
                .into(),
        }])];

        assert_eq!(
            approval_denial_from_history(&history),
            Some(("shell".into(), "approval required".into()))
        );
    }

    #[test]
    fn policy_denial_from_value_derives_reason_when_missing() {
        let value = serde_json::json!({
            "code": "approval_required",
            "tool": "shell",
        });

        assert_eq!(
            policy_denial_from_value(&value),
            Some(("approval_required".into(), "shell".into(), "shell".into()))
        );
    }

    #[test]
    fn malformed_approval_payload_maps_to_error_instead_of_completed() {
        let request = sample_request(WebhookSessionSource::Explicit);
        let result = execute_result_to_webhook_result(
            &request,
            "test-model",
            AgentTurnResult {
                session_id: Some("webhook-123".into()),
                execution_mode: ExecutionMode::Standard,
                final_text: Some("done".into()),
                terminal_outcome: AgentTurnOutcome::Completed,
                approval_required: Some(serde_json::json!({
                    "code": "approval_required",
                    "reason": "missing tool",
                })),
                policy_blocked: None,
                event_log: vec![AgentTurnEvent::Prepared, AgentTurnEvent::Completed],
                tools_called: vec![],
            },
        );

        assert_eq!(result.outcome, WebhookTerminalOutcome::Error);
        assert_eq!(result.response_text, None);
    }

    fn execute_result_to_webhook_result(
        request: &WebhookTurnRequest,
        model: &str,
        result: AgentTurnResult,
    ) -> WebhookTurnResult {
        if let Some(policy_blocked) = result.policy_blocked.as_ref() {
            match policy_denial_from_value(policy_blocked) {
                Some((code, tool, reason)) if code == crate::security::PLAN_MODE_BLOCKED_CODE => {
                    map_canonical_result(
                        request,
                        model,
                        CanonicalWebhookResult::PlanModeBlocked {
                            tool,
                            reason,
                            execution_mode: result.execution_mode,
                        },
                    )
                }
                _ => map_canonical_result(request, model, CanonicalWebhookResult::Error),
            }
        } else if let Some(approval_required) = result.approval_required.as_ref() {
            match policy_denial_from_value(approval_required) {
                Some((_code, tool, reason)) => map_canonical_result(
                    request,
                    model,
                    CanonicalWebhookResult::ApprovalRequired { tool, reason },
                ),
                None => map_canonical_result(request, model, CanonicalWebhookResult::Error),
            }
        } else {
            map_canonical_result(request, model, CanonicalWebhookResult::Agent(result))
        }
    }

    #[test]
    fn canonical_event_frames_are_emitted_when_requested() {
        let mut request = sample_request(WebhookSessionSource::Explicit);
        request.include_sse_frames = true;

        let result = map_canonical_result(
            &request,
            "test-model",
            CanonicalWebhookResult::Agent(AgentTurnResult {
                session_id: Some("webhook-123".into()),
                execution_mode: ExecutionMode::Standard,
                final_text: Some("done".into()),
                terminal_outcome: AgentTurnOutcome::Completed,
                approval_required: None,
                policy_blocked: None,
                event_log: vec![AgentTurnEvent::Prepared, AgentTurnEvent::Completed],
                tools_called: vec![],
            }),
        );

        assert_eq!(result.event_frames.len(), 2);
        assert!(result.event_frames[0].contains("event: start\n"));
        assert!(result.event_frames[1].contains("event: complete\n"));
    }

    #[derive(Default)]
    struct TestMemory;

    #[async_trait]
    impl Memory for TestMemory {
        fn name(&self) -> &str {
            "test"
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

    struct ScriptedProvider {
        responses: Mutex<Vec<ChatResponse>>,
        calls: AtomicUsize,
    }

    impl ScriptedProvider {
        fn new(responses: Vec<ChatResponse>) -> Self {
            let mut responses = responses;
            responses.reverse();
            Self {
                responses: Mutex::new(responses),
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Provider for ScriptedProvider {
        fn supports_native_tools(&self) -> bool {
            true
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("unused".into())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.responses.lock().pop().unwrap())
        }
    }

    fn test_config() -> (TempDir, Config) {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.config_path = temp.path().join("config.toml");
        config.workspace_dir = temp.path().join("workspace");
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        (temp, config)
    }

    #[tokio::test]
    async fn execute_maps_blocked_shell_tool_to_approval_required() {
        let (_temp, config) = test_config();
        let provider: Arc<dyn Provider> = Arc::new(ScriptedProvider::new(vec![
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-shell".into(),
                    name: "shell".into(),
                    arguments: r#"{"command":"pwd"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("shell blocked".into()),
                tool_calls: Vec::new(),
            },
        ]));

        let result = execute(
            &config,
            provider,
            Arc::new(TestMemory),
            Arc::new(NoopObserver) as Arc<dyn Observer>,
            None,
            "test-model",
            WebhookTurnRequest {
                session_id: "session-shell".into(),
                session_source: WebhookSessionSource::Explicit,
                message: "run shell".into(),
                execution_mode: ExecutionMode::Standard,
                include_sse_frames: false,
            },
        )
        .await;

        assert_eq!(
            result.outcome,
            WebhookTerminalOutcome::ApprovalRequired {
                tool: "shell".into(),
                reason: "shell".into(),
            }
        );
    }

    #[tokio::test]
    async fn execute_maps_plan_mode_block_to_machine_readable_denial() {
        let (_temp, mut config) = test_config();
        config.agent.execution_mode = ExecutionMode::Plan;
        let provider: Arc<dyn Provider> = Arc::new(ScriptedProvider::new(vec![
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-write".into(),
                    name: "file_write".into(),
                    arguments: r#"{"path":"x","content":"y"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("blocked".into()),
                tool_calls: Vec::new(),
            },
        ]));

        let result = execute(
            &config,
            provider,
            Arc::new(TestMemory),
            Arc::new(NoopObserver) as Arc<dyn Observer>,
            None,
            "test-model",
            WebhookTurnRequest {
                session_id: "session-plan".into(),
                session_source: WebhookSessionSource::Explicit,
                message: "write file".into(),
                execution_mode: ExecutionMode::Plan,
                include_sse_frames: false,
            },
        )
        .await;

        assert!(matches!(
            result.outcome,
            WebhookTerminalOutcome::PlanModeBlocked {
                ref tool,
                execution_mode: ExecutionMode::Plan,
                ..
            } if tool == "file_write"
        ));
    }
}
