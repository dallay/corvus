use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::agent::dispatcher::{evaluate_tool_risk_for_origin, DispatchAction};
use corvus::agent::mission::MissionState;
use corvus::agent::mission::MissionTerminationReason;
use corvus::approval::{structured_denial_payload_for_origin, ApprovalManager};
use corvus::config::AutonomyConfig;
use corvus::config::MissionConfig;
use corvus::memory::Memory;
use corvus::observability::{Observer, ObserverEvent};
use corvus::providers::{ChatRequest, ChatResponse, Provider, ToolCall};
use corvus::security::{AutonomyLevel, ExecutionOrigin, SecurityPolicy, ToolPolicyDecision};
use corvus::tools::{Tool, ToolResult};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CapturingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl CapturingObserver {
    fn snapshot(&self) -> Vec<ObserverEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl Observer for CapturingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().unwrap().push(event.clone());
    }

    fn record_metric(&self, _metric: &corvus::observability::traits::ObserverMetric) {}

    fn name(&self) -> &str {
        "capturing"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct MissionSecurityProvider {
    responses: Mutex<VecDeque<anyhow::Result<ChatResponse>>>,
}

#[async_trait::async_trait]
impl Provider for MissionSecurityProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Ok(ChatResponse {
                text: Some("done".to_string()),
                tool_calls: vec![],
            });
        }
        responses.pop_front().unwrap()
    }
}

struct CountingTool {
    executions: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Tool for CountingTool {
    fn name(&self) -> &str {
        "mcp.docs.search"
    }

    fn description(&self) -> &str {
        "Counts executions"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResult {
            success: true,
            output: "ok".to_string(),
            error: None,
        })
    }
}

#[test]
fn mission_dispatcher_risk_classification_has_no_bypass_path() {
    assert_eq!(
        evaluate_tool_risk_for_origin("mcp.docs.search", ExecutionOrigin::Mission),
        evaluate_tool_risk_for_origin("mcp.docs.search", ExecutionOrigin::Standard)
    );
    assert!(matches!(
        evaluate_tool_risk_for_origin("mcp.docs.search", ExecutionOrigin::Mission),
        DispatchAction::ApprovalRequired(_)
    ));
}

#[test]
fn mission_policy_checks_match_standard_semantics() {
    let policy = SecurityPolicy::default();

    assert_eq!(
        policy.evaluate_tool_policy_for_origin("mcp.docs.search", ExecutionOrigin::Mission),
        ToolPolicyDecision::ApprovalRequired
    );
    assert_eq!(
        policy.evaluate_tool_policy_for_origin("file_read", ExecutionOrigin::Mission),
        ToolPolicyDecision::Allow
    );
    assert_eq!(
        policy.evaluate_tool_policy_for_origin("mcp.docs.search", ExecutionOrigin::Mission),
        policy.evaluate_tool_policy_for_origin("mcp.docs.search", ExecutionOrigin::Standard)
    );
}

#[test]
fn mission_approval_gate_follows_standard_path() {
    let config = AutonomyConfig {
        level: AutonomyLevel::Supervised,
        auto_approve: vec!["file_read".to_string()],
        always_ask: vec!["shell".to_string()],
        ..AutonomyConfig::default()
    };
    let manager = ApprovalManager::from_config(&config);

    assert_eq!(
        manager.needs_approval_for_origin("shell", ExecutionOrigin::Mission),
        manager.needs_approval_for_origin("shell", ExecutionOrigin::Standard)
    );
    assert!(manager.needs_approval_for_origin("shell", ExecutionOrigin::Mission));
}

#[test]
fn mission_denial_payload_preserves_structured_fields() {
    let denial = structured_denial_payload_for_origin(
        "mcp.docs.search",
        "approval required",
        ExecutionOrigin::Mission,
    );

    assert_eq!(denial["code"], "approval_required");
    assert_eq!(denial["tool"], "mcp.docs.search");
    assert_eq!(denial["reason"], "approval required");
}

#[tokio::test]
async fn mission_policy_denial_path_blocks_tool_side_effects() {
    let tool_executions = Arc::new(AtomicUsize::new(0));
    let provider = Box::new(MissionSecurityProvider {
        responses: Mutex::new(VecDeque::from(vec![
            Ok(ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "call-1".to_string(),
                    name: "mcp.docs.search".to_string(),
                    arguments: "{\"query\":\"x\"}".to_string(),
                }],
            }),
            Ok(ChatResponse {
                text: Some("checkpoint completed without tool execution".to_string()),
                tool_calls: vec![],
            }),
        ])),
    });

    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(corvus::memory::NoneMemory::new());
    let mut agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(CountingTool {
            executions: Arc::clone(&tool_executions),
        })])
        .memory(memory)
        .observer(observer_dyn)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(std::path::PathBuf::from("/tmp"))
        .mission_config(MissionConfig {
            enabled: true,
            ..MissionConfig::default()
        })
        .build()
        .unwrap();

    let outcome = agent.run_mission("single checkpoint", None).await.unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::PolicyDenied)
    );
    assert_eq!(tool_executions.load(Ordering::SeqCst), 0);
    assert!(observer.snapshot().iter().any(|event| matches!(
        event,
        ObserverEvent::MissionTerminated {
            termination_reason,
            rollback,
            ..
        } if termination_reason == "policy_denied" && !rollback
    )));
}
