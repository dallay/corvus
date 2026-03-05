use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::agent::mission::{
    MissionCoordinator, MissionGovernance, MissionState, MissionTerminationReason,
};
use corvus::config::MissionConfig;
use corvus::memory::Memory;
use corvus::observability::{Observer, ObserverEvent};
use corvus::providers::{ChatRequest, ChatResponse, Provider};
use corvus::tools::{Tool, ToolResult};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

struct GovernanceProvider {
    responses: Mutex<VecDeque<anyhow::Result<ChatResponse>>>,
    prompts: Arc<Mutex<Vec<String>>>,
    delay: Duration,
}

#[async_trait]
impl Provider for GovernanceProvider {
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

        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
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

struct GovernanceTool;

#[async_trait]
impl Tool for GovernanceTool {
    fn name(&self) -> &str {
        "noop"
    }

    fn description(&self) -> &str {
        "No-op governance test tool"
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
    delay: Duration,
) -> (Agent, Arc<CapturingObserver>) {
    let provider = Box::new(GovernanceProvider {
        responses: Mutex::new(VecDeque::from(responses)),
        prompts,
        delay,
    });
    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(corvus::memory::NoneMemory::new());

    let agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(GovernanceTool)])
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
async fn mission_terminates_with_budget_exhausted_before_next_checkpoint() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, observer) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 300_000,
            max_steps: 1,
            max_estimated_cost_cents: 100,
        },
        vec![Ok(ChatResponse {
            text: Some("checkpoint-1".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
        Duration::from_millis(0),
    );

    let outcome = agent
        .run_mission("step-one -> step-two", None)
        .await
        .unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::BudgetExhausted)
    );
    assert_eq!(outcome.checkpoints_completed, 1);
    assert_eq!(
        prompts.lock().unwrap().clone(),
        vec!["step-one".to_string()]
    );

    let events = observer.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionTerminated {
            mission_id,
            checkpoint_index,
            termination_reason,
            duration,
            rollback,
        } if mission_id == &outcome.mission_id
            && *checkpoint_index == Some(1)
            && termination_reason == "budget_exhausted"
            && !duration.is_zero()
            && !rollback
    )));
}

#[tokio::test]
async fn mission_terminates_with_sla_exceeded_after_checkpoint_accounting() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, observer) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 1,
            max_steps: 10,
            max_estimated_cost_cents: 100,
        },
        vec![Ok(ChatResponse {
            text: Some("checkpoint-1".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
        Duration::from_millis(10),
    );

    let outcome = agent
        .run_mission("step-one -> step-two", None)
        .await
        .unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::SlaExceeded)
    );
    assert_eq!(outcome.checkpoints_completed, 0);
    assert_eq!(
        prompts.lock().unwrap().clone(),
        vec!["step-one".to_string()]
    );

    let events = observer.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionGuardrailViolation {
            mission_id,
            checkpoint_index,
            termination_reason,
            detail,
            ..
        } if mission_id == &outcome.mission_id
            && *checkpoint_index == Some(0)
            && termination_reason == "sla_exceeded"
            && detail == "sla_exceeded"
            && !detail.to_ascii_lowercase().contains("token")
            && !detail.to_ascii_lowercase().contains("password")
            && !detail.to_ascii_lowercase().contains("secret")
            && !detail.to_ascii_lowercase().contains("api_key")
            && !detail.to_ascii_lowercase().contains("auth")
    )));
}

#[test]
fn unknown_accounting_state_fails_closed_on_overflow() {
    let coordinator = MissionCoordinator::new(MissionGovernance {
        max_runtime_ms: u64::MAX,
        max_steps: u32::MAX,
        max_estimated_cost_cents: u32::MAX,
        elapsed_ms: 0,
        completed_steps: 0,
        accumulated_cost_cents: 0,
    });

    coordinator
        .record_checkpoint_accounting(u64::MAX, 0)
        .expect("first accounting update should succeed");

    let error = coordinator
        .record_checkpoint_accounting(1, 0)
        .expect_err("overflow must fail closed");
    assert_eq!(
        error,
        MissionTerminationReason::GovernanceConstraintViolated
    );
}

