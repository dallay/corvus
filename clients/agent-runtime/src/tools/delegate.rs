//! # Delegate Tool
//!
//! Delegates a subtask to a named sub-agent configured in `config.agents`.
//!
//! ## Execution Modes
//!
//! - **OneShot** (default): A single LLM call via provider. No tool loop.
//! - **Session**: Launches a bounded child `Agent` in code mode with full tool iteration.
//!   The child agent runs until it emits a `FINAL RESULT` block, hits the iteration budget,
//!   or times out.
//!
//! ## Security Boundary
//!
//! Child sessions run through the same `SecurityPolicy` stack as direct sessions.
//! There is no bypass path for delegated sessions — the child `Agent` is bootstrapped
//! via `Agent::code_from_config` which applies the same policy defaults.
//!
//! ## Rollback
//!
//! To revert Session mode: remove the `DelegateExecutionMode::Session` branch in
//! `execute()` and the `run_session()` helper. The `OneShot` path is unmodified and
//! fully backward-compatible.
//!
//! ## Config Inheritance
//!
//! Child sessions clone the parent `Config` and apply overrides from `DelegateAgentConfig`,
//! preserving policy, workspace, and audit settings.

use super::traits::{Tool, ToolResult};
use crate::agent::coordinator::{
    ChildAgentId, ChildLaunchRequest, Coordinator, CoordinatorChildOutcome, CoordinatorChildRunner,
    CoordinatorLaunchRequest, CoordinatorOutcome, DelegatedAgentRunner, FanInPolicy,
    SupervisedOrchestrationService,
};
use crate::config::{Config, DelegateAgentConfig, DelegateExecutionMode};
use crate::providers::{self, Provider};
use crate::security::policy::ToolOperation;
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Default timeout for sub-agent provider calls.
const DELEGATE_TIMEOUT_SECS: u64 = 120;

#[async_trait]
trait SessionCoordinatorExecutor: Send + Sync {
    async fn execute(
        &self,
        request: CoordinatorLaunchRequest,
        base_config: Arc<Config>,
        agents: Arc<HashMap<String, DelegateAgentConfig>>,
        fallback_credential: Option<String>,
    ) -> Result<CoordinatorOutcome, anyhow::Error>;
}

struct DefaultSessionCoordinatorExecutor;

#[async_trait]
impl SessionCoordinatorExecutor for DefaultSessionCoordinatorExecutor {
    async fn execute(
        &self,
        request: CoordinatorLaunchRequest,
        base_config: Arc<Config>,
        agents: Arc<HashMap<String, DelegateAgentConfig>>,
        fallback_credential: Option<String>,
    ) -> Result<CoordinatorOutcome, anyhow::Error> {
        let coordinator = Coordinator::new();
        let runner = Arc::new(DelegatedAgentRunner::new(
            base_config,
            agents,
            fallback_credential,
        ));
        coordinator.run(request, runner).await.map_err(Into::into)
    }
}

/// Routes a delegate session through [`SupervisedOrchestrationService::run_to_completion`].
///
/// This executor is used in production when the tool registry is built with a shared
/// `SupervisedOrchestrationService`, enabling lifecycle tools (`delegate_launch`,
/// `delegate_cancel`, `delegate_inspect`) to observe and control the same in-process runs.
///
/// ## Scope
///
/// Process-local only. Child agents may exchange internal envelopes through the
/// mailbox-backed transport, but remote bridge delivery and recovery remain out of scope.
struct SupervisedSessionCoordinatorExecutor {
    service: Arc<SupervisedOrchestrationService>,
    runner: Arc<dyn CoordinatorChildRunner>,
}

#[async_trait]
impl SessionCoordinatorExecutor for SupervisedSessionCoordinatorExecutor {
    async fn execute(
        &self,
        request: CoordinatorLaunchRequest,
        _base_config: Arc<Config>,
        _agents: Arc<HashMap<String, DelegateAgentConfig>>,
        _fallback_credential: Option<String>,
    ) -> Result<CoordinatorOutcome, anyhow::Error> {
        self.service
            .run_to_completion(request, self.runner.clone())
            .await
            .map_err(Into::into)
    }
}

