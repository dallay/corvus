//! # Delegate Cancel Tool
//!
//! Cancels an active supervised orchestration run identified by `handle`.
//! Use with handles returned by `delegate_launch`.
//!
//! ## Scope
//!
//! This slice is **in-process only**. Peer messaging, remote transport,
//! cross-node isolation, and escalation are **not** supported.

use super::traits::{Tool, ToolResult};
use crate::agent::coordinator::{OrchestrationHandle, SupervisedOrchestrationService};
use async_trait::async_trait;
use std::sync::Arc;

/// Tool that cancels an in-progress orchestration run.
///
/// Accepted JSON input:
/// ```json
/// { "handle": "<opaque handle string returned by delegate_launch>" }
/// ```
///
/// Returns JSON with a `cancel_result` field on success, a not-found result
/// when the handle is unknown, or an error when the service fails.
pub struct DelegateCancelTool {
    service: Arc<SupervisedOrchestrationService>,
}

impl DelegateCancelTool {
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
impl Tool for DelegateCancelTool {
    fn name(&self) -> &str {
        "delegate_cancel"
    }

    fn description(&self) -> &str {
        "Cancel an active supervised orchestration run. \
         Requires the handle returned by delegate_launch. In-process only — \
         peer messaging, remote transport, isolation, and escalation are not supported."
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

        match self.service.cancel(&handle).await {
            Ok(Some(cancel_result)) => {
                let structured = serde_json::json!({ "cancel_result": cancel_result });
                Ok(ToolResult {
                    success: true,
                    output: format!("Cancelled handle {handle_str}"),
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
                error: Some(format!("Cancel failed: {e}")),
                structured: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::coordinator::{
        CancellationReason, ChildAgentId, ChildExecutionResult, ChildLaunchRequest,
        ChildTerminalStatus, CoordinatorChildRunner, CoordinatorError, CoordinatorLaunchRequest,
        CoordinatorMessage, CoordinatorTransport, EnvelopeMeta, FanInPolicy, MessageEnvelope,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;

    /// Immediately completes the child with a successful result.
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
                    coordinator_id: dispatch.meta.coordinator_id,
                    child_id: Some(request.child_id.clone()),
                    sequence: dispatch.meta.sequence,
                    correlation_id: dispatch.meta.correlation_id,
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

    /// Blocks until cancellation is requested, then reports the child as cancelled.
    struct WaitForCancellationRunner;

    #[async_trait]
    impl CoordinatorChildRunner for WaitForCancellationRunner {
        async fn run_child(
            &self,
            request: ChildLaunchRequest,
            dispatch: MessageEnvelope<CoordinatorMessage>,
            cancellation: tokio_util::sync::CancellationToken,
        ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
            cancellation.cancelled().await;
            Ok(MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: dispatch.meta.coordinator_id,
                    child_id: Some(request.child_id.clone()),
                    sequence: dispatch.meta.sequence,
                    correlation_id: dispatch.meta.correlation_id,
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::InProcess,
                },
                payload: CoordinatorMessage::ChildCancelled {
                    reason: CancellationReason::ParentRequested,
                },
            })
        }
    }

    fn service() -> Arc<SupervisedOrchestrationService> {
        Arc::new(SupervisedOrchestrationService::new())
    }

    fn tool(svc: Arc<SupervisedOrchestrationService>) -> DelegateCancelTool {
        DelegateCancelTool::new(svc)
    }

    fn one_child_request() -> CoordinatorLaunchRequest {
        CoordinatorLaunchRequest {
            parent_session_id: None,
            children: vec![ChildLaunchRequest {
                child_id: ChildAgentId("c1".into()),
                agent_name: "AgentA".into(),
                prompt: "do it".into(),
                context: None,
                launch_index: 0,
            }],
            fan_in: FanInPolicy::AllMustSucceed,
        }
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
            .execute(serde_json::json!({ "handle": "ghost-handle" }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("ghost-handle"));
    }

    #[tokio::test]
    async fn cancel_active_run_returns_accepted() {
        let svc = service();
        let runner = Arc::new(WaitForCancellationRunner);

        let receipt = svc.launch(one_child_request(), runner).await.unwrap();
        let handle_str = receipt.handle.0.clone();

        // Give the spawned task time to reach the cancellation-await point.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let t = tool(Arc::clone(&svc));
        let result = t
            .execute(serde_json::json!({ "handle": handle_str }))
            .await
            .unwrap();

        assert!(result.success, "expected success, got: {:?}", result.error);
        let structured = result.structured.unwrap();
        let disposition = &structured["cancel_result"]["disposition"];
        assert_eq!(
            disposition.as_str().unwrap_or(""),
            "accepted",
            "expected accepted disposition"
        );
    }

    #[tokio::test]
    async fn cancel_terminal_run_returns_already_terminal() {
        let svc = service();
        let runner = Arc::new(NoOpRunner);

        let receipt = svc.launch(one_child_request(), runner).await.unwrap();
        let handle_str = receipt.handle.0.clone();

        // First cancel: drives the Active run to Terminal (returns Accepted).
        // NoOpRunner completes instantly, so cancel() awaits the join handle
        // and transitions the registry entry to RunEntry::Terminal.
        let first = svc
            .cancel(&receipt.handle)
            .await
            .expect("first cancel must not fail");
        assert!(first.is_some(), "first cancel must find the run");

        // Second cancel: the entry is now Terminal; service returns AlreadyTerminal.
        let t = tool(Arc::clone(&svc));
        let result = t
            .execute(serde_json::json!({ "handle": handle_str }))
            .await
            .unwrap();

        assert!(result.success, "expected success, got: {:?}", result.error);
        let structured = result.structured.unwrap();
        let disposition = &structured["cancel_result"]["disposition"];
        assert_eq!(
            disposition.as_str().unwrap_or(""),
            "already_terminal",
            "expected already_terminal disposition"
        );
    }
}
