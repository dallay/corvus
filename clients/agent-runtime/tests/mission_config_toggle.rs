use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::agent::mission::{MissionState, MissionTerminationReason};
use corvus::config::MissionConfig;
use corvus::memory::Memory;
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

struct ToggleProvider {
    responses: Mutex<VecDeque<anyhow::Result<ChatResponse>>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Provider for ToggleProvider {
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
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        if let Some(last_user) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| message.content.clone())
        {
            self.prompts.lock().unwrap().push(last_user);
        }

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

struct ToggleTool;

#[async_trait]
impl Tool for ToggleTool {
    fn name(&self) -> &str {
        "noop"
    }

    fn description(&self) -> &str {
        "No-op test tool"
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

fn build_agent(
    mission_config: MissionConfig,
    responses: Vec<anyhow::Result<ChatResponse>>,
    prompts: Arc<Mutex<Vec<String>>>,
) -> (Agent, Arc<CapturingObserver>) {
    let provider = Box::new(ToggleProvider {
        responses: Mutex::new(VecDeque::from(responses)),
        prompts,
    });
    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(corvus::memory::NoneMemory::new());

    let agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(ToggleTool)])
        .memory(memory)
        .observer(observer_dyn)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(std::path::PathBuf::from("/tmp"))
        .mission_config(mission_config)
        .build()
        .unwrap();

    (agent, observer)
}

#[tokio::test]
async fn mission_toggle_true_to_false_routes_new_requests_to_legacy_with_rollback_telemetry() {
    let prompts = Arc::new(Mutex::new(Vec::new()));

    let (mut enabled_agent, enabled_observer) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 300_000,
            max_steps: 1,
            max_estimated_cost_cents: 100,
        },
        vec![Ok(ChatResponse {
            text: Some("checkpoint-ok".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
    );

    let terminated = enabled_agent
        .run_mission("checkpoint-one -> checkpoint-two", None)
        .await
        .unwrap();
    assert_eq!(terminated.state, MissionState::Terminated);
    assert_eq!(
        terminated.termination,
        Some(MissionTerminationReason::BudgetExhausted)
    );
    assert_eq!(
        terminated.resume_metadata.last_successful_checkpoint,
        Some(0)
    );

    let enabled_events = enabled_observer.snapshot();
    assert!(enabled_events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionTerminated {
            mission_id,
            checkpoint_index,
            termination_reason,
            rollback,
            ..
        } if mission_id == &terminated.mission_id
            && *checkpoint_index == Some(1)
            && termination_reason == "budget_exhausted"
            && !rollback
    )));

    let (mut disabled_agent, disabled_observer) = build_agent(
        MissionConfig {
            enabled: false,
            ..MissionConfig::default()
        },
        vec![Ok(ChatResponse {
            text: Some("legacy-complete".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
    );

    let resumed = disabled_agent
        .run_mission(
            "post-toggle request should use legacy loop",
            terminated.resume_metadata.last_successful_checkpoint,
        )
        .await
        .unwrap();

    assert_eq!(resumed.state, MissionState::Completed);
    assert_eq!(resumed.termination, None);
    assert_eq!(resumed.checkpoints_completed, 0);
    assert_eq!(resumed.resume_metadata.last_successful_checkpoint, None);
    assert_eq!(resumed.resume_metadata.latest_failure, None);

    let all_prompts = prompts.lock().unwrap().clone();
    assert_eq!(
        all_prompts,
        vec![
            "checkpoint-one".to_string(),
            "post-toggle request should use legacy loop".to_string(),
        ]
    );

    let disabled_events = disabled_observer.snapshot();
    assert!(disabled_events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionTerminated {
            mission_id,
            checkpoint_index,
            termination_reason,
            rollback,
            ..
        } if mission_id == &resumed.mission_id
            && *checkpoint_index == Some(0)
            && termination_reason == "mission_disabled_rollback"
            && *rollback
    )));
    assert!(disabled_events
        .iter()
        .all(|event| !matches!(event, ObserverEvent::MissionCheckpointProgress { .. })));
}