#[test]
fn governance_termination_reason_precedence_is_deterministic() {
    let coordinator = MissionCoordinator::new(MissionGovernance {
        max_runtime_ms: 1,
        max_steps: 5,
        max_estimated_cost_cents: 1,
        elapsed_ms: 0,
        completed_steps: 0,
        accumulated_cost_cents: 0,
    });

    let error = coordinator
        .record_checkpoint_accounting(2, 2)
        .expect_err("budget and SLA overrun should terminate deterministically");
    assert_eq!(error, MissionTerminationReason::BudgetExhausted);
}

#[tokio::test]
async fn invalid_mission_config_fails_closed_before_execution() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, _) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 0,
            max_steps: 10,
            max_estimated_cost_cents: 100,
        },
        vec![Ok(ChatResponse {
            text: Some("checkpoint-1".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
        Duration::from_millis(0),
    );

    let outcome = agent
        .run_mission("step-one -> step-two", None)
        .await
        .unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::GovernanceConstraintViolated)
    );
    assert!(prompts.lock().unwrap().is_empty());
}

#[tokio::test]
async fn zero_step_ceiling_fails_closed_before_checkpoint_execution() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, _) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 300_000,
            max_steps: 0,
            max_estimated_cost_cents: 100,
        },
        vec![Ok(ChatResponse {
            text: Some("checkpoint-1".to_string()),
            tool_calls: vec![],
        })],
        Arc::clone(&prompts),
        Duration::from_millis(0),
    );

    let outcome = agent
        .run_mission("step-one -> step-two", None)
        .await
        .unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::GovernanceConstraintViolated)
    );
    assert!(prompts.lock().unwrap().is_empty());
}

#[test]
fn malformed_or_incomplete_mission_config_json_is_rejected_fail_closed() {
    let malformed = r#"{"enabled":true,"max_runtime_ms":1000,"max_steps":10"#;
    let missing_fields = r#"{"enabled":true,"max_runtime_ms":1000}"#;
    let invalid_values =
        r#"{"enabled":true,"max_runtime_ms":0,"max_steps":10,"max_estimated_cost_cents":100}"#;
    let max_steps_zero =
        r#"{"enabled":true,"max_runtime_ms":1000,"max_steps":0,"max_estimated_cost_cents":100}"#;
    let negative_values =
        r#"{"enabled":true,"max_runtime_ms":1000,"max_steps":-1,"max_estimated_cost_cents":100}"#;

    for raw in [
        malformed,
        missing_fields,
        invalid_values,
        max_steps_zero,
        negative_values,
    ] {
        let error = MissionGovernance::from_json_strict(raw)
            .expect_err("invalid governance payload must fail closed");
        assert_eq!(
            error,
            MissionTerminationReason::GovernanceConstraintViolated
        );
    }
}

#[tokio::test]
async fn mission_error_events_are_sanitized_before_observer_record() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, observer) = build_agent(
        MissionConfig {
            enabled: true,
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
        },
        vec![Err(anyhow::anyhow!(
            "fatal provider crash token=abc123 password=hunter2 secret=value api_key=xyz auth=bearer"
        ))],
        Arc::clone(&prompts),
        Duration::from_millis(0),
    );

    let outcome = agent.run_mission("single-checkpoint", None).await.unwrap();
    assert_eq!(outcome.state, MissionState::Terminated);
    assert_eq!(
        outcome.termination,
        Some(MissionTerminationReason::Unrecoverable)
    );

    let events = observer.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::Error { component, message }
            if component == "mission"
                && message == "***REDACTED***"
                && !message.contains("abc123")
                && !message.contains("hunter2")
    )));
}
