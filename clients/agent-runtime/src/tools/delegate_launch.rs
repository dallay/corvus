//! # Delegate Launch Tool
//!
//! Launches a supervised multi-child orchestration run via
//! [`SupervisedOrchestrationService`]. Returns a stable handle and an initial
//! snapshot that callers can use with `delegate_inspect` and `delegate_cancel`.
//!
//! ## Scope
//!
//! This slice is **process-local only**. Mailbox-backed delivery may cross process
//! boundaries internally, but peer messaging, remote transport, restart recovery,
//! and escalation remain out of scope and validation errors for callers.

use super::traits::{Tool, ToolResult};
use crate::agent::coordinator::{
    ChildAgentId, ChildLaunchRequest, CoordinatorChildRunner, CoordinatorLaunchRequest,
    FanInPolicy, SupervisedOrchestrationService,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Tool that launches a supervised orchestration run for one or more children.
///
/// Accepted JSON input:
/// ```json
/// {
///   "children": [
///     { "child_id": "a", "agent_name": "AgentA", "prompt": "do X" }
///   ]
/// }
/// ```
///
/// Returns JSON with `handle` and `snapshot` fields on success.
pub struct DelegateLaunchTool {
    service: Arc<SupervisedOrchestrationService>,
    runner: Arc<dyn CoordinatorChildRunner>,
}

impl DelegateLaunchTool {
    /// Create a new tool sharing `service` and `runner` with other lifecycle tools.
    pub fn new(
        service: Arc<SupervisedOrchestrationService>,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Self {
        Self { service, runner }
    }

    fn validation_error(msg: impl Into<String>) -> ToolResult {
        ToolResult {
            success: false,
            output: String::new(),
            error: Some(msg.into()),
            structured: None,
        }
    }
}

#[async_trait]
impl Tool for DelegateLaunchTool {
    fn name(&self) -> &str {
        "delegate_launch"
    }

    fn description(&self) -> &str {
        "Launch a supervised multi-child orchestration run. Returns a handle and initial snapshot \
         usable with delegate_inspect and delegate_cancel. Process-local only — mailbox-backed \
         internal delivery is supported, but remote transport, recovery, isolation, and \
         escalation are not supported."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "children": {
                    "type": "array",
                    "minItems": 1,
                    "description": "One or more child agent launch descriptors.",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["child_id", "agent_name", "prompt"],
                        "properties": {
                            "child_id": {
                                "type": "string",
                                "minLength": 1,
                                "description": "Unique identifier for this child within the run."
                            },
                            "agent_name": {
                                "type": "string",
                                "minLength": 1,
                                "description": "Name of the agent configuration to use."
                            },
                            "prompt": {
                                "type": "string",
                                "minLength": 1,
                                "description": "Task prompt for this child."
                            },
                            "context": {
                                "type": "string",
                                "description": "Optional context to prepend to the child's prompt."
                            }
                        }
                    }
                }
            },
            "required": ["children"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let children_val = match args.get("children") {
            Some(v) => v,
            None => {
                return Ok(Self::validation_error("Missing 'children' parameter"));
            }
        };

        let children_arr = match children_val.as_array() {
            Some(a) if !a.is_empty() => a,
            _ => {
                return Ok(Self::validation_error(
                    "'children' must be a non-empty array",
                ));
            }
        };

        let mut child_requests: Vec<ChildLaunchRequest> = Vec::with_capacity(children_arr.len());
        let mut seen_ids: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(children_arr.len());

        for (launch_index, item) in children_arr.iter().enumerate() {
            if item.get("stream").is_some()
                || item.get("stream_results").is_some()
                || item.get("stream_tool_progress").is_some()
            {
                return Ok(Self::validation_error(
                    "streaming payloads remain out of scope for this slice",
                ));
            }

            let child_id = match item.get("child_id").and_then(|v| v.as_str()) {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => {
                    return Ok(Self::validation_error(format!(
                        "Child at index {launch_index} is missing a non-empty 'child_id'"
                    )));
                }
            };

            if !seen_ids.insert(child_id.clone()) {
                return Ok(Self::validation_error(format!(
                    "Duplicate child_id '{child_id}'"
                )));
            }

            let agent_name = match item.get("agent_name").and_then(|v| v.as_str()) {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => {
                    return Ok(Self::validation_error(format!(
                        "Child '{child_id}' is missing a non-empty 'agent_name'"
                    )));
                }
            };

            let prompt = match item.get("prompt").and_then(|v| v.as_str()) {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => {
                    return Ok(Self::validation_error(format!(
                        "Child '{child_id}' is missing a non-empty 'prompt'"
                    )));
                }
            };

            let context = item
                .get("context")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            child_requests.push(ChildLaunchRequest {
                child_id: ChildAgentId(child_id),
                agent_name,
                prompt,
                context,
                launch_index: u32::try_from(launch_index).unwrap_or(u32::MAX),
            });
        }

        let request = CoordinatorLaunchRequest {
            parent_session_id: None,
            children: child_requests,
            fan_in: FanInPolicy::AllMustSucceed,
        };

        let receipt = match self.service.launch(request, Arc::clone(&self.runner)).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Launch failed: {e}")),
                    structured: None,
                });
            }
        };

        let structured = serde_json::json!({
            "handle": receipt.handle,
            "snapshot": receipt.snapshot,
        });

        Ok(ToolResult {
            success: true,
            output: format!("Launched orchestration run {}", receipt.handle.0),
            error: None,
            structured: Some(structured),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::coordinator::{
        ChildExecutionResult, ChildLaunchRequest, ChildTerminalStatus, CoordinatorChildRunner,
        CoordinatorError, CoordinatorMessage, CoordinatorTransport, EnvelopeMeta, MessageEnvelope,
    };
    use crate::agent::mailbox::{MailboxBackedChildRunner, MailboxWakeupHub, SqliteMailboxStore};
    use async_trait::async_trait;
    use chrono::Utc;
    use tempfile::TempDir;

    // Minimal no-op runner for validation tests (launch never runs children in
    // these tests because all tests exercise validation paths that return before
    // dispatching children).
    struct NoOpRunner;

    #[async_trait]
    impl CoordinatorChildRunner for NoOpRunner {
        async fn run_child(
            &self,
            request: ChildLaunchRequest,
            dispatch: MessageEnvelope<CoordinatorMessage>,
            _cancellation: tokio_util::sync::CancellationToken,
        ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
            Ok(MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: dispatch.meta.coordinator_id.clone(),
                    child_id: Some(request.child_id.clone()),
                    sequence: dispatch.meta.sequence,
                    message_id: format!("{}:launch", dispatch.meta.message_id),
                    correlation_id: dispatch.meta.correlation_id.clone(),
                    sender: crate::agent::mailbox::LogicalEndpoint::child(
                        dispatch.meta.coordinator_id.clone(),
                        request.child_id.clone(),
                    ),
                    recipient: crate::agent::mailbox::LogicalEndpoint::coordinator_child(
                        dispatch.meta.coordinator_id.clone(),
                        request.child_id.clone(),
                    ),
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::InProcess,
                },
                payload: CoordinatorMessage::ChildCompleted {
                    result: ChildExecutionResult {
                        session_id: request.child_id.0.clone(),
                        tool_result: crate::tools::traits::ToolResult {
                            success: true,
                            output: "done".into(),
                            error: None,
                            structured: None,
                        },
                        status: ChildTerminalStatus::Succeeded,
                    },
                },
            })
        }
    }

    fn tool() -> DelegateLaunchTool {
        DelegateLaunchTool::new(
            Arc::new(SupervisedOrchestrationService::new()),
            Arc::new(NoOpRunner),
        )
    }

    #[tokio::test]
    async fn rejects_missing_children_field() {
        let result = tool().execute(serde_json::json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("'children'"));
    }

    #[tokio::test]
    async fn rejects_empty_children_array() {
        let result = tool()
            .execute(serde_json::json!({ "children": [] }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("non-empty array"));
    }

    #[tokio::test]
    async fn rejects_duplicate_child_id() {
        let result = tool()
            .execute(serde_json::json!({
                "children": [
                    { "child_id": "a", "agent_name": "AgentA", "prompt": "p1" },
                    { "child_id": "a", "agent_name": "AgentB", "prompt": "p2" }
                ]
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Duplicate child_id"));
    }

    #[tokio::test]
    async fn rejects_empty_child_id() {
        let result = tool()
            .execute(serde_json::json!({
                "children": [
                    { "child_id": "", "agent_name": "AgentA", "prompt": "p" }
                ]
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn mailbox_backed_launch_keeps_handle_and_snapshot_contract() {
        let tmp = TempDir::new().unwrap();
        let service = Arc::new(SupervisedOrchestrationService::new());
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(NoOpRunner),
            Arc::new(MailboxWakeupHub::default()),
        ));
        let tool = DelegateLaunchTool::new(service, runner);

        let result = tool
            .execute(serde_json::json!({
                "children": [
                    { "child_id": "a", "agent_name": "AgentA", "prompt": "p" }
                ]
            }))
            .await
            .unwrap();

        assert!(result.success);
        let structured = result.structured.expect("structured payload");
        assert!(structured["handle"].is_string());
        assert!(structured["snapshot"].is_object());
    }

    #[tokio::test]
    async fn rejects_streaming_payload_requests_as_out_of_scope() {
        let tmp = TempDir::new().unwrap();
        let service = Arc::new(SupervisedOrchestrationService::new());
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(NoOpRunner),
            Arc::new(MailboxWakeupHub::default()),
        ));
        let tool = DelegateLaunchTool::new(service, runner);

        let result = tool
            .execute(serde_json::json!({
                "children": [
                    {
                        "child_id": "a",
                        "agent_name": "AgentA",
                        "prompt": "p",
                        "stream": true
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("streaming payloads remain out of scope"));
    }

    #[tokio::test]
    async fn rejects_empty_prompt() {
        let result = tool()
            .execute(serde_json::json!({
                "children": [
                    { "child_id": "a", "agent_name": "AgentA", "prompt": "" }
                ]
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