/// Tool that delegates a subtask to a named agent with a different
/// provider/model configuration. Enables multi-agent workflows where
/// a primary agent can hand off specialized work (research, coding,
/// summarization) to purpose-built sub-agents.
pub struct DelegateTool {
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    security: Arc<SecurityPolicy>,
    /// Global credential fallback (from config.api_key)
    fallback_credential: Option<String>,
    /// Depth at which this tool instance lives in the delegation chain.
    depth: u32,
    base_config: Arc<Config>,
    session_executor: Arc<dyn SessionCoordinatorExecutor>,
}

impl DelegateTool {
    pub fn new(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        base_config: Arc<Config>,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            depth: 0,
            base_config,
            session_executor: Arc::new(DefaultSessionCoordinatorExecutor),
        }
    }

    /// Create a DelegateTool for a sub-agent (with incremented depth).
    /// When sub-agents eventually get their own tool registry, construct
    /// their DelegateTool via this method with `depth: parent.depth + 1`.
    pub fn with_depth(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        depth: u32,
        base_config: Arc<Config>,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            depth,
            base_config,
            session_executor: Arc::new(DefaultSessionCoordinatorExecutor),
        }
    }

    #[cfg(test)]
    fn with_session_executor(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        base_config: Arc<Config>,
        session_executor: Arc<dyn SessionCoordinatorExecutor>,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            depth: 0,
            base_config,
            session_executor,
        }
    }

    /// Create a `DelegateTool` wired to a shared [`SupervisedOrchestrationService`].
    ///
    /// Use this constructor in the production tool registry so that `DelegateTool`
    /// runs child sessions through the same supervised service that the lifecycle
    /// tools (`delegate_launch`, `delegate_cancel`, `delegate_inspect`) observe.
    ///
    /// ## Scope
    ///
    /// Process-local only — child agents may use mailbox-backed internal delivery,
    /// but remote transport and recovery remain unsupported.
    pub fn with_supervised_executor(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        base_config: Arc<Config>,
        service: Arc<SupervisedOrchestrationService>,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            depth: 0,
            base_config,
            session_executor: Arc::new(SupervisedSessionCoordinatorExecutor { service, runner }),
        }
    }

    fn fail_closed_session_result(agent_name: &str, message: impl Into<String>) -> ToolResult {
        ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Agent '{agent_name}' session failed closed: {}",
                message.into()
            )),
            structured: None,
        }
    }

    fn session_result_from_child_outcome(
        agent_name: &str,
        outcome: &CoordinatorChildOutcome,
    ) -> ToolResult {
        match outcome {
            CoordinatorChildOutcome::Succeeded { result, .. } => result.tool_result.clone(),
            CoordinatorChildOutcome::Failed { error, .. } => {
                error.tool_result.clone().unwrap_or_else(|| {
                    Self::fail_closed_session_result(agent_name, error.error.clone())
                })
            }
            CoordinatorChildOutcome::Cancelled { reason, .. } => {
                Self::fail_closed_session_result(agent_name, format!("cancelled: {reason:?}"))
            }
        }
    }

    fn session_result_from_outcome(agent_name: &str, outcome: CoordinatorOutcome) -> ToolResult {
        match outcome {
            CoordinatorOutcome::Completed { children, .. } => children
                .first()
                .map(|child| Self::session_result_from_child_outcome(agent_name, child))
                .unwrap_or_else(|| {
                    Self::fail_closed_session_result(
                        agent_name,
                        "coordinator completed without a child outcome",
                    )
                }),
            CoordinatorOutcome::Failed {
                error, children, ..
            } => children
                .first()
                .map(|child| Self::session_result_from_child_outcome(agent_name, child))
                .unwrap_or_else(|| Self::fail_closed_session_result(agent_name, error)),
            CoordinatorOutcome::Cancelled {
                reason, children, ..
            } => children
                .first()
                .map(|child| Self::session_result_from_child_outcome(agent_name, child))
                .unwrap_or_else(|| {
                    Self::fail_closed_session_result(agent_name, format!("cancelled: {reason:?}"))
                }),
        }
    }

    /// Run a delegated sub-agent in Session (full tool-loop) mode through the
    /// coordinator seam using a single-child launch request.
    async fn run_session(
        &self,
        agent_name: &str,
        prompt: &str,
        context: Option<&str>,
    ) -> anyhow::Result<ToolResult> {
        let request = CoordinatorLaunchRequest {
            parent_session_id: None,
            children: vec![ChildLaunchRequest {
                child_id: ChildAgentId(agent_name.to_string()),
                agent_name: agent_name.to_string(),
                prompt: prompt.to_string(),
                context: context.map(ToOwned::to_owned),
                launch_index: 0,
                execution: None,
            }],
            fan_in: FanInPolicy::AllMustSucceed,
        };

        let outcome = self
            .session_executor
            .execute(
                request,
                self.base_config.clone(),
                self.agents.clone(),
                self.fallback_credential.clone(),
            )
            .await;

        Ok(match outcome {
            Ok(outcome) => Self::session_result_from_outcome(agent_name, outcome),
            Err(error) => Self::fail_closed_session_result(agent_name, error.to_string()),
        })
    }
}

