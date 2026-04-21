//! # Delegate Inspect Tool
//!
//! Returns a point-in-time snapshot of a supervised orchestration run
//! identified by `handle`. Use with handles returned by `delegate_launch`.
//!
//! ## Scope
//!
//! This slice is **process-local only**. Mailbox-backed child delivery may cross
//! process boundaries internally, but remote transport, restart recovery,
//! cross-node isolation, and escalation remain unsupported.

use super::traits::{Tool, ToolResult};
use crate::agent::coordinator::{OrchestrationHandle, SupervisedOrchestrationService};
use async_trait::async_trait;
use std::sync::Arc;

/// Tool that returns a snapshot of an in-progress or completed orchestration run.
///
/// Accepted JSON input:
/// ```json
/// { "handle": "<opaque handle string returned by delegate_launch>" }
/// ```
///
/// Returns JSON with a `snapshot` field on success, or a not-found result
/// when the handle is unknown.
pub struct DelegateInspectTool {
    service: Arc<SupervisedOrchestrationService>,
}

impl DelegateInspectTool {
    /// Create a new tool sharing the `service` with other lifecycle tools.
    pub fn new(service: Arc<SupervisedOrchestrationService>) -> Self {
        Self { service }
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
impl Tool for DelegateInspectTool {
    fn name(&self) -> &str {
        "delegate_inspect"
    }

    fn description(&self) -> &str {
        "Return a point-in-time snapshot of a supervised orchestration run. \
         Requires the handle returned by delegate_launch. Process-local only — \
         mailbox-backed internal delivery is supported, but remote transport, recovery, \
         isolation, and escalation are not supported."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["handle"],
            "properties": {
                "handle": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Opaque handle string returned by delegate_launch."
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let handle_str = match args.get("handle").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => {
                return Ok(Self::validation_error(
                    "Missing or empty 'handle' parameter",
                ));
            }
        };

        let handle = OrchestrationHandle(handle_str.clone());

        match self.service.inspect(&handle) {
            Ok(Some(snapshot)) => {
                let structured = serde_json::json!({ "snapshot": snapshot });
                Ok(ToolResult {
                    success: true,
                    output: format!("Snapshot for handle {handle_str}"),
                    error: None,
                    structured: Some(structured),
                })
            }
            Ok(None) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "No orchestration run found for handle '{handle_str}'"
                )),
                structured: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Inspect failed: {e}")),
                structured: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::coordinator::{
        ChildAgentId, ChildExecutionResult, ChildLaunchRequest, ChildTerminalStatus,
        CoordinatorChildRunner, CoordinatorError, CoordinatorLaunchRequest, CoordinatorMessage,
        CoordinatorTransport, EnvelopeMeta, FanInPolicy, MessageEnvelope,
    };
    use crate::agent::mailbox::{MailboxBackedChildRunner, MailboxWakeupHub, SqliteMailboxStore};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;
    use tempfile::TempDir;

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
                    message_id: format!("{}:inspect", dispatch.meta.message_id),
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

    struct GatedRunner {
        started: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl CoordinatorChildRunner for GatedRunner {
        async fn run_child(
            &self,
            request: ChildLaunchRequest,
            dispatch: MessageEnvelope<CoordinatorMessage>,
            _cancellation: tokio_util::sync::CancellationToken,
        ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
            self.started.notify_waiters();
            self.release.notified().await;
            Ok(MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: dispatch.meta.coordinator_id.clone(),
                    child_id: Some(request.child_id.clone()),
                    sequence: dispatch.meta.sequence,
                    message_id: format!("{}:inspect", dispatch.meta.message_id),
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
                    transport: CoordinatorTransport::Mailbox,
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

    fn service() -> Arc<SupervisedOrchestrationService> {
        Arc::new(SupervisedOrchestrationService::new())
    }

    fn tool(svc: Arc<SupervisedOrchestrationService>) -> DelegateInspectTool {
        DelegateInspectTool::new(svc)
    }

    #[tokio::test]
    async fn rejects_missing_handle() {
        let t = tool(service());
        let result = t.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("handle"));
    }

    #[tokio::test]
    async fn rejects_empty_handle() {
        let t = tool(service());
        let result = t
            .execute(serde_json::json!({ "handle": "   " }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("handle"));
    }

    #[tokio::test]
    async fn returns_not_found_for_unknown_handle() {
        let t = tool(service());
        let result = t
            .execute(serde_json::json!({ "handle": "does-not-exist" }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("does-not-exist"));
    }

    #[tokio::test]
    async fn returns_snapshot_for_known_handle() {
        let svc = service();
        let runner = Arc::new(NoOpRunner);

        let request = CoordinatorLaunchRequest {
            parent_session_id: None,
            children: vec![ChildLaunchRequest {
                child_id: ChildAgentId("c1".into()),
                agent_name: "AgentA".into(),
                prompt: "do it".into(),
                context: None,
                launch_index: 0,
            }],
            fan_in: FanInPolicy::AllMustSucceed,
        };

        let receipt = svc.launch(request, runner).await.unwrap();
        let handle_str = receipt.handle.0.clone();

        let t = tool(Arc::clone(&svc));
        let result = t
            .execute(serde_json::json!({ "handle": handle_str }))
            .await
            .unwrap();

        assert!(result.success, "expected success, got: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert!(structured["snapshot"].is_object());
    }

    #[tokio::test]
    async fn mailbox_backed_inspect_remains_process_local() {
        let tmp = TempDir::new().unwrap();
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(NoOpRunner),
            Arc::new(MailboxWakeupHub::default()),
        ));

        let original_service = Arc::new(SupervisedOrchestrationService::new());
        let request = CoordinatorLaunchRequest {
            parent_session_id: None,
            children: vec![ChildLaunchRequest {
                child_id: ChildAgentId("c1".into()),
                agent_name: "AgentA".into(),
                prompt: "do it".into(),
                context: None,
                launch_index: 0,
            }],
            fan_in: FanInPolicy::AllMustSucceed,
        };

        let receipt = original_service.launch(request, runner).await.unwrap();
        let result = tool(Arc::new(SupervisedOrchestrationService::new()))
            .execute(serde_json::json!({ "handle": receipt.handle.0 }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("No orchestration run found"));
    }

    #[tokio::test]
    async fn mailbox_backed_inspect_returns_snapshot_for_owning_service() {
        let tmp = TempDir::new().unwrap();
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(GatedRunner {
                started: started.clone(),
                release: release.clone(),
            }),
            Arc::new(MailboxWakeupHub::default()),
        ));

        let svc = Arc::new(SupervisedOrchestrationService::new());
        let request = CoordinatorLaunchRequest {
            parent_session_id: None,
            children: vec![ChildLaunchRequest {
                child_id: ChildAgentId("c1".into()),
                agent_name: "AgentA".into(),
                prompt: "do it".into(),
                context: None,
                launch_index: 0,
            }],
            fan_in: FanInPolicy::AllMustSucceed,
        };

        let receipt = svc.launch(request, runner).await.unwrap();
        started.notified().await;

        let result = tool(Arc::clone(&svc))
            .execute(serde_json::json!({ "handle": receipt.handle.0.clone() }))
            .await
            .unwrap();

        assert!(result.success, "expected success, got: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert_eq!(structured["snapshot"]["handle"], receipt.handle.0);

        release.notify_waiters();
    }
}
