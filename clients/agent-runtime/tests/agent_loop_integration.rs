use async_trait::async_trait;
use corvus::agent::agent::Agent;
use corvus::agent::dispatcher::NativeToolDispatcher;
use corvus::memory::Memory;
use corvus::observability::{NoopObserver, Observer};
use corvus::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
use corvus::tools::{Tool, ToolResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct IntegrationProvider {
    calls: Mutex<Vec<ChatResponse>>,
}

#[async_trait]
impl Provider for IntegrationProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("fallback".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let mut guard = self.calls.lock().unwrap();
        if guard.is_empty() {
            return Ok(ChatResponse {
                text: Some("done".to_string()),
                tool_calls: vec![],
            });
        }

        Ok(guard.remove(0))
    }
}

struct IntegrationTool {
    executions: Arc<AtomicUsize>,
}

#[async_trait]
impl Tool for IntegrationTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
          "type": "object",
          "properties": {
            "message": {"type": "string"}
          }
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResult {
            success: true,
            output: "tool-output".to_string(),
            error: None,
        })
    }
}

#[tokio::test]
async fn full_prompt_tool_response_cycle_with_dummy_provider() {
    let workspace = tempfile::TempDir::new().expect("tempdir");

    let provider = Box::new(IntegrationProvider {
        calls: Mutex::new(vec![
            ChatResponse {
                text: Some("".to_string()),
                tool_calls: vec![ToolCall {
                    id: "tc-1".to_string(),
                    name: "echo".to_string(),
                    arguments: "{\"message\":\"hello\"}".to_string(),
                }],
            },
            ChatResponse {
                text: Some("final integration response".to_string()),
                tool_calls: vec![],
            },
        ]),
    });

    let executions = Arc::new(AtomicUsize::new(0));
    let tool = IntegrationTool {
        executions: Arc::clone(&executions),
    };

    let observer: Arc<dyn Observer> = Arc::new(NoopObserver);
    let memory: Arc<dyn Memory> = Arc::new(corvus::memory::NoneMemory::new());

    let mut agent = Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(tool)])
        .memory(memory)
        .observer(observer)
        .tool_dispatcher(Box::new(NativeToolDispatcher))
        .workspace_dir(workspace.path().to_path_buf())
        .build()
        .unwrap();

    let response = agent.turn("integrate").await.unwrap();

    assert_eq!(response, "final integration response");
    assert_eq!(executions.load(Ordering::SeqCst), 1);

    let history = agent.history();
    assert!(history.iter().any(|item| {
        matches!(
            item,
            corvus::providers::ConversationMessage::AssistantToolCalls { .. }
        )
    }));
    assert!(history.iter().any(|item| {
    matches!(
      item,
      corvus::providers::ConversationMessage::Chat(ChatMessage { role, content }) if role == "assistant" && content == "final integration response"
    )
  }));
}