#[async_trait]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "Delegate a subtask to a specialized agent. Use when: a task benefits from a different model \
         (e.g. fast summarization, deep reasoning, code generation). The sub-agent runs a single \
         prompt and returns its response."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        let agent_names: Vec<&str> = self.agents.keys().map(|s: &String| s.as_str()).collect();
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "agent": {
                    "type": "string",
                    "minLength": 1,
                    "description": format!(
                        "Name of the agent to delegate to. Available: {}",
                        if agent_names.is_empty() {
                            "(none configured)".to_string()
                        } else {
                            agent_names.join(", ")
                        }
                    )
                },
                "prompt": {
                    "type": "string",
                    "minLength": 1,
                    "description": "The task/prompt to send to the sub-agent"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context to prepend (e.g. relevant code, prior findings)"
                }
            },
            "required": ["agent", "prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let agent_name = args
            .get("agent")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .ok_or_else(|| anyhow::anyhow!("Missing 'agent' parameter"))?;

        if agent_name.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("'agent' parameter must not be empty".into()),
                structured: None,
            });
        }

        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;

        if prompt.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("'prompt' parameter must not be empty".into()),
                structured: None,
            });
        }

        let context = args
            .get("context")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");

        // Look up agent config
        let agent_config = match self.agents.get(agent_name) {
            Some(cfg) => cfg,
            None => {
                let available: Vec<&str> =
                    self.agents.keys().map(|s: &String| s.as_str()).collect();
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Unknown agent '{agent_name}'. Available agents: {}",
                        if available.is_empty() {
                            "(none configured)".to_string()
                        } else {
                            available.join(", ")
                        }
                    )),
                    structured: None,
                });
            }
        };

        // Check recursion depth (immutable — set at construction, incremented for sub-agents)
        if self.depth >= agent_config.max_depth {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Delegation depth limit reached ({depth}/{max}). \
                     Cannot delegate further to prevent infinite loops.",
                    depth = self.depth,
                    max = agent_config.max_depth
                )),
                structured: None,
            });
        }

        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, "delegate")
        {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
                structured: None,
            });
        }

        // Dispatch to Session or OneShot based on the agent's execution_mode
        if agent_config.execution_mode == DelegateExecutionMode::Session {
            return self
                .run_session(agent_name, prompt, (!context.is_empty()).then_some(context))
                .await;
        }

        // Create provider for this agent
        let provider_credential_owned = agent_config
            .api_key
            .clone()
            .or_else(|| self.fallback_credential.clone());
        #[allow(clippy::option_as_ref_deref)]
        let provider_credential = provider_credential_owned.as_ref().map(String::as_str);

        let provider: Box<dyn Provider> =
            match providers::create_provider(&agent_config.provider, provider_credential) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "Failed to create provider '{}' for agent '{agent_name}': {e}",
                            agent_config.provider
                        )),
                        structured: None,
                    });
                }
            };

        // Build the message
        let full_prompt = if context.is_empty() {
            prompt.to_string()
        } else {
            format!("[Context]\n{context}\n\n[Task]\n{prompt}")
        };

        let temperature = agent_config.temperature.unwrap_or(0.7);

        // Wrap the provider call in a timeout to prevent indefinite blocking
        let result = tokio::time::timeout(
            Duration::from_secs(DELEGATE_TIMEOUT_SECS),
            provider.chat_with_system(
                agent_config.system_prompt.as_deref(),
                &full_prompt,
                &agent_config.model,
                temperature,
            ),
        )
        .await;

        let result = match result {
            Ok(inner) => inner,
            Err(_elapsed) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Agent '{agent_name}' timed out after {DELEGATE_TIMEOUT_SECS}s"
                    )),
                    structured: None,
                });
            }
        };

        match result {
            Ok(response) => {
                let mut rendered = response;
                if rendered.trim().is_empty() {
                    rendered = "[Empty response]".to_string();
                }

                Ok(ToolResult {
                    success: true,
                    output: format!(
                        "[Agent '{agent_name}' ({provider}/{model})]\n{rendered}",
                        provider = agent_config.provider,
                        model = agent_config.model
                    ),
                    error: None,
                    structured: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Agent '{agent_name}' failed: {e}")),
                structured: None,
            }),
        }
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
    use crate::config::{Config, DelegateExecutionMode};
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;
    use tokio::sync::Mutex as AsyncMutex;

    struct StubSessionCoordinatorExecutor {
        requests: Arc<AsyncMutex<Vec<CoordinatorLaunchRequest>>>,
        outcome: CoordinatorOutcome,
    }

    impl StubSessionCoordinatorExecutor {
        fn new(outcome: CoordinatorOutcome) -> Self {
            Self {
                requests: Arc::new(AsyncMutex::new(Vec::new())),
                outcome,
            }
        }

        async fn recorded_requests(&self) -> Vec<CoordinatorLaunchRequest> {
            self.requests.lock().await.clone()
        }
    }

    #[async_trait]
    impl SessionCoordinatorExecutor for StubSessionCoordinatorExecutor {
        async fn execute(
            &self,
            request: CoordinatorLaunchRequest,
            _base_config: Arc<Config>,
            _agents: Arc<HashMap<String, DelegateAgentConfig>>,
            _fallback_credential: Option<String>,
        ) -> Result<CoordinatorOutcome, anyhow::Error> {
            self.requests.lock().await.push(request);
            Ok(self.outcome.clone())
        }
    }

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default())
    }

    fn test_base_config(tmp: &TempDir) -> Arc<Config> {
        let mut config = crate::test_support::test_config(tmp);
        config.memory.backend = "none".to_string();
        Arc::new(config)
    }

    fn sample_agents() -> HashMap<String, DelegateAgentConfig> {
        let mut agents = HashMap::new();
        agents.insert(
            "researcher".to_string(),
            DelegateAgentConfig {
                provider: "ollama".to_string(),
                model: "llama3".to_string(),
                system_prompt: Some("You are a research assistant.".to_string()),
                api_key: None,
                temperature: Some(0.3),
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );
        agents.insert(
            "coder".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("delegate-test-credential".to_string()),
                temperature: None,
                max_depth: 2,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );
        agents
    }

    #[test]
    fn name_and_schema() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        assert_eq!(tool.name(), "delegate");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["agent"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
        assert!(schema["properties"]["context"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("agent")));
        assert!(required.contains(&json!("prompt")));
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["properties"]["agent"]["minLength"], json!(1));
        assert_eq!(schema["properties"]["prompt"]["minLength"], json!(1));
    }

    #[test]
    fn description_not_empty() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn schema_lists_agent_names() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let schema = tool.parameters_schema();
        let desc = schema["properties"]["agent"]["description"]
            .as_str()
            .unwrap();
        assert!(desc.contains("researcher") || desc.contains("coder"));
    }

    #[tokio::test]
    async fn missing_agent_param() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool.execute(json!({"prompt": "test"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_prompt_param() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool.execute(json!({"agent": "researcher"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unknown_agent_returns_error() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "nonexistent", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown agent"));
    }

    #[tokio::test]
    async fn depth_limit_enforced() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::with_depth(
            sample_agents(),
            None,
            test_security(),
            3,
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "researcher", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("depth limit"));
    }

    #[tokio::test]
    async fn depth_limit_per_agent() {
        // coder has max_depth=2, so depth=2 should be blocked
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::with_depth(
            sample_agents(),
            None,
            test_security(),
            2,
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "coder", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("depth limit"));
    }

    #[test]
    fn empty_agents_schema() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            HashMap::new(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let schema = tool.parameters_schema();
        let desc = schema["properties"]["agent"]["description"]
            .as_str()
            .unwrap();
        assert!(desc.contains("none configured"));
    }

    #[tokio::test]
    async fn invalid_provider_returns_error() {
        let mut agents = HashMap::new();
        agents.insert(
            "broken".to_string(),
            DelegateAgentConfig {
                provider: "totally-invalid-provider".to_string(),
                model: "model".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "broken", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Failed to create provider"));
    }

    #[tokio::test]
    async fn blank_agent_rejected() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "  ", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("must not be empty"));
    }

    #[tokio::test]
    async fn blank_prompt_rejected() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "researcher", "prompt": "  \t  "}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("must not be empty"));
    }

    #[tokio::test]
    async fn whitespace_agent_name_trimmed_and_found() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            sample_agents(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        // " researcher " with surrounding whitespace — after trim becomes "researcher"
        let result = tool
            .execute(json!({"agent": " researcher ", "prompt": "test"}))
            .await
            .unwrap();
        // Should find "researcher" after trim — will fail at provider level
        // since ollama isn't running, but must NOT get "Unknown agent".
        assert!(
            result.error.is_none()
                || !result
                    .error
                    .as_deref()
                    .unwrap_or("")
                    .contains("Unknown agent")
        );
    }

    #[tokio::test]
    async fn delegation_blocked_in_readonly_mode() {
        let readonly = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(sample_agents(), None, readonly, test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "researcher", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("read-only mode"));
    }

    #[tokio::test]
    async fn delegation_blocked_when_rate_limited() {
        let limited = Arc::new(SecurityPolicy {
            max_actions_per_hour: 0,
            ..SecurityPolicy::default()
        });
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(sample_agents(), None, limited, test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "researcher", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Rate limit exceeded"));
    }

    #[tokio::test]
    async fn delegate_context_is_prepended_to_prompt() {
        let mut agents = HashMap::new();
        agents.insert(
            "tester".to_string(),
            DelegateAgentConfig {
                provider: "invalid-for-test".to_string(),
                model: "test-model".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({
                "agent": "tester",
                "prompt": "do something",
                "context": "some context data"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Failed to create provider"));
    }

    #[tokio::test]
    async fn delegate_empty_context_omits_prefix() {
        let mut agents = HashMap::new();
        agents.insert(
            "tester".to_string(),
            DelegateAgentConfig {
                provider: "invalid-for-test".to_string(),
                model: "test-model".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({
                "agent": "tester",
                "prompt": "do something",
                "context": ""
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Failed to create provider"));
    }

    #[test]
    fn delegate_depth_construction() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::with_depth(
            sample_agents(),
            None,
            test_security(),
            5,
            test_base_config(&tmp),
        );
        assert_eq!(tool.depth, 5);
    }

    #[tokio::test]
    async fn delegate_no_agents_configured() {
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(
            HashMap::new(),
            None,
            test_security(),
            test_base_config(&tmp),
        );
        let result = tool
            .execute(json!({"agent": "any", "prompt": "test"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("none configured"));
    }

    // ── Task 3.1: Session mode RED tests ──────────────────────────

    /// Session mode agent with an invalid provider must return a graceful error
    /// (not panic) before any agent loop is entered.
    #[tokio::test]
    async fn session_mode_invalid_provider_returns_error() {
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "totally-invalid-provider".to_string(),
                model: "model".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();
        assert!(!result.success);
        // Must fail at provider/config level — not at "Unknown agent"
        assert!(
            result
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to create provider")
                || result.error.as_deref().unwrap_or("").contains("session")
                || result.error.as_deref().unwrap_or("").contains("provider")
                || result.error.as_deref().unwrap_or("").contains("config"),
            "unexpected error: {:?}",
            result.error
        );
    }

    #[tokio::test]
    async fn session_mode_routes_through_single_child_coordinator_request() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let executor = Arc::new(StubSessionCoordinatorExecutor::new(
            CoordinatorOutcome::Completed {
                coordinator_id: "coord-1".to_string(),
                children: vec![],
            },
        ));
        let tool = DelegateTool::with_session_executor(
            agents,
            None,
            test_security(),
            test_base_config(&tmp),
            executor.clone(),
        );

        let _ = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await;

        let requests = executor.recorded_requests().await;
        assert_eq!(
            requests.len(),
            1,
            "session mode must delegate through the coordinator seam"
        );
        let request = &requests[0];
        assert_eq!(request.children.len(), 1);
        assert_eq!(request.fan_in, FanInPolicy::AllMustSucceed);
        assert_eq!(request.children[0].agent_name, "code_agent");
        assert_eq!(request.children[0].launch_index, 0);
        assert!(request.children[0].context.is_none());
    }

    #[tokio::test]
    async fn session_mode_preserves_single_child_tool_result_contract_from_coordinator_outcome() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let expected = ToolResult {
            success: false,
            output: "[Agent 'code_agent' session (openrouter/anthropic/claude-sonnet-4-20250514)]\nFINAL RESULT: blocked".to_string(),
            error: Some("blocked by policy".to_string()),
            structured: Some(json!({"status": "error", "summary": "blocked by policy"})),
        };
        let executor = Arc::new(StubSessionCoordinatorExecutor::new(
            CoordinatorOutcome::Failed {
                coordinator_id: "coord-2".to_string(),
                error: "blocked by policy".to_string(),
                children: vec![CoordinatorChildOutcome::Failed {
                    child_id: ChildAgentId("code_agent".to_string()),
                    launch_index: 0,
                    error: crate::agent::coordinator::ChildExecutionError {
                        session_id: Some("session-1".to_string()),
                        error: "blocked by policy".to_string(),
                        tool_result: Some(expected.clone()),
                    },
                }],
            },
        ));
        let tool = DelegateTool::with_session_executor(
            agents,
            None,
            test_security(),
            test_base_config(&tmp),
            executor,
        );

        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();

        assert_eq!(result.success, expected.success);
        assert_eq!(result.output, expected.output);
        assert_eq!(result.error, expected.error);
        assert_eq!(result.structured, expected.structured);
    }

    struct SuccessfulRunner;

    #[async_trait]
    impl CoordinatorChildRunner for SuccessfulRunner {
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
                    message_id: format!("reply-{}", request.child_id.0),
                    correlation_id: dispatch.meta.correlation_id.clone(),
                    sender: crate::agent::mailbox::LogicalEndpoint::child(
                        dispatch.meta.coordinator_id.clone(),
                        request.child_id.clone(),
                    ),
                    recipient: crate::agent::mailbox::LogicalEndpoint::coordinator_child(
                        dispatch.meta.coordinator_id.clone(),
                        request.child_id.clone(),
                    ),
                    sent_at: chrono::Utc::now(),
                    transport: CoordinatorTransport::Mailbox,
                },
                payload: CoordinatorMessage::ChildCompleted {
                    result: ChildExecutionResult {
                        session_id: format!("session-{}", request.child_id.0),
                        tool_result: ToolResult {
                            success: true,
                            output: "mailbox-session-ok".to_string(),
                            error: None,
                            structured: Some(json!({"mode": "mailbox"})),
                        },
                        status: ChildTerminalStatus::Succeeded,
                    },
                },
            })
        }
    }

    #[tokio::test]
    async fn supervised_session_mode_keeps_delegate_contract_with_mailbox_runner() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );

        let service = Arc::new(SupervisedOrchestrationService::new());
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(SuccessfulRunner),
            Arc::new(MailboxWakeupHub::default()),
        ));
        let tool = DelegateTool::with_supervised_executor(
            agents,
            None,
            test_security(),
            test_base_config(&tmp),
            service,
            runner,
        );

        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "mailbox-session-ok");
        assert_eq!(result.structured, Some(json!({"mode": "mailbox"})));
    }

    #[tokio::test]
    async fn supervised_single_child_delegate_remains_compatible_with_shared_orchestration_contract(
    ) {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );

        let service = Arc::new(SupervisedOrchestrationService::new());
        let mailbox = Arc::new(
            SqliteMailboxStore::from_db_path(tmp.path().join("state/orchestration/mailbox.db"))
                .unwrap(),
        );
        let runner: Arc<dyn CoordinatorChildRunner> = Arc::new(MailboxBackedChildRunner::new(
            mailbox,
            Arc::new(SuccessfulRunner),
            Arc::new(MailboxWakeupHub::default()),
        ));
        let tool = DelegateTool::with_supervised_executor(
            agents,
            None,
            test_security(),
            test_base_config(&tmp),
            Arc::clone(&service),
            runner,
        );

        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();

        assert!(result.success);
        let handles = service.registered_handles();
        assert_eq!(
            handles.len(),
            1,
            "single-child delegate should use one orchestration handle"
        );
        let snapshot = service
            .inspect(&handles[0])
            .unwrap()
            .expect("snapshot for shared handle");
        assert_eq!(snapshot.children.len(), 1);
    }

    #[tokio::test]
    async fn oneshot_mode_does_not_route_through_session_coordinator_executor() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "researcher".to_string(),
            DelegateAgentConfig {
                provider: "totally-invalid-provider".to_string(),
                model: "model".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::OneShot,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let executor = Arc::new(StubSessionCoordinatorExecutor::new(
            CoordinatorOutcome::Completed {
                coordinator_id: "coord-oneshot".to_string(),
                children: vec![],
            },
        ));
        let tool = DelegateTool::with_session_executor(
            agents,
            None,
            test_security(),
            test_base_config(&tmp),
            executor.clone(),
        );

        let result = tool
            .execute(json!({"agent": "researcher", "prompt": "summarize this"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Failed to create provider"));
        assert!(executor.recorded_requests().await.is_empty());
    }

    /// Session mode must be blocked in read-only security policy (same as OneShot).
    #[tokio::test]
    async fn session_mode_blocked_in_readonly_policy() {
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let readonly = Arc::new(SecurityPolicy {
            autonomy: crate::security::AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, readonly, test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(
            result
                .error
                .as_deref()
                .unwrap_or("")
                .contains("read-only mode"),
            "expected read-only error, got: {:?}",
            result.error
        );
    }

    /// Session mode rejects deferred transport fields at schema level and fails in read-only mode.
    /// Note: This validates schema rejection, not runtime coordinator call inspection.
    #[tokio::test]
    async fn session_mode_schema_rejects_deferred_transport_fields() {
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        let readonly = Arc::new(SecurityPolicy {
            autonomy: crate::security::AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tmp = TempDir::new().unwrap();
        let tool = DelegateTool::new(agents, None, readonly, test_base_config(&tmp));

        let result = tool
            .execute(json!({
                "agent": "code_agent",
                "prompt": "write tests",
                "transport": "cross_process",
                "mailbox": "disk",
                "remote_bridge": true,
                "worktree": { "isolated": true },
                "permission_escalation": { "delegate": true }
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("read-only mode"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["additionalProperties"], json!(false));
        let properties = schema["properties"].as_object().unwrap();
        assert!(!properties.contains_key("transport"));
        assert!(!properties.contains_key("mailbox"));
        assert!(!properties.contains_key("remote_bridge"));
        assert!(!properties.contains_key("worktree"));
        assert!(!properties.contains_key("permission_escalation"));
    }

    /// Session mode must respect the depth limit (same as OneShot).
    #[tokio::test]
    async fn session_mode_respects_depth_limit() {
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4-20250514".to_string(),
                system_prompt: None,
                api_key: Some("test-key".to_string()),
                temperature: None,
                max_depth: 2,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: None,
                timeout_ms: None,
            },
        );
        // depth=2 equals max_depth=2, so it is at the limit and must be blocked
        let tmp = TempDir::new().unwrap();
        let tool =
            DelegateTool::with_depth(agents, None, test_security(), 2, test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(
            result
                .error
                .as_deref()
                .unwrap_or("")
                .contains("depth limit"),
            "expected depth limit error, got: {:?}",
            result.error
        );
    }

    /// Session mode hitting the iteration budget must return a structured non-success result.
    #[tokio::test]
    async fn session_mode_iteration_budget_returns_structured_result() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "ollama".to_string(),
                model: "llama3".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: Some(0),
                timeout_ms: Some(60_000),
            },
        );
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();

        assert!(!result.success);
        let structured = result.structured.expect("structured result");
        assert_eq!(structured["status"], "budget_exceeded");
        let blockers = structured["blockers"].as_array().unwrap();
        assert!(blockers
            .iter()
            .any(|b| { b.as_str().unwrap_or("").contains("maximum tool iterations") }));
    }

    /// Session mode timeout must return a structured non-success result.
    #[tokio::test]
    async fn session_mode_timeout_returns_structured_result() {
        let tmp = TempDir::new().unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "code_agent".to_string(),
            DelegateAgentConfig {
                provider: "ollama".to_string(),
                model: "llama3".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::Session,
                max_iterations: Some(1),
                timeout_ms: Some(0),
            },
        );
        let tool = DelegateTool::new(agents, None, test_security(), test_base_config(&tmp));
        let result = tool
            .execute(json!({"agent": "code_agent", "prompt": "write tests"}))
            .await
            .unwrap();

        assert!(!result.success);
        let structured = result.structured.expect("structured result");
        assert_eq!(structured["status"], "budget_exceeded");
        let blockers = structured["blockers"].as_array().unwrap();
        assert!(blockers.iter().any(|b| {
            b.as_str().unwrap_or("").contains("timeout")
                || b.as_str().unwrap_or("").contains("timeout exceeded")
        }));
    }

    /// A `DelegateAgentConfig` with `execution_mode: Session` must not be confused
    /// with `OneShot` — verify the config field round-trips correctly.
    #[test]
    fn session_mode_config_field_is_session() {
        let cfg = DelegateAgentConfig {
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            max_depth: 3,
            execution_mode: DelegateExecutionMode::Session,
            max_iterations: Some(10),
            timeout_ms: Some(30_000),
        };
        assert_eq!(cfg.execution_mode, DelegateExecutionMode::Session);
        assert_ne!(cfg.execution_mode, DelegateExecutionMode::OneShot);
        assert_eq!(cfg.max_iterations, Some(10));
        assert_eq!(cfg.timeout_ms, Some(30_000));
    }
}
