use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::agent::mission::MissionState;
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

struct MissionProvider {
    responses: Mutex<VecDeque<anyhow::Result<ChatResponse>>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Provider for MissionProvider {
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

struct MissionTool;

#[async_trait]
impl Tool for MissionTool {
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
            structured: None,
        })
    }
}

fn build_agent(
    responses: Vec<anyhow::Result<ChatResponse>>,
    prompts: Arc<Mutex<Vec<String>>>,
    resume_enabled: bool,
) -> (Agent, Arc<CapturingObserver>) {
    let provider = Box::new(MissionProvider {
        responses: Mutex::new(VecDeque::from(responses)),
        prompts,
    });
    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(corvus::memory::NoneMemory::new());

    let agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(MissionTool)])
        .memory(memory)
        .observer(observer_dyn)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(std::path::PathBuf::from("/tmp"))
        .mission_config(MissionConfig {
            enabled: resume_enabled,
            ..MissionConfig::default()
        })
        .build()
        .unwrap();

    (agent, observer)
}

#[tokio::test]
async fn mission_runs_objective_intake_and_ordered_checkpoints() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let responses = vec![
        Ok(ChatResponse {
            text: Some("checkpoint-1".to_string()),
            tool_calls: vec![],
        }),
        Ok(ChatResponse {
            text: Some("checkpoint-2".to_string()),
            tool_calls: vec![],
        }),
        Ok(ChatResponse {
            text: Some("checkpoint-3".to_string()),
            tool_calls: vec![],
        }),
    ];

    let (mut agent, observer) = build_agent(responses, Arc::clone(&prompts), true);
    let outcome = agent
        .run_mission(
            "intake objective -> plan mission -> execute checkpoint",
            None,
        )
        .await
        .unwrap();

    assert_eq!(outcome.state, MissionState::Completed);
    assert_eq!(outcome.termination, None);
    assert_eq!(outcome.checkpoints_completed, 3);
    assert_eq!(outcome.resume_metadata.last_successful_checkpoint, Some(2));
    assert!(outcome.mission_id.starts_with("mission-"));

    let prompts = prompts.lock().unwrap().clone();
    assert_eq!(
        prompts,
        vec![
            "intake objective".to_string(),
            "plan mission".to_string(),
            "execute checkpoint".to_string(),
        ]
    );

    let events = observer.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionStarted {
            mission_id,
            checkpoint_count,
            resume_from,
        } if mission_id == &outcome.mission_id && *checkpoint_count == 3 && resume_from.is_none()
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionCheckpointProgress {
            mission_id,
            checkpoint_index,
            status,
            duration,
        } if mission_id == &outcome.mission_id
            && *checkpoint_index == 0
            && status == "completed"
            && !duration.is_zero()
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionCompleted {
            mission_id,
            checkpoints_completed,
            duration,
        } if mission_id == &outcome.mission_id
            && *checkpoints_completed == 3
            && !duration.is_zero()
    )));
}

#[tokio::test]
async fn mission_resume_metadata_skips_previously_completed_checkpoint() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let responses = vec![
        Ok(ChatResponse {
            text: Some("resume-1".to_string()),
            tool_calls: vec![],
        }),
        Ok(ChatResponse {
            text: Some("resume-2".to_string()),
            tool_calls: vec![],
        }),
    ];

    let (mut agent, _) = build_agent(responses, Arc::clone(&prompts), true);
    let outcome = agent
        .run_mission(
            "checkpoint-zero -> checkpoint-one -> checkpoint-two",
            Some(0),
        )
        .await
        .unwrap();

    assert_eq!(outcome.state, MissionState::Completed);
    assert_eq!(outcome.checkpoints_completed, 3);
    assert_eq!(outcome.resume_metadata.last_successful_checkpoint, Some(2));

    let prompts = prompts.lock().unwrap().clone();
    assert_eq!(
        prompts,
        vec!["checkpoint-one".to_string(), "checkpoint-two".to_string(),]
    );
}

#[tokio::test]
async fn mission_replans_after_recoverable_checkpoint_failure() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let responses = vec![
        Ok(ChatResponse {
            text: Some("first-success".to_string()),
            tool_calls: vec![],
        }),
        Err(anyhow::anyhow!("temporary timeout from provider")),
        Ok(ChatResponse {
            text: Some("second-success".to_string()),
            tool_calls: vec![],
        }),
    ];

    let (mut agent, observer) = build_agent(responses, Arc::clone(&prompts), true);
    let outcome = agent
        .run_mission("checkpoint-a -> checkpoint-b", None)
        .await
        .unwrap();

    assert_eq!(outcome.state, MissionState::Completed);
    assert_eq!(outcome.checkpoints_completed, 2);
    assert_eq!(outcome.resume_metadata.last_successful_checkpoint, Some(1));

    let failure = outcome
        .resume_metadata
        .latest_failure
        .expect("failure metadata");
    assert_eq!(failure.checkpoint_index, 1);
    assert!(failure.recoverable);
    assert!(failure.reason.contains("timeout"));

    let prompts = prompts.lock().unwrap().clone();
    assert_eq!(
        prompts,
        vec![
            "checkpoint-a".to_string(),
            "checkpoint-b".to_string(),
            "checkpoint-b".to_string(),
        ]
    );

    let events = observer.snapshot();
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionCheckpointProgress {
            mission_id,
            checkpoint_index,
            status,
            duration,
        } if mission_id == &outcome.mission_id
            && *checkpoint_index == 1
            && status == "failed"
            && !duration.is_zero()
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::MissionCheckpointProgress {
            mission_id,
            checkpoint_index,
            status,
            ..
        } if mission_id == &outcome.mission_id
            && *checkpoint_index == 1
            && status == "replanning"
    )));
}

#[tokio::test]
async fn mission_disabled_preserves_legacy_non_mission_turn_behavior() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let responses = vec![Ok(ChatResponse {
        text: Some("legacy turn".to_string()),
        tool_calls: vec![],
    })];

    let (mut agent, observer) = build_agent(responses, Arc::clone(&prompts), false);
    let outcome = agent
        .run_mission("single objective while mission disabled", None)
        .await
        .unwrap();

    assert_eq!(outcome.state, MissionState::Completed);
    assert_eq!(outcome.termination, None);
    assert_eq!(outcome.checkpoints_completed, 0);
    assert_eq!(outcome.resume_metadata.last_successful_checkpoint, None,);

    let prompts = prompts.lock().unwrap().clone();
    assert_eq!(prompts, vec!["single objective while mission disabled"]);

    let events = observer.snapshot();
    assert!(events
        .iter()
        .all(|event| !matches!(event, ObserverEvent::MissionStarted { .. })));
}
