use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::config::MissionConfig;
use corvus::memory::{Memory, MemoryCategory, MemoryEntry};
use corvus::observability::{Observer, ObserverEvent};
use corvus::providers::{ChatRequest, ChatResponse, Provider};
use corvus::tools::{Tool, ToolResult};

#[derive(Default)]
struct CapturingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl CapturingObserver {
    fn snapshot(&self) -> Vec<ObserverEvent> {
        self.events.lock().expect("lock observer events").clone()
    }
}

impl Observer for CapturingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events
            .lock()
            .expect("lock observer events")
            .push(event.clone());
    }

    fn record_metric(&self, _metric: &corvus::observability::traits::ObserverMetric) {}

    fn name(&self) -> &str {
        "capturing"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct LegacyProvider;

#[async_trait]
impl Provider for LegacyProvider {
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
        let prompt = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| message.content.clone())
            .unwrap_or_default();
        Ok(ChatResponse {
            text: Some(format!("legacy:{prompt}")),
            tool_calls: vec![],
        })
    }
}

struct LegacyMemory;

#[async_trait]
impl Memory for LegacyMemory {
    fn name(&self) -> &str {
        "legacy-memory"
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

struct LegacyTool;

#[async_trait]
impl Tool for LegacyTool {
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

fn build_legacy_agent(enabled: bool) -> (Agent, Arc<CapturingObserver>) {
    let observer = Arc::new(CapturingObserver::default());
    let observer_dyn: Arc<dyn Observer> = observer.clone();
    let memory: Arc<dyn Memory> = Arc::new(LegacyMemory);

    let agent = Agent::builder()
        .provider(Box::new(LegacyProvider))
        .tools(vec![Box::new(LegacyTool)])
        .memory(memory)
        .observer(observer_dyn)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(PathBuf::from("/tmp"))
        .mission_config(MissionConfig {
            enabled,
            ..MissionConfig::default()
        })
        .build()
        .expect("build legacy agent");

    (agent, observer)
}

fn runtime_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn no_legacy_loop_reexport_in_agent_mod() {
    let mod_rs = fs::read_to_string(runtime_path("src/agent/mod.rs")).expect("read agent/mod.rs");
    assert!(
        !mod_rs.contains("pub use loop_")
            && !mod_rs.contains("pub(crate) use loop_")
            && !mod_rs.contains("pub use loop_::"),
        "legacy loop re-export must be removed"
    );
}

#[test]
fn runtime_entrypoints_do_not_reference_loop_module_directly() {
    for path in ["src/main.rs", "src/daemon/mod.rs", "src/cron/scheduler.rs"] {
        let content = fs::read_to_string(runtime_path(path)).expect("read runtime entrypoint");
        assert!(
            !(content.contains("agent::loop_")
                || (content.contains("agent::{") && content.contains("loop_"))),
            "{path} should not call legacy loop module directly"
        );
    }
}

#[test]
fn channels_runtime_has_no_legacy_loop_import() {
    let content =
        fs::read_to_string(runtime_path("src/channels/mod.rs")).expect("read channels/mod.rs");
    assert!(
        !(content.contains("agent::loop_")
            || (content.contains("agent::{") && content.contains("loop_"))),
        "channels runtime should not import legacy loop module"
    );
}

#[test]
fn legacy_loop_file_is_removed() {
    let exists = runtime_path("src/agent/loop_.rs").exists();
    assert!(!exists, "legacy loop file must be removed");
}

#[tokio::test]
async fn mission_disabled_routes_to_legacy_turn_semantics() {
    let (mut agent, observer) = build_legacy_agent(false);
    let outcome = agent
        .run_mission("legacy mission-disabled path", None)
        .await
        .expect("run mission with mission disabled");

    assert_eq!(outcome.checkpoints_completed, 0);
    assert_eq!(outcome.termination, None);
    assert!(
        observer
            .snapshot()
            .iter()
            .all(|event| !matches!(event, ObserverEvent::MissionStarted { .. })),
        "mission-disabled mode should not emit mission lifecycle events"
    );
}

#[tokio::test]
async fn mission_disabled_does_not_emit_rollback_without_prior_checkpoint() {
    let (mut agent, observer) = build_legacy_agent(false);
    let _ = agent
        .run_mission("legacy mission-disabled path", None)
        .await
        .expect("run mission with mission disabled");

    assert!(
        observer.snapshot().iter().all(|event| !matches!(
            event,
            ObserverEvent::MissionTerminated { rollback: true, .. }
        )),
        "rollback telemetry must be absent when no checkpoint resume index exists"
    );
}
