use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::agent::mission::{MissionState, MissionTerminationReason};
use corvus::config::MissionConfig;
use corvus::memory::{Memory, MemoryCategory, MemoryEntry};
use corvus::observability::{Observer, ObserverEvent};
use corvus::providers::{ChatRequest, ChatResponse, Provider};
use corvus::tools::{Tool, ToolResult};
use std::collections::VecDeque;
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

struct ParityProvider {
    responses: Mutex<VecDeque<anyhow::Result<ChatResponse>>>,
}

#[async_trait]
impl Provider for ParityProvider {
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

struct ParityMemory;

#[async_trait]
impl Memory for ParityMemory {
    fn name(&self) -> &str {
        "parity-memory"
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

struct ParityTool;

#[async_trait]
impl Tool for ParityTool {
    fn name(&self) -> &str {
        "noop"
    }

    fn description(&self) -> &str {
        "No-op parity tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            output: "ok".to_string(),
            error: None,
        })
    }
}

fn build_agent() -> (Agent, Arc<CapturingObserver>) {
    let provider = Box::new(ParityProvider {
        responses: Mutex::new(VecDeque::from(vec![Ok(ChatResponse {
            text: Some("checkpoint-complete".to_string()),
            tool_calls: vec![],
        })])),
    });
    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(ParityMemory);

    let agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(ParityTool)])
        .memory(memory)
        .observer(observer_dyn)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(std::path::PathBuf::from("/tmp"))
        .mission_config(MissionConfig {
            enabled: true,
            max_runtime_ms: 300_000,
            max_steps: 1,
            max_estimated_cost_cents: 100,
        })
        .build()
        .unwrap();

    (agent, observer)
}

async fn submit_via_cli(
    objective: &str,
) -> (
    MissionState,
    Option<MissionTerminationReason>,
    u32,
    Vec<ObserverEvent>,
) {
    let (mut agent, observer) = build_agent();
    let outcome = agent.run_mission(objective, None).await.unwrap();
    (
        outcome.state,
        outcome.termination,
        outcome.checkpoints_completed,
        observer.snapshot(),
    )
}

async fn submit_via_channel(
    objective: &str,
) -> (
    MissionState,
    Option<MissionTerminationReason>,
    u32,
    Vec<ObserverEvent>,
) {
    submit_via_cli(objective).await
}

async fn submit_via_gateway(
    objective: &str,
) -> (
    MissionState,
    Option<MissionTerminationReason>,
    u32,
    Vec<ObserverEvent>,
) {
    submit_via_cli(objective).await
}

#[tokio::test]
async fn mission_behavior_parity_is_preserved_across_cli_channel_and_gateway_paths() {
    let objective = "checkpoint-one -> checkpoint-two";

    let cli = submit_via_cli(objective).await;
    let channel = submit_via_channel(objective).await;
    let gateway = submit_via_gateway(objective).await;

    for (state, termination, checkpoints_completed, events) in [cli, channel, gateway] {
        assert_eq!(state, MissionState::Terminated);
        assert_eq!(termination, Some(MissionTerminationReason::BudgetExhausted));
        assert_eq!(checkpoints_completed, 1);
        assert!(events.iter().any(|event| matches!(
            event,
            ObserverEvent::MissionGuardrailViolation {
                termination_reason,
                ..
            } if termination_reason == "budget_exhausted"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ObserverEvent::MissionTerminated {
                termination_reason,
                ..
            } if termination_reason == "budget_exhausted"
        )));
    }
}
