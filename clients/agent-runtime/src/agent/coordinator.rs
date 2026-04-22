//! In-process coordinator foundations for Track 4 Slice 1.
//!
//! This module is intentionally scoped to supervised in-process orchestration only.
//! Mailbox persistence, remote bridge transport, worktree isolation, and permission
//! escalation flows remain deferred to later Track 4 slices.

use crate::agent::code_session::{CodeSessionResult, CodeSessionStatus};
use crate::agent::mailbox::LogicalEndpoint;
use crate::agent::{Agent, AgentExecutionError};
use crate::config::{Config, DelegateAgentConfig};
use crate::tools::ToolResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type SupervisionRegistry = BTreeMap<ChildAgentId, ChildRecord>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorState {
    Initialized,
    Dispatching,
    Supervising,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl CoordinatorState {
    pub fn allows_transition_to(&self, target: &Self) -> bool {
        use CoordinatorState::{
            Cancelled, Cancelling, Completed, Dispatching, Failed, Initialized, Supervising,
        };

        matches!(
            (self, target),
            (Initialized, Dispatching)
                | (Dispatching, Supervising | Cancelling | Failed)
                | (Supervising, Completed | Cancelling | Failed)
                | (Cancelling, Cancelled | Failed)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorTransport {
    InProcess,
    Mailbox,
    RemoteBridge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanInPolicy {
    AllMustSucceed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChildAgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildState {
    Queued,
    Starting,
    Running,
    WaitingOnParent,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl ChildState {
    fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChildExecutionSpec {
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub sandbox_mode: Option<String>,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    #[serde(default)]
    pub tool_denylist: Vec<String>,
    #[serde(default)]
    pub provider_override: Option<String>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub transport: Option<CoordinatorTransport>,
    #[serde(default)]
    pub read_only_project_access: bool,
    #[serde(default)]
    pub permission_broker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedExecutionRequest {
    pub transport: CoordinatorTransport,
    pub sandbox_mode: Option<String>,
    pub repository_id: Option<String>,
    pub worktree_id: Option<String>,
    pub read_only_project_access: bool,
    pub tool_allowlist: Vec<String>,
    pub tool_denylist: Vec<String>,
    pub provider_override: Option<String>,
    pub model_override: Option<String>,
    pub working_directory: Option<String>,
    pub permission_broker: Option<String>,
}

impl From<&ChildExecutionSpec> for NormalizedExecutionRequest {
    fn from(value: &ChildExecutionSpec) -> Self {
        Self {
            transport: value
                .transport
                .clone()
                .unwrap_or(CoordinatorTransport::InProcess),
            sandbox_mode: value.sandbox_mode.clone(),
            repository_id: value.repository_id.clone(),
            worktree_id: value.worktree_id.clone(),
            read_only_project_access: value.read_only_project_access,
            tool_allowlist: value.tool_allowlist.clone(),
            tool_denylist: value.tool_denylist.clone(),
            provider_override: value.provider_override.clone(),
            model_override: value.model_override.clone(),
            working_directory: value.working_directory.clone(),
            permission_broker: value.permission_broker.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalBrokerMode {
    None,
    ParentOwnedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcedExecutionGuarantees {
    pub transport: CoordinatorTransport,
    pub process_local_handle_authority: bool,
    pub mailbox_backed_delivery: bool,
    pub repository_isolation_enforced: bool,
    pub worktree_isolation_enforced: bool,
    pub sandbox_clone_enforced: bool,
    pub remote_bridge_connected: bool,
    pub approval_broker_mode: ApprovalBrokerMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildExecutionMetadataView {
    pub requested: NormalizedExecutionRequest,
    pub enforced: EnforcedExecutionGuarantees,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchContractRejection {
    UnsupportedTransport { requested: CoordinatorTransport },
    UnsupportedIsolation { field: String, requested: String },
    UnsupportedPermissionBroker { reason: String },
}

impl std::fmt::Display for LaunchContractRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTransport { requested } => {
                write!(f, "unsupported transport request: {requested:?}")
            }
            Self::UnsupportedIsolation { field, requested } => {
                write!(f, "unsupported isolation request for {field}: {requested}")
            }
            Self::UnsupportedPermissionBroker { reason } => {
                write!(f, "unsupported permission broker request: {reason}")
            }
        }
    }
}

fn enforced_execution_guarantees(
    requested: &NormalizedExecutionRequest,
) -> EnforcedExecutionGuarantees {
    EnforcedExecutionGuarantees {
        transport: match requested.transport {
            CoordinatorTransport::Mailbox => CoordinatorTransport::Mailbox,
            CoordinatorTransport::InProcess | CoordinatorTransport::RemoteBridge => {
                CoordinatorTransport::InProcess
            }
        },
        process_local_handle_authority: true,
        mailbox_backed_delivery: requested.transport == CoordinatorTransport::Mailbox,
        repository_isolation_enforced: false,
        worktree_isolation_enforced: false,
        sandbox_clone_enforced: false,
        remote_bridge_connected: false,
        approval_broker_mode: ApprovalBrokerMode::ParentOwnedOnly,
    }
}

fn normalize_execution_metadata(
    spec: Option<&ChildExecutionSpec>,
) -> Result<Option<ChildExecutionMetadataView>, LaunchContractRejection> {
    let Some(spec) = spec else {
        return Ok(None);
    };

    let requested = NormalizedExecutionRequest::from(spec);

    if requested.transport == CoordinatorTransport::RemoteBridge {
        return Err(LaunchContractRejection::UnsupportedTransport {
            requested: CoordinatorTransport::RemoteBridge,
        });
    }

    if let Some(repository_id) = &requested.repository_id {
        return Err(LaunchContractRejection::UnsupportedIsolation {
            field: "repository_id".to_string(),
            requested: repository_id.clone(),
        });
    }

    if let Some(worktree_id) = &requested.worktree_id {
        return Err(LaunchContractRejection::UnsupportedIsolation {
            field: "worktree_id".to_string(),
            requested: worktree_id.clone(),
        });
    }

    if let Some(sandbox_mode) = &requested.sandbox_mode {
        let normalized = sandbox_mode.trim().to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "clone" | "cloned" | "isolated" | "workspace_clone"
        ) {
            return Err(LaunchContractRejection::UnsupportedIsolation {
                field: "sandbox_mode".to_string(),
                requested: sandbox_mode.clone(),
            });
        }
    }

    if let Some(permission_broker) = &requested.permission_broker {
        let normalized = permission_broker.trim().to_ascii_lowercase();
        if normalized != "parent_owned_only" {
            return Err(LaunchContractRejection::UnsupportedPermissionBroker {
                reason: permission_broker.clone(),
            });
        }
    }

    Ok(Some(ChildExecutionMetadataView {
        enforced: enforced_execution_guarantees(&requested),
        requested,
    }))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub tool_name: String,
    pub reason: String,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved {
        request_id: String,
        decided_at: DateTime<Utc>,
    },
    Denied {
        request_id: String,
        decided_at: DateTime<Utc>,
        reason: String,
    },
    Cancelled {
        request_id: String,
        decided_at: DateTime<Utc>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalStatus {
    None,
    Pending { request: ApprovalRequest },
    Resolved { decision: ApprovalDecision },
}

impl Default for ApprovalStatus {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoordinatorEvent {
    ChildStateChanged {
        child_id: String,
        state: ChildState,
        summary: Option<String>,
        sequence: u64,
    },
    ApprovalRequested {
        child_id: String,
        request: ApprovalRequest,
        sequence: u64,
    },
    ApprovalResolved {
        child_id: String,
        decision: ApprovalDecision,
        sequence: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancellationReason {
    ParentRequested,
    SiblingFailed { child_id: ChildAgentId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildTerminationReason {
    Completed,
    Failed(String),
    Cancelled(CancellationReason),
}

#[derive(Debug, Clone)]
pub struct ChildRecord {
    pub child_id: ChildAgentId,
    pub agent_name: String,
    pub launch_index: u32,
    pub session_id: Option<String>,
    pub state: ChildState,
    pub execution: Option<ChildExecutionMetadataView>,
    pub approval: ApprovalStatus,
    pub last_sequence: u64,
    pub terminal_reason: Option<ChildTerminationReason>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeMeta {
    pub coordinator_id: String,
    pub child_id: Option<ChildAgentId>,
    pub sequence: u64,
    pub message_id: String,
    pub correlation_id: String,
    pub sender: LogicalEndpoint,
    pub recipient: LogicalEndpoint,
    pub sent_at: DateTime<Utc>,
    pub transport: CoordinatorTransport,
}

#[derive(Debug, Clone)]
pub struct MessageEnvelope<T> {
    pub meta: EnvelopeMeta,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinatorLaunchRequest {
    pub parent_session_id: Option<String>,
    pub children: Vec<ChildLaunchRequest>,
    pub fan_in: FanInPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildLaunchRequest {
    pub child_id: ChildAgentId,
    pub agent_name: String,
    pub prompt: String,
    pub context: Option<String>,
    pub launch_index: u32,
    pub execution: Option<ChildExecutionSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildTerminalStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildExecutionResult {
    pub session_id: String,
    pub tool_result: ToolResult,
    pub status: ChildTerminalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildExecutionError {
    pub session_id: Option<String>,
    pub error: String,
    pub tool_result: Option<ToolResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorMessage {
    DispatchChild(ChildLaunchRequest),
    CancelChild { reason: CancellationReason },
    ChildStarted { session_id: Option<String> },
    ChildProgress { summary: String },
    RequestApproval { request: ApprovalRequest },
    ResolveApproval { decision: ApprovalDecision },
    ChildCompleted { result: ChildExecutionResult },
    ChildFailed { error: ChildExecutionError },
    ChildCancelled { reason: CancellationReason },
}

#[derive(Debug, Clone)]
pub enum CoordinatorChildOutcome {
    Succeeded {
        child_id: ChildAgentId,
        launch_index: u32,
        result: ChildExecutionResult,
    },
    Failed {
        child_id: ChildAgentId,
        launch_index: u32,
        error: ChildExecutionError,
    },
    Cancelled {
        child_id: ChildAgentId,
        launch_index: u32,
        reason: CancellationReason,
    },
}

#[derive(Debug, Clone)]
pub enum CoordinatorOutcome {
    Completed {
        coordinator_id: String,
        children: Vec<CoordinatorChildOutcome>,
    },
    Failed {
        coordinator_id: String,
        error: String,
        children: Vec<CoordinatorChildOutcome>,
    },
    Cancelled {
        coordinator_id: String,
        reason: CancellationReason,
        children: Vec<CoordinatorChildOutcome>,
    },
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CoordinatorError {
    #[error("invalid state transition")]
    InvalidStateTransition,
    #[error("terminal state is immutable")]
    AlreadyTerminalState,
    #[error("duplicate child id: {0}")]
    DuplicateChild(String),
    #[error("duplicate launch index: {0}")]
    DuplicateLaunchIndex(u32),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
    #[error("coordinator failed closed: {0}")]
    FailedClosed(String),
    #[error("launch contract rejected: {0}")]
    LaunchContractRejected(LaunchContractRejection),
}

#[async_trait]
pub trait CoordinatorChildRunner: Send + Sync {
    async fn run_child(
        &self,
        request: ChildLaunchRequest,
        dispatch: MessageEnvelope<CoordinatorMessage>,
        cancellation: CancellationToken,
    ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError>;
}

pub struct DelegatedAgentRunner {
    base_config: Arc<Config>,
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    fallback_credential: Option<String>,
}

impl DelegatedAgentRunner {
    pub fn new(
        base_config: Arc<Config>,
        agents: Arc<HashMap<String, DelegateAgentConfig>>,
        fallback_credential: Option<String>,
    ) -> Self {
        Self {
            base_config,
            agents,
            fallback_credential,
        }
    }

    fn delegate_config(
        &self,
        request: &ChildLaunchRequest,
    ) -> Result<&DelegateAgentConfig, CoordinatorError> {
        self.agents.get(&request.agent_name).ok_or_else(|| {
            CoordinatorError::FailedClosed(format!(
                "missing delegate config for {}",
                request.agent_name
            ))
        })
    }

    fn build_effective_config(
        &self,
        request: &ChildLaunchRequest,
        agent_config: &DelegateAgentConfig,
    ) -> (Config, Duration, String) {
        let mut config = (*self.base_config).clone();
        config.default_provider = Some(agent_config.provider.clone());
        config.default_model = Some(agent_config.model.clone());
        config.agent.profile = "code".to_string();
        config.agent.code_session.enabled = true;
        if let Some(iterations) = agent_config.max_iterations {
            config.agent.max_tool_iterations = iterations;
            config.agent.code_session.max_iterations = iterations;
        }
        if let Some(key) = &agent_config.api_key {
            config.api_key = Some(key.clone());
        } else if let Some(key) = &self.fallback_credential {
            config.api_key = Some(key.clone());
        }
        if let Some(execution) = &request.execution {
            if let Some(provider_override) = &execution.provider_override {
                config.default_provider = Some(provider_override.clone());
            }
            if let Some(model_override) = &execution.model_override {
                config.default_model = Some(model_override.clone());
            }
            if let Some(working_directory) = &execution.working_directory {
                config.workspace_dir = std::path::PathBuf::from(working_directory);
            }
        }

        let timeout_ms = agent_config
            .timeout_ms
            .or(Some(config.agent.code_session.timeout_ms))
            .unwrap_or(120_000)
            .max(1);

        let prompt = if let Some(context) = request
            .context
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            format!("[Context]\n{context}\n\n[Task]\n{}", request.prompt)
        } else {
            request.prompt.clone()
        };
        (config, Duration::from_millis(timeout_ms), prompt)
    }

    fn session_tool_result(
        agent_name: &str,
        agent_config: &DelegateAgentConfig,
        result: &CodeSessionResult,
    ) -> ToolResult {
        let rendered = result.render();
        ToolResult {
            success: result.is_success(),
            output: format!(
                "[Agent '{agent_name}' session ({provider}/{model})]\n{rendered}",
                provider = agent_config.provider,
                model = agent_config.model,
            ),
            error: (!result.is_success()).then(|| result.summary.clone()),
            structured: Some(result.to_structured()),
        }
    }

    fn session_error_result(
        session_id: &str,
        summary: String,
        status: CodeSessionStatus,
    ) -> CodeSessionResult {
        let mut result = CodeSessionResult::from_error(session_id, status, summary.clone());
        result.blockers.push(summary);
        result
    }
}

#[async_trait]
impl CoordinatorChildRunner for DelegatedAgentRunner {
    async fn run_child(
        &self,
        request: ChildLaunchRequest,
        dispatch: MessageEnvelope<CoordinatorMessage>,
        cancellation: CancellationToken,
    ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
        let agent_config = self.delegate_config(&request)?.clone();
        let session_id = Uuid::new_v4().to_string();
        let (config, timeout, prompt) = self.build_effective_config(&request, &agent_config);

        let child_future = async {
            let mut agent =
                Agent::code_from_config_with_delegated(&config, true).map_err(|error| {
                    CoordinatorMessage::ChildFailed {
                        error: ChildExecutionError {
                            session_id: Some(session_id.clone()),
                            error: format!(
                                "Failed to create delegated session for '{}': {error}",
                                request.agent_name
                            ),
                            tool_result: Some(Self::session_tool_result(
                                &request.agent_name,
                                &agent_config,
                                &Self::session_error_result(
                                    &session_id,
                                    format!(
                                        "Failed to create delegated session for '{}': {error}",
                                        request.agent_name
                                    ),
                                    CodeSessionStatus::Error,
                                ),
                            )),
                        },
                    }
                })?;

            let result = tokio::time::timeout(timeout, agent.turn(&prompt)).await;

            let payload = match result {
                Ok(Ok(output)) => {
                    let parsed = CodeSessionResult::parse_from_output(&output, &session_id);
                    if parsed.is_success() {
                        CoordinatorMessage::ChildCompleted {
                            result: ChildExecutionResult {
                                session_id: session_id.clone(),
                                tool_result: Self::session_tool_result(
                                    &request.agent_name,
                                    &agent_config,
                                    &parsed,
                                ),
                                status: ChildTerminalStatus::Succeeded,
                            },
                        }
                    } else {
                        CoordinatorMessage::ChildFailed {
                            error: ChildExecutionError {
                                session_id: Some(session_id.clone()),
                                error: parsed.summary.clone(),
                                tool_result: Some(Self::session_tool_result(
                                    &request.agent_name,
                                    &agent_config,
                                    &parsed,
                                )),
                            },
                        }
                    }
                }
                Ok(Err(error)) => {
                    #[allow(clippy::match_same_arms, unreachable_patterns)]
                    let status = match error.downcast_ref::<AgentExecutionError>() {
                        Some(
                            AgentExecutionError::IterationBudgetExceeded { .. }
                            | AgentExecutionError::CostBudgetExceeded { .. },
                        ) => CodeSessionStatus::BudgetExceeded,
                        // Catch-all for any new AgentExecutionError variants: map to Error
                        Some(_) => CodeSessionStatus::Error,
                        // None means it's not an AgentExecutionError at all
                        None => CodeSessionStatus::Error,
                    };
                    let parsed = Self::session_error_result(
                        &session_id,
                        format!("Agent '{}' session failed: {error}", request.agent_name),
                        status,
                    );
                    CoordinatorMessage::ChildFailed {
                        error: ChildExecutionError {
                            session_id: Some(session_id.clone()),
                            error: parsed.summary.clone(),
                            tool_result: Some(Self::session_tool_result(
                                &request.agent_name,
                                &agent_config,
                                &parsed,
                            )),
                        },
                    }
                }
                Err(_) => {
                    let mut parsed = Self::session_error_result(
                        &session_id,
                        format!(
                            "Agent '{}' session timed out after {}ms",
                            request.agent_name,
                            timeout.as_millis()
                        ),
                        CodeSessionStatus::BudgetExceeded,
                    );
                    parsed
                        .blockers
                        .push("timeout exceeded before completion".to_string());
                    CoordinatorMessage::ChildFailed {
                        error: ChildExecutionError {
                            session_id: Some(session_id.clone()),
                            error: parsed.summary.clone(),
                            tool_result: Some(Self::session_tool_result(
                                &request.agent_name,
                                &agent_config,
                                &parsed,
                            )),
                        },
                    }
                }
            };

            Ok::<CoordinatorMessage, CoordinatorMessage>(payload)
        };

        let payload = tokio::select! {
            () = cancellation.cancelled() => CoordinatorMessage::ChildCancelled { reason: CancellationReason::ParentRequested },
            payload = child_future => match payload {
                Ok(message) | Err(message) => message,
            },
        };

        Ok(MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: dispatch.meta.coordinator_id.clone(),
                child_id: Some(request.child_id.clone()),
                sequence: dispatch.meta.sequence,
                message_id: Uuid::new_v4().to_string(),
                correlation_id: dispatch.meta.correlation_id.clone(),
                sender: LogicalEndpoint::child(
                    dispatch.meta.coordinator_id.clone(),
                    request.child_id.clone(),
                ),
                recipient: LogicalEndpoint::coordinator_child(
                    dispatch.meta.coordinator_id.clone(),
                    request.child_id.clone(),
                ),
                sent_at: Utc::now(),
                transport: CoordinatorTransport::InProcess,
            },
            payload,
        })
    }
}

pub struct Coordinator {
    coordinator_id: String,
    state: Arc<Mutex<CoordinatorState>>,
    registry: Arc<Mutex<SupervisionRegistry>>,
    outcomes: Arc<Mutex<BTreeMap<ChildAgentId, CoordinatorChildOutcome>>>,
    applied_messages: Arc<Mutex<HashMap<(ChildAgentId, String), String>>>,
    events: Arc<Mutex<Vec<CoordinatorEvent>>>,
    next_sequence: AtomicU64,
}

struct TerminalUpdate {
    outcome: CoordinatorChildOutcome,
    session_id: String,
    state: ChildState,
    terminal_reason: ChildTerminationReason,
    summary: String,
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            coordinator_id: Uuid::new_v4().to_string(),
            state: Arc::new(Mutex::new(CoordinatorState::Initialized)),
            registry: Arc::new(Mutex::new(BTreeMap::new())),
            outcomes: Arc::new(Mutex::new(BTreeMap::new())),
            applied_messages: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            next_sequence: AtomicU64::new(1),
        }
    }

    pub fn coordinator_id(&self) -> &str {
        &self.coordinator_id
    }

    pub fn current_state(&self) -> Result<CoordinatorState, CoordinatorError> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| CoordinatorError::FailedClosed("state lock poisoned".to_string()))
    }

    pub fn transition(
        &self,
        target: CoordinatorState,
    ) -> Result<CoordinatorState, CoordinatorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("state lock poisoned".to_string()))?;

        if state.is_terminal() {
            return if *state == target {
                Ok(state.clone())
            } else {
                Err(CoordinatorError::AlreadyTerminalState)
            };
        }

        if !state.allows_transition_to(&target) {
            return Err(CoordinatorError::InvalidStateTransition);
        }

        *state = target;
        Ok(state.clone())
    }

    fn log_event(&self, event: CoordinatorEvent) -> Result<(), CoordinatorError> {
        self.events
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("event log lock poisoned".to_string()))?
            .push(event);
        Ok(())
    }

    pub fn event_log(&self) -> Result<Vec<CoordinatorEvent>, CoordinatorError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| CoordinatorError::FailedClosed("event log lock poisoned".to_string()))
    }

    pub fn admit_child(&self, request: &ChildLaunchRequest) -> Result<(), CoordinatorError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("registry lock poisoned".to_string()))?;

        if registry.contains_key(&request.child_id) {
            return Err(CoordinatorError::DuplicateChild(request.child_id.0.clone()));
        }

        if registry
            .values()
            .any(|record| record.launch_index == request.launch_index)
        {
            return Err(CoordinatorError::DuplicateLaunchIndex(request.launch_index));
        }

        registry.insert(
            request.child_id.clone(),
            ChildRecord {
                child_id: request.child_id.clone(),
                agent_name: request.agent_name.clone(),
                launch_index: request.launch_index,
                session_id: None,
                state: ChildState::Queued,
                execution: normalize_execution_metadata(request.execution.as_ref())
                    .map_err(CoordinatorError::LaunchContractRejected)?,
                approval: ApprovalStatus::None,
                last_sequence: 0,
                terminal_reason: None,
                summary: None,
            },
        );
        Ok(())
    }

    pub fn ordered_child_ids(&self) -> Result<Vec<ChildAgentId>, CoordinatorError> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("registry lock poisoned".to_string()))?;
        let mut records: Vec<_> = registry.values().cloned().collect();
        records.sort_by_key(|record| record.launch_index);
        Ok(records.into_iter().map(|record| record.child_id).collect())
    }

    pub fn child_record(
        &self,
        child_id: &ChildAgentId,
    ) -> Result<Option<ChildRecord>, CoordinatorError> {
        self.registry
            .lock()
            .map(|registry| registry.get(child_id).cloned())
            .map_err(|_| CoordinatorError::FailedClosed("registry lock poisoned".to_string()))
    }

    fn apply_cancellation_visibility(
        &self,
        reason: &CancellationReason,
    ) -> Result<(), CoordinatorError> {
        for child_id in self.ordered_child_ids()? {
            let Some(record) = self.child_record(&child_id)? else {
                continue;
            };
            if record.state.is_terminal() || record.state == ChildState::Cancelling {
                continue;
            }

            let cancel_envelope = self.next_envelope(
                Some(child_id.clone()),
                format!("cancel:{}", record.launch_index),
                CoordinatorMessage::CancelChild {
                    reason: reason.clone(),
                },
            );
            self.apply_envelope(&cancel_envelope)?;
        }
        Ok(())
    }

    pub fn next_envelope(
        &self,
        child_id: Option<ChildAgentId>,
        correlation_id: impl Into<String>,
        payload: CoordinatorMessage,
    ) -> MessageEnvelope<CoordinatorMessage> {
        MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: self.coordinator_id.clone(),
                message_id: Uuid::new_v4().to_string(),
                sender: self.sender_for(child_id.as_ref(), &payload),
                recipient: self.recipient_for(child_id.as_ref(), &payload),
                child_id,
                sequence: self.next_sequence.fetch_add(1, Ordering::SeqCst),
                correlation_id: correlation_id.into(),
                sent_at: Utc::now(),
                transport: CoordinatorTransport::InProcess,
            },
            payload,
        }
    }

    fn resequence_envelope(
        &self,
        envelope: MessageEnvelope<CoordinatorMessage>,
    ) -> MessageEnvelope<CoordinatorMessage> {
        MessageEnvelope {
            meta: EnvelopeMeta {
                sequence: self.next_sequence.fetch_add(1, Ordering::SeqCst),
                sent_at: Utc::now(),
                ..envelope.meta
            },
            payload: envelope.payload,
        }
    }

    pub fn apply_envelope(
        &self,
        envelope: &MessageEnvelope<CoordinatorMessage>,
    ) -> Result<(), CoordinatorError> {
        self.validate_envelope(envelope)?;
        let child_id = envelope
            .meta
            .child_id
            .clone()
            .ok_or_else(|| CoordinatorError::InvalidEnvelope("missing child id".to_string()))?;

        let payload_digest = serde_json::to_string(&envelope.payload).map_err(|error| {
            CoordinatorError::FailedClosed(format!("failed to serialize envelope payload: {error}"))
        })?;
        let duplicate_key = (child_id.clone(), envelope.meta.message_id.clone());

        let mut applied_messages = self.applied_messages.lock().map_err(|_| {
            CoordinatorError::FailedClosed("applied message lock poisoned".to_string())
        })?;
        if let Some(existing_digest) = applied_messages.get(&duplicate_key) {
            if existing_digest == &payload_digest {
                return Ok(());
            }
            return Err(CoordinatorError::InvalidEnvelope(format!(
                "conflicting duplicate message {} for child {}",
                envelope.meta.message_id, child_id.0
            )));
        }

        let mut registry = self
            .registry
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("registry lock poisoned".to_string()))?;
        let record = registry.get_mut(&child_id).ok_or_else(|| {
            CoordinatorError::InvalidEnvelope(format!("unknown child {}", child_id.0))
        })?;

        if envelope.meta.sequence <= record.last_sequence {
            return Err(CoordinatorError::InvalidEnvelope(format!(
                "sequence {} is not monotonic for child {}",
                envelope.meta.sequence, child_id.0
            )));
        }
        record.last_sequence = envelope.meta.sequence;

        match &envelope.payload {
            CoordinatorMessage::DispatchChild(_) => {
                record.state = ChildState::Starting;
                record.summary = Some("dispatching child".to_string());
            }
            CoordinatorMessage::CancelChild { reason } => {
                if !record.state.is_terminal() {
                    record.state = ChildState::Cancelling;
                    record.summary = Some(format!("cancelling: {reason:?}"));
                }
            }
            CoordinatorMessage::ChildStarted { session_id } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::Running;
                record.approval = ApprovalStatus::None;
                record.session_id = session_id.clone();
            }
            CoordinatorMessage::ChildProgress { summary } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::Running;
                record.summary = Some(summary.clone());
            }
            CoordinatorMessage::RequestApproval { request } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::WaitingOnParent;
                record.summary = Some(format!(
                    "awaiting parent approval for {}",
                    request.tool_name
                ));
                record.approval = ApprovalStatus::Pending {
                    request: request.clone(),
                };
                self.log_event(CoordinatorEvent::ApprovalRequested {
                    child_id: child_id.0.clone(),
                    request: request.clone(),
                    sequence: envelope.meta.sequence,
                })?;
            }
            CoordinatorMessage::ResolveApproval { decision } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::Running;
                record.summary = Some("approval decision recorded".to_string());
                record.approval = ApprovalStatus::Resolved {
                    decision: decision.clone(),
                };
                self.log_event(CoordinatorEvent::ApprovalResolved {
                    child_id: child_id.0.clone(),
                    decision: decision.clone(),
                    sequence: envelope.meta.sequence,
                })?;
            }
            CoordinatorMessage::ChildCompleted { result } => {
                self.record_terminal(
                    record,
                    &child_id,
                    TerminalUpdate {
                        outcome: CoordinatorChildOutcome::Succeeded {
                            child_id: record.child_id.clone(),
                            launch_index: record.launch_index,
                            result: result.clone(),
                        },
                        session_id: result.session_id.clone(),
                        state: ChildState::Completed,
                        terminal_reason: ChildTerminationReason::Completed,
                        summary: result.tool_result.output.clone(),
                    },
                )?;
            }
            CoordinatorMessage::ChildFailed { error } => {
                self.record_terminal(
                    record,
                    &child_id,
                    TerminalUpdate {
                        outcome: CoordinatorChildOutcome::Failed {
                            child_id: record.child_id.clone(),
                            launch_index: record.launch_index,
                            error: error.clone(),
                        },
                        session_id: error.session_id.clone().unwrap_or_default(),
                        state: ChildState::Failed,
                        terminal_reason: ChildTerminationReason::Failed(error.error.clone()),
                        summary: error.error.clone(),
                    },
                )?;
            }
            CoordinatorMessage::ChildCancelled { reason } => {
                self.record_terminal(
                    record,
                    &child_id,
                    TerminalUpdate {
                        outcome: CoordinatorChildOutcome::Cancelled {
                            child_id: record.child_id.clone(),
                            launch_index: record.launch_index,
                            reason: reason.clone(),
                        },
                        session_id: record.session_id.clone().unwrap_or_default(),
                        state: ChildState::Cancelled,
                        terminal_reason: ChildTerminationReason::Cancelled(reason.clone()),
                        summary: format!("cancelled: {reason:?}"),
                    },
                )?;
            }
        }

        self.log_event(CoordinatorEvent::ChildStateChanged {
            child_id: child_id.0.clone(),
            state: record.state.clone(),
            summary: record.summary.clone(),
            sequence: envelope.meta.sequence,
        })?;

        applied_messages.insert(duplicate_key, payload_digest);
        Ok(())
    }

    fn sender_for(
        &self,
        child_id: Option<&ChildAgentId>,
        payload: &CoordinatorMessage,
    ) -> LogicalEndpoint {
        match payload {
            CoordinatorMessage::DispatchChild(_)
            | CoordinatorMessage::CancelChild { .. }
            | CoordinatorMessage::ResolveApproval { .. } => {
                LogicalEndpoint::coordinator(self.coordinator_id.clone())
            }
            CoordinatorMessage::ChildStarted { .. }
            | CoordinatorMessage::ChildProgress { .. }
            | CoordinatorMessage::RequestApproval { .. }
            | CoordinatorMessage::ChildCompleted { .. }
            | CoordinatorMessage::ChildFailed { .. }
            | CoordinatorMessage::ChildCancelled { .. } => child_id
                .cloned()
                .map(|value| LogicalEndpoint::child(self.coordinator_id.clone(), value))
                .unwrap_or_else(|| LogicalEndpoint::coordinator(self.coordinator_id.clone())),
        }
    }

    fn recipient_for(
        &self,
        child_id: Option<&ChildAgentId>,
        payload: &CoordinatorMessage,
    ) -> LogicalEndpoint {
        match payload {
            CoordinatorMessage::DispatchChild(_)
            | CoordinatorMessage::CancelChild { .. }
            | CoordinatorMessage::ResolveApproval { .. } => child_id
                .cloned()
                .map(|value| LogicalEndpoint::child(self.coordinator_id.clone(), value))
                .unwrap_or_else(|| LogicalEndpoint::coordinator(self.coordinator_id.clone())),
            CoordinatorMessage::ChildStarted { .. }
            | CoordinatorMessage::ChildProgress { .. }
            | CoordinatorMessage::RequestApproval { .. }
            | CoordinatorMessage::ChildCompleted { .. }
            | CoordinatorMessage::ChildFailed { .. }
            | CoordinatorMessage::ChildCancelled { .. } => child_id
                .cloned()
                .map(|value| LogicalEndpoint::coordinator_child(self.coordinator_id.clone(), value))
                .unwrap_or_else(|| LogicalEndpoint::coordinator(self.coordinator_id.clone())),
        }
    }

    fn record_terminal(
        &self,
        record: &mut ChildRecord,
        child_id: &ChildAgentId,
        update: TerminalUpdate,
    ) -> Result<(), CoordinatorError> {
        if record.state.is_terminal() {
            return Err(CoordinatorError::AlreadyTerminalState);
        }
        record.state = update.state;
        if !update.session_id.is_empty() {
            record.session_id = Some(update.session_id);
        }
        record.terminal_reason = Some(update.terminal_reason);
        record.summary = Some(update.summary);

        self.outcomes
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("outcome lock poisoned".to_string()))?
            .insert(child_id.clone(), update.outcome);
        Ok(())
    }

    fn validate_envelope(
        &self,
        envelope: &MessageEnvelope<CoordinatorMessage>,
    ) -> Result<(), CoordinatorError> {
        if envelope.meta.coordinator_id != self.coordinator_id {
            return Err(CoordinatorError::InvalidEnvelope(
                "coordinator id mismatch".to_string(),
            ));
        }
        if envelope.meta.correlation_id.trim().is_empty() {
            return Err(CoordinatorError::InvalidEnvelope(
                "missing correlation id".to_string(),
            ));
        }
        if !matches!(
            envelope.meta.transport,
            CoordinatorTransport::InProcess
                | CoordinatorTransport::Mailbox
                | CoordinatorTransport::RemoteBridge
        ) {
            return Err(CoordinatorError::InvalidEnvelope(
                "unsupported transport".to_string(),
            ));
        }
        if envelope.meta.child_id.is_none() {
            return Err(CoordinatorError::InvalidEnvelope(
                "missing child id".to_string(),
            ));
        }
        let Some(child_id) = envelope.meta.child_id.as_ref() else {
            return Err(CoordinatorError::InvalidEnvelope(
                "missing child id".to_string(),
            ));
        };

        match &envelope.payload {
            CoordinatorMessage::DispatchChild(_)
            | CoordinatorMessage::CancelChild { .. }
            | CoordinatorMessage::ResolveApproval { .. } => {
                match (&envelope.meta.sender, &envelope.meta.recipient) {
                    (
                        LogicalEndpoint::Coordinator { coordinator_id, .. },
                        LogicalEndpoint::Child {
                            coordinator_id: recipient_id,
                            child_id: recipient_child,
                        },
                    ) if coordinator_id == &self.coordinator_id
                        && recipient_id == &self.coordinator_id
                        && recipient_child == child_id => {}
                    _ => {
                        return Err(CoordinatorError::InvalidEnvelope(
                            "misaddressed coordinator dispatch envelope".to_string(),
                        ))
                    }
                }
            }
            CoordinatorMessage::ChildStarted { .. }
            | CoordinatorMessage::ChildProgress { .. }
            | CoordinatorMessage::RequestApproval { .. }
            | CoordinatorMessage::ChildCompleted { .. }
            | CoordinatorMessage::ChildFailed { .. }
            | CoordinatorMessage::ChildCancelled { .. } => {
                match (&envelope.meta.sender, &envelope.meta.recipient) {
                    (
                        LogicalEndpoint::Child {
                            coordinator_id: sender_id,
                            child_id: sender_child,
                        },
                        LogicalEndpoint::Coordinator {
                            coordinator_id: recipient_id,
                            child_id: recipient_child,
                        },
                    ) if sender_id == &self.coordinator_id
                        && recipient_id == &self.coordinator_id
                        && sender_child == child_id
                        && recipient_child.as_ref() == Some(child_id) => {}
                    _ => {
                        return Err(CoordinatorError::InvalidEnvelope(
                            "misaddressed child response envelope".to_string(),
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    fn ordered_outcomes(&self) -> Result<Vec<CoordinatorChildOutcome>, CoordinatorError> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("registry lock poisoned".to_string()))?;
        let outcomes = self
            .outcomes
            .lock()
            .map_err(|_| CoordinatorError::FailedClosed("outcome lock poisoned".to_string()))?;

        let mut ordered: Vec<_> = registry.values().cloned().collect();
        ordered.sort_by_key(|record| record.launch_index);

        Ok(ordered
            .into_iter()
            .filter_map(|record| outcomes.get(&record.child_id).cloned())
            .collect())
    }

    fn failure_reason(outcome: &CoordinatorChildOutcome) -> Option<CancellationReason> {
        match outcome {
            CoordinatorChildOutcome::Failed { child_id, .. } => {
                Some(CancellationReason::SiblingFailed {
                    child_id: child_id.clone(),
                })
            }
            _ => None,
        }
    }

    pub async fn run(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Result<CoordinatorOutcome, CoordinatorError> {
        self.run_with_cancellation(request, runner, CancellationToken::new())
            .await
    }

    pub async fn run_with_cancellation(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
        parent_cancellation: CancellationToken,
    ) -> Result<CoordinatorOutcome, CoordinatorError> {
        for child in &request.children {
            self.admit_child(child)?;
        }
        self.transition(CoordinatorState::Dispatching)?;

        let coordinator_cancellation = CancellationToken::new();
        let mut join_set: JoinSet<(
            ChildAgentId,
            Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError>,
        )> = JoinSet::new();

        for child in &request.children {
            let dispatch = self.next_envelope(
                Some(child.child_id.clone()),
                format!("dispatch:{}", child.launch_index),
                CoordinatorMessage::DispatchChild(child.clone()),
            );
            self.apply_envelope(&dispatch)?;

            let started = self.next_envelope(
                Some(child.child_id.clone()),
                dispatch.meta.correlation_id.clone(),
                CoordinatorMessage::ChildStarted { session_id: None },
            );
            self.apply_envelope(&started)?;

            let runner = runner.clone();
            let child_request = child.clone();
            let child_id = child.child_id.clone();
            let cancellation = coordinator_cancellation.child_token();
            join_set.spawn(async move {
                let result = runner
                    .run_child(child_request, dispatch, cancellation)
                    .await;
                (child_id, result)
            });
        }

        self.transition(CoordinatorState::Supervising)?;

        let mut failed = false;
        let mut failed_message = None;
        let mut cancel_reason = None;

        while !join_set.is_empty() {
            tokio::select! {
                () = parent_cancellation.cancelled(), if !parent_cancellation.is_cancelled() => {},
                joined = join_set.join_next() => {
                    match joined {
                        Some(Ok((_child_id, Ok(envelope)))) => {
                            let envelope = self.resequence_envelope(envelope);
                            self.apply_envelope(&envelope)?;
                            if matches!(envelope.payload, CoordinatorMessage::ChildFailed { .. }) && !failed {
                                failed = true;
                                let outcomes = self.ordered_outcomes()?;
                                if let Some(reason) = outcomes.iter().find_map(Self::failure_reason) {
                                    cancel_reason = Some(reason.clone());
                                    failed_message = Some(match &reason {
                                        CancellationReason::SiblingFailed { child_id } => format!("child {} failed", child_id.0),
                                        CancellationReason::ParentRequested => "parent cancelled".to_string(),
                                    });
                                }
                                if !coordinator_cancellation.is_cancelled() {
                                    let _ = self.transition(CoordinatorState::Cancelling);
                                    if let Some(reason) = &cancel_reason {
                                        self.apply_cancellation_visibility(reason)?;
                                    }
                                    coordinator_cancellation.cancel();
                                }
                            }
                        }
                        Some(Ok((_child_id, Err(error)))) => {
                            failed = true;
                            failed_message = Some(error.to_string());
                            if !coordinator_cancellation.is_cancelled() {
                                let _ = self.transition(CoordinatorState::Cancelling);
                                coordinator_cancellation.cancel();
                            }
                        }
                        Some(Err(error)) => {
                            failed = true;
                            failed_message = Some(format!("join error: {error}"));
                            if !coordinator_cancellation.is_cancelled() {
                                let _ = self.transition(CoordinatorState::Cancelling);
                                coordinator_cancellation.cancel();
                            }
                        }
                        None => break,
                    }
                }
            }

            if parent_cancellation.is_cancelled() && !coordinator_cancellation.is_cancelled() {
                cancel_reason = Some(CancellationReason::ParentRequested);
                let _ = self.transition(CoordinatorState::Cancelling);
                self.apply_cancellation_visibility(&CancellationReason::ParentRequested)?;
                coordinator_cancellation.cancel();
            }
        }

        let outcomes = self.ordered_outcomes()?;

        if failed {
            self.transition(CoordinatorState::Failed)?;
            return Ok(CoordinatorOutcome::Failed {
                coordinator_id: self.coordinator_id.clone(),
                error: failed_message.unwrap_or_else(|| "coordinator failed".to_string()),
                children: outcomes,
            });
        }

        if let Some(reason) = cancel_reason {
            self.transition(CoordinatorState::Cancelled)?;
            return Ok(CoordinatorOutcome::Cancelled {
                coordinator_id: self.coordinator_id.clone(),
                reason,
                children: outcomes,
            });
        }

        self.transition(CoordinatorState::Completed)?;
        Ok(CoordinatorOutcome::Completed {
            coordinator_id: self.coordinator_id.clone(),
            children: outcomes,
        })
    }
}

// ── Track 4 Slice 2: Supervised Child Lifecycle ──────────────────────────────
//
// Public read-model types and the `SupervisedOrchestrationService` that wraps
// the `Coordinator` behind a lifecycle-aware registry.  All deferred
// capabilities (peer messaging, remote transport, worktree isolation, permission
// escalation) are explicitly out of scope for this slice.

use std::sync::RwLock;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

// ── Task 1.2: Read-model state enums ─────────────────────────────────────────

/// Public mirror of [`CoordinatorState`] that does not leak internal details.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorStateView {
    Initialized,
    Dispatching,
    Supervising,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl From<CoordinatorState> for CoordinatorStateView {
    fn from(s: CoordinatorState) -> Self {
        match s {
            CoordinatorState::Initialized => Self::Initialized,
            CoordinatorState::Dispatching => Self::Dispatching,
            CoordinatorState::Supervising => Self::Supervising,
            CoordinatorState::Cancelling => Self::Cancelling,
            CoordinatorState::Completed => Self::Completed,
            CoordinatorState::Failed => Self::Failed,
            CoordinatorState::Cancelled => Self::Cancelled,
        }
    }
}

/// Public mirror of [`ChildState`] that does not leak internal details.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildStateView {
    Queued,
    Starting,
    Running,
    WaitingOnParent,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl From<ChildState> for ChildStateView {
    fn from(s: ChildState) -> Self {
        match s {
            ChildState::Queued => Self::Queued,
            ChildState::Starting => Self::Starting,
            ChildState::Running => Self::Running,
            ChildState::WaitingOnParent => Self::WaitingOnParent,
            ChildState::Cancelling => Self::Cancelling,
            ChildState::Completed => Self::Completed,
            ChildState::Failed => Self::Failed,
            ChildState::Cancelled => Self::Cancelled,
        }
    }
}

// ── Task 1.1: Public read-model types ────────────────────────────────────────

/// Opaque handle that uniquely identifies a supervised orchestration run.
///
/// Implements `Hash + Eq` so it can serve as a `HashMap` key.
/// Generated via UUIDv4; treat as an opaque string externally.
///
/// # Non-goals (deferred)
/// Peer messaging, remote transport, worktree isolation, and permission
/// escalation are not addressable via this handle in this slice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OrchestrationHandle(pub String);

impl std::fmt::Display for OrchestrationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl OrchestrationHandle {
    fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Returned immediately by `SupervisedOrchestrationService::launch()`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrchestrationLaunchReceipt {
    pub handle: OrchestrationHandle,
    pub snapshot: OrchestrationSnapshot,
}

/// Point-in-time read-model snapshot of a supervised orchestration run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrchestrationSnapshot {
    pub handle: OrchestrationHandle,
    pub parent_session_id: Option<String>,
    pub state: CoordinatorStateView,
    pub children: Vec<ChildLifecycleView>,
    pub events: Vec<LifecycleEventView>,
    pub outcome: Option<OrchestrationOutcomeView>,
}

/// Read-model view of a single child within an orchestration run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChildLifecycleView {
    pub child_id: String,
    pub agent_name: String,
    pub launch_index: u32,
    pub session_id: Option<String>,
    pub state: ChildStateView,
    pub execution: Option<ChildExecutionMetadataView>,
    pub approval: ApprovalStatus,
    pub summary: Option<String>,
    pub terminal_reason: Option<ChildTerminationView>,
}

impl From<&ChildRecord> for ChildLifecycleView {
    fn from(r: &ChildRecord) -> Self {
        Self {
            child_id: r.child_id.0.clone(),
            agent_name: r.agent_name.clone(),
            launch_index: r.launch_index,
            session_id: r.session_id.clone(),
            state: r.state.clone().into(),
            execution: r.execution.clone(),
            approval: r.approval.clone(),
            summary: r.summary.clone(),
            terminal_reason: r.terminal_reason.as_ref().map(ChildTerminationView::from),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LifecycleEventView {
    pub sequence: u64,
    pub child_id: String,
    pub kind: String,
    pub summary: Option<String>,
}

impl From<&CoordinatorEvent> for LifecycleEventView {
    fn from(event: &CoordinatorEvent) -> Self {
        match event {
            CoordinatorEvent::ChildStateChanged {
                child_id,
                state,
                summary,
                sequence,
            } => Self {
                sequence: *sequence,
                child_id: child_id.clone(),
                kind: format!("child_state_changed:{state:?}").to_ascii_lowercase(),
                summary: summary.clone(),
            },
            CoordinatorEvent::ApprovalRequested {
                child_id,
                request,
                sequence,
            } => Self {
                sequence: *sequence,
                child_id: child_id.clone(),
                kind: "approval_requested".to_string(),
                summary: Some(request.reason.clone()),
            },
            CoordinatorEvent::ApprovalResolved {
                child_id,
                decision,
                sequence,
            } => Self {
                sequence: *sequence,
                child_id: child_id.clone(),
                kind: "approval_resolved".to_string(),
                summary: Some(format!("{decision:?}")),
            },
        }
    }
}

/// Read-model view of why a child's run terminated.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChildTerminationView {
    Completed,
    Failed { message: String },
    Cancelled { reason: CancellationReasonView },
}

impl From<&ChildTerminationReason> for ChildTerminationView {
    fn from(r: &ChildTerminationReason) -> Self {
        match r {
            ChildTerminationReason::Completed => Self::Completed,
            ChildTerminationReason::Failed(msg) => Self::Failed {
                message: msg.clone(),
            },
            ChildTerminationReason::Cancelled(reason) => Self::Cancelled {
                reason: reason.into(),
            },
        }
    }
}

/// Read-model view of a cancellation reason.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CancellationReasonView {
    ParentRequested,
    SiblingFailed { child_id: String },
}

impl From<&CancellationReason> for CancellationReasonView {
    fn from(r: &CancellationReason) -> Self {
        match r {
            CancellationReason::ParentRequested => Self::ParentRequested,
            CancellationReason::SiblingFailed { child_id } => Self::SiblingFailed {
                child_id: child_id.0.clone(),
            },
        }
    }
}

/// Compact read-model view of a single child outcome used inside
/// [`OrchestrationOutcomeView`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChildOutcomeView {
    pub child_id: String,
    pub launch_index: u32,
    pub state: ChildStateView,
}

impl From<&CoordinatorChildOutcome> for ChildOutcomeView {
    fn from(o: &CoordinatorChildOutcome) -> Self {
        match o {
            CoordinatorChildOutcome::Succeeded {
                child_id,
                launch_index,
                ..
            } => Self {
                child_id: child_id.0.clone(),
                launch_index: *launch_index,
                state: ChildStateView::Completed,
            },
            CoordinatorChildOutcome::Failed {
                child_id,
                launch_index,
                ..
            } => Self {
                child_id: child_id.0.clone(),
                launch_index: *launch_index,
                state: ChildStateView::Failed,
            },
            CoordinatorChildOutcome::Cancelled {
                child_id,
                launch_index,
                ..
            } => Self {
                child_id: child_id.0.clone(),
                launch_index: *launch_index,
                state: ChildStateView::Cancelled,
            },
        }
    }
}

/// Terminal outcome of a supervised orchestration run.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrchestrationOutcomeView {
    Completed {
        handle: OrchestrationHandle,
        children: Vec<ChildOutcomeView>,
    },
    Failed {
        handle: OrchestrationHandle,
        error: String,
        children: Vec<ChildOutcomeView>,
    },
    Cancelled {
        handle: OrchestrationHandle,
        reason: CancellationReasonView,
        children: Vec<ChildOutcomeView>,
    },
}

/// Disposition of a cancel request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelDisposition {
    /// The cancellation was accepted and the run has now terminated.
    Accepted,
    /// The run was already in a terminal state; no action was taken.
    AlreadyTerminal,
}

/// Result returned by `SupervisedOrchestrationService::cancel()`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CancelResult {
    pub disposition: CancelDisposition,
    pub snapshot: OrchestrationSnapshot,
}

// ── Task 1.3: Internal registry types ────────────────────────────────────────

/// Live state of a running orchestration tracked by the service.
struct ActiveRun {
    coordinator: Arc<Coordinator>,
    cancel_token: CancellationToken,
    /// Wrapped in `AsyncMutex<Option<...>>` so `cancel()` can `.take()` it
    /// without holding the registry `RwLock` across an `.await`.
    join_handle: AsyncMutex<Option<JoinHandle<Result<CoordinatorOutcome, CoordinatorError>>>>,
    request: CoordinatorLaunchRequest,
}

/// Entry in the service registry.
enum RunEntry {
    Active(ActiveRun),
    Terminal(OrchestrationSnapshot, CoordinatorOutcome),
}

// ── Task 1.3 (cont.) + Phase 2: SupervisedOrchestrationService ───────────────

/// Error type for orchestration service operations.
#[derive(Debug, thiserror::Error)]
pub enum OrchestrationServiceError {
    #[error("registry lock poisoned")]
    LockPoisoned,
    #[error("coordinator error: {0}")]
    CoordinatorError(#[from] CoordinatorError),
}

/// Lifecycle-aware in-process orchestration service.
///
/// Wraps the `Coordinator` behind a handle-based registry, exposing
/// `launch`, `inspect`, `cancel`, and `run_to_completion` service
/// entrypoints.
///
/// # Non-goals (deferred to later slices)
/// - Peer-to-peer child messaging
/// - Remote bridge transport
/// - Worktree / filesystem isolation
/// - Permission escalation flows
pub struct SupervisedOrchestrationService {
    registry: RwLock<HashMap<OrchestrationHandle, RunEntry>>,
}

impl SupervisedOrchestrationService {
    /// Create a new service with an empty registry.
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(HashMap::new()),
        }
    }

    fn validate_launch_request(
        request: &CoordinatorLaunchRequest,
    ) -> Result<(), OrchestrationServiceError> {
        let validator = Coordinator::new();
        for child in &request.children {
            validator
                .admit_child(child)
                .map_err(OrchestrationServiceError::CoordinatorError)?;
        }
        Ok(())
    }

    // ── Task 1.4: snapshot helper ─────────────────────────────────────────

    fn snapshot_from_coordinator(
        handle: &OrchestrationHandle,
        coordinator: &Coordinator,
        request: &CoordinatorLaunchRequest,
        outcome: Option<OrchestrationOutcomeView>,
    ) -> Result<OrchestrationSnapshot, OrchestrationServiceError> {
        let state: CoordinatorStateView = coordinator
            .current_state()
            .map_err(OrchestrationServiceError::CoordinatorError)?
            .into();

        let child_ids = coordinator
            .ordered_child_ids()
            .map_err(OrchestrationServiceError::CoordinatorError)?;

        let mut children = Vec::with_capacity(child_ids.len());
        for id in &child_ids {
            if let Some(record) = coordinator
                .child_record(id)
                .map_err(OrchestrationServiceError::CoordinatorError)?
            {
                children.push(ChildLifecycleView::from(&record));
            }
        }
        let events = coordinator
            .event_log()
            .map_err(OrchestrationServiceError::CoordinatorError)?
            .iter()
            .map(LifecycleEventView::from)
            .collect();

        Ok(OrchestrationSnapshot {
            handle: handle.clone(),
            parent_session_id: request.parent_session_id.clone(),
            state,
            children,
            events,
            outcome,
        })
    }

    // ── Task 2.1: launch ──────────────────────────────────────────────────

    /// Launch a new supervised orchestration run.
    ///
    /// Creates a `Coordinator`, spawns it under a parent cancellation token,
    /// registers the active run, and returns a receipt with an initial
    /// snapshot.
    // `async` is kept for API consistency with cancel/inspect — the spawn inside
    // the body uses await, and callers always .await this function.
    #[allow(clippy::unused_async)]
    pub async fn launch(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Result<OrchestrationLaunchReceipt, OrchestrationServiceError> {
        Self::validate_launch_request(&request)?;
        let handle = OrchestrationHandle::new();
        let snapshot_seed = Arc::new(Coordinator::new());
        for child in &request.children {
            snapshot_seed
                .admit_child(child)
                .map_err(OrchestrationServiceError::CoordinatorError)?;
        }
        let coordinator = Arc::new(Coordinator::new());
        let cancel_token = CancellationToken::new();

        let join_handle = {
            let coordinator = coordinator.clone();
            let token = cancel_token.clone();
            let req = request.clone();
            tokio::spawn(async move { coordinator.run_with_cancellation(req, runner, token).await })
        };

        let snapshot = Self::snapshot_from_coordinator(&handle, &snapshot_seed, &request, None)?;

        let active = ActiveRun {
            coordinator,
            cancel_token,
            join_handle: AsyncMutex::new(Some(join_handle)),
            request,
        };

        self.registry
            .write()
            .map_err(|_| OrchestrationServiceError::LockPoisoned)?
            .insert(handle.clone(), RunEntry::Active(active));

        Ok(OrchestrationLaunchReceipt { handle, snapshot })
    }

    // ── Task 2.2: inspect ─────────────────────────────────────────────────

    /// Return a point-in-time snapshot for the given handle, or `None` if
    /// the handle is unknown.
    pub fn inspect(
        &self,
        handle: &OrchestrationHandle,
    ) -> Result<Option<OrchestrationSnapshot>, OrchestrationServiceError> {
        let registry = self
            .registry
            .read()
            .map_err(|_| OrchestrationServiceError::LockPoisoned)?;

        match registry.get(handle) {
            None => Ok(None),
            Some(RunEntry::Terminal(snapshot, _)) => Ok(Some(snapshot.clone())),
            Some(RunEntry::Active(active)) => {
                let snapshot = Self::snapshot_from_coordinator(
                    handle,
                    &active.coordinator,
                    &active.request,
                    None,
                )?;
                Ok(Some(snapshot))
            }
        }
    }

    // ── Task 2.3: cancel ──────────────────────────────────────────────────

    /// Cancel a supervised run by handle.
    ///
    /// Returns `None` for unknown handles.
    /// Returns `CancelDisposition::AlreadyTerminal` if the run is already done.
    ///
    /// IMPORTANT: does NOT hold the `RwLock` guard across any `.await`.
    pub async fn cancel(
        &self,
        handle: &OrchestrationHandle,
    ) -> Result<Option<CancelResult>, OrchestrationServiceError> {
        // Step 1: read token + check terminal — hold read lock briefly.
        let token_opt = {
            let registry = self
                .registry
                .read()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;
            match registry.get(handle) {
                None => return Ok(None),
                Some(RunEntry::Terminal(snapshot, _)) => {
                    return Ok(Some(CancelResult {
                        disposition: CancelDisposition::AlreadyTerminal,
                        snapshot: snapshot.clone(),
                    }));
                }
                Some(RunEntry::Active(active)) => Some(active.cancel_token.clone()),
            }
        }; // read lock dropped here

        // Step 2: trigger cancellation outside any lock.
        if let Some(token) = token_opt {
            token.cancel();
        }

        // Step 3: take JoinHandle under async mutex (not RwLock).
        let join_handle_opt = {
            let registry = self
                .registry
                .read()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;
            if let Some(RunEntry::Active(active)) = registry.get(handle) {
                // Lock the async mutex to take the handle.
                // `.try_lock()` is fine here since we just cancelled — no other
                // future will be awaiting this handle except us.
                active
                    .join_handle
                    .try_lock()
                    .ok()
                    .and_then(|mut g| g.take())
            } else {
                None
            }
        }; // read lock dropped here

        // Step 4: await the join handle without any lock held.
        let coordinator_outcome = if let Some(jh) = join_handle_opt {
            jh.await
                .map_err(|e| {
                    OrchestrationServiceError::CoordinatorError(CoordinatorError::FailedClosed(
                        format!("join error: {e}"),
                    ))
                })?
                .map_err(OrchestrationServiceError::CoordinatorError)?
        } else {
            // Already taken (concurrent cancel?) — re-inspect.
            let registry = self
                .registry
                .read()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;
            if let Some(RunEntry::Terminal(snapshot, _)) = registry.get(handle) {
                return Ok(Some(CancelResult {
                    disposition: CancelDisposition::AlreadyTerminal,
                    snapshot: snapshot.clone(),
                }));
            }
            return Ok(None);
        };

        // Step 5: build outcome view + snapshot, store Terminal entry.
        let outcome_view = Self::build_outcome_view(handle, &coordinator_outcome);

        // We need to read request from the active entry to build the snapshot.
        // Move active → terminal under write lock.
        let snapshot = {
            let mut registry = self
                .registry
                .write()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;

            if let Some(RunEntry::Active(active)) = registry.get(handle) {
                let snap = Self::snapshot_from_coordinator(
                    handle,
                    &active.coordinator,
                    &active.request,
                    Some(outcome_view.clone()),
                )?;
                registry.insert(
                    handle.clone(),
                    RunEntry::Terminal(snap.clone(), coordinator_outcome),
                );
                snap
            } else if let Some(RunEntry::Terminal(snap, _)) = registry.get(handle) {
                snap.clone()
            } else {
                return Ok(None);
            }
        };

        Ok(Some(CancelResult {
            disposition: CancelDisposition::Accepted,
            snapshot,
        }))
    }

    // ── Task 2.4: run_to_completion ───────────────────────────────────────

    /// Thin convenience wrapper: launch and await the terminal outcome.
    ///
    /// Reused by `DelegateTool` session-mode compatibility path.
    pub async fn run_to_completion(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Result<CoordinatorOutcome, OrchestrationServiceError> {
        let receipt = self.launch(request, runner).await?;
        let handle = receipt.handle;

        // Take join handle and await without holding any lock.
        let join_handle_opt = {
            let registry = self
                .registry
                .read()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;
            if let Some(RunEntry::Active(active)) = registry.get(&handle) {
                active
                    .join_handle
                    .try_lock()
                    .ok()
                    .and_then(|mut g| g.take())
            } else {
                None
            }
        };

        let outcome = if let Some(jh) = join_handle_opt {
            jh.await
                .map_err(|e| {
                    OrchestrationServiceError::CoordinatorError(CoordinatorError::FailedClosed(
                        format!("join error: {e}"),
                    ))
                })?
                .map_err(OrchestrationServiceError::CoordinatorError)?
        } else {
            return Err(OrchestrationServiceError::CoordinatorError(
                CoordinatorError::FailedClosed("join handle already taken".to_string()),
            ));
        };

        let outcome_view = Self::build_outcome_view(&handle, &outcome);

        // Store terminal entry.
        {
            let mut registry = self
                .registry
                .write()
                .map_err(|_| OrchestrationServiceError::LockPoisoned)?;
            if let Some(RunEntry::Active(active)) = registry.get(&handle) {
                let snap = Self::snapshot_from_coordinator(
                    &handle,
                    &active.coordinator,
                    &active.request,
                    Some(outcome_view),
                )?;
                registry.insert(handle.clone(), RunEntry::Terminal(snap, outcome.clone()));
            }
        }

        Ok(outcome)
    }

    // ── Private helpers ───────────────────────────────────────────────────

    fn build_outcome_view(
        handle: &OrchestrationHandle,
        outcome: &CoordinatorOutcome,
    ) -> OrchestrationOutcomeView {
        match outcome {
            CoordinatorOutcome::Completed { children, .. } => OrchestrationOutcomeView::Completed {
                handle: handle.clone(),
                children: children.iter().map(ChildOutcomeView::from).collect(),
            },
            CoordinatorOutcome::Failed {
                error, children, ..
            } => OrchestrationOutcomeView::Failed {
                handle: handle.clone(),
                error: error.clone(),
                children: children.iter().map(ChildOutcomeView::from).collect(),
            },
            CoordinatorOutcome::Cancelled {
                reason, children, ..
            } => OrchestrationOutcomeView::Cancelled {
                handle: handle.clone(),
                reason: reason.into(),
                children: children.iter().map(ChildOutcomeView::from).collect(),
            },
        }
    }
}

#[cfg(test)]
impl SupervisedOrchestrationService {
    pub(crate) fn registered_handles(&self) -> Vec<OrchestrationHandle> {
        self.registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .cloned()
            .collect()
    }
}

impl Default for SupervisedOrchestrationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::Mutex as AsyncMutex;

    type CorrelationRecord = (
        String,
        CoordinatorTransport,
        String,
        CoordinatorTransport,
        String,
    );

    #[derive(Clone)]
    enum StubBehavior {
        Success {
            output: &'static str,
            delay_ms: u64,
        },
        GatedSuccess {
            output: &'static str,
            started: Arc<tokio::sync::Notify>,
            release: Arc<tokio::sync::Notify>,
        },
        Failure {
            error: &'static str,
            delay_ms: u64,
        },
        WaitForCancellation,
    }

    struct StubRunner {
        behaviors: BTreeMap<String, StubBehavior>,
        cancellations: Arc<AsyncMutex<Vec<String>>>,
        correlations: Arc<AsyncMutex<Vec<CorrelationRecord>>>,
    }

    impl StubRunner {
        fn new(behaviors: BTreeMap<String, StubBehavior>) -> Self {
            Self {
                behaviors,
                cancellations: Arc::new(AsyncMutex::new(Vec::new())),
                correlations: Arc::new(AsyncMutex::new(Vec::new())),
            }
        }

        async fn cancellations(&self) -> Vec<String> {
            self.cancellations.lock().await.clone()
        }

        async fn correlations(&self) -> Vec<CorrelationRecord> {
            self.correlations.lock().await.clone()
        }
    }

    #[async_trait]
    impl CoordinatorChildRunner for StubRunner {
        async fn run_child(
            &self,
            request: ChildLaunchRequest,
            dispatch: MessageEnvelope<CoordinatorMessage>,
            cancellation: CancellationToken,
        ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
            let behavior = self
                .behaviors
                .get(&request.child_id.0)
                .cloned()
                .ok_or_else(|| {
                    CoordinatorError::FailedClosed("missing stub behavior".to_string())
                })?;

            match behavior {
                StubBehavior::Success { output, delay_ms } => {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    let response_correlation = dispatch.meta.correlation_id.clone();
                    let response = MessageEnvelope {
                        meta: EnvelopeMeta {
                            coordinator_id: dispatch.meta.coordinator_id.clone(),
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            message_id: format!("{}:success", dispatch.meta.message_id),
                            correlation_id: response_correlation.clone(),
                            sender: LogicalEndpoint::child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            recipient: LogicalEndpoint::coordinator_child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            sent_at: Utc::now(),
                            transport: CoordinatorTransport::InProcess,
                        },
                        payload: CoordinatorMessage::ChildCompleted {
                            result: ChildExecutionResult {
                                session_id: format!("session-{}", request.child_id.0),
                                tool_result: ToolResult {
                                    success: true,
                                    output: output.to_string(),
                                    error: None,
                                    structured: None,
                                },
                                status: ChildTerminalStatus::Succeeded,
                            },
                        },
                    };
                    self.correlations.lock().await.push((
                        request.child_id.0.clone(),
                        dispatch.meta.transport.clone(),
                        dispatch.meta.correlation_id.clone(),
                        response.meta.transport.clone(),
                        response_correlation,
                    ));
                    Ok(response)
                }
                StubBehavior::GatedSuccess {
                    output,
                    started,
                    release,
                } => {
                    started.notify_waiters();
                    release.notified().await;
                    let response_correlation = dispatch.meta.correlation_id.clone();
                    let response = MessageEnvelope {
                        meta: EnvelopeMeta {
                            coordinator_id: dispatch.meta.coordinator_id.clone(),
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            message_id: format!("{}:gated", dispatch.meta.message_id),
                            correlation_id: response_correlation.clone(),
                            sender: LogicalEndpoint::child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            recipient: LogicalEndpoint::coordinator_child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            sent_at: Utc::now(),
                            transport: CoordinatorTransport::InProcess,
                        },
                        payload: CoordinatorMessage::ChildCompleted {
                            result: ChildExecutionResult {
                                session_id: format!("session-{}", request.child_id.0),
                                tool_result: ToolResult {
                                    success: true,
                                    output: output.to_string(),
                                    error: None,
                                    structured: None,
                                },
                                status: ChildTerminalStatus::Succeeded,
                            },
                        },
                    };
                    self.correlations.lock().await.push((
                        request.child_id.0.clone(),
                        dispatch.meta.transport.clone(),
                        dispatch.meta.correlation_id.clone(),
                        response.meta.transport.clone(),
                        response_correlation,
                    ));
                    Ok(response)
                }
                StubBehavior::Failure { error, delay_ms } => {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    let response_correlation = dispatch.meta.correlation_id.clone();
                    let response = MessageEnvelope {
                        meta: EnvelopeMeta {
                            coordinator_id: dispatch.meta.coordinator_id.clone(),
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            message_id: format!("{}:failure", dispatch.meta.message_id),
                            correlation_id: response_correlation.clone(),
                            sender: LogicalEndpoint::child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            recipient: LogicalEndpoint::coordinator_child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            sent_at: Utc::now(),
                            transport: CoordinatorTransport::InProcess,
                        },
                        payload: CoordinatorMessage::ChildFailed {
                            error: ChildExecutionError {
                                session_id: Some(format!("session-{}", request.child_id.0)),
                                error: error.to_string(),
                                tool_result: Some(ToolResult {
                                    success: false,
                                    output: String::new(),
                                    error: Some(error.to_string()),
                                    structured: None,
                                }),
                            },
                        },
                    };
                    self.correlations.lock().await.push((
                        request.child_id.0.clone(),
                        dispatch.meta.transport.clone(),
                        dispatch.meta.correlation_id.clone(),
                        response.meta.transport.clone(),
                        response_correlation,
                    ));
                    Ok(response)
                }
                StubBehavior::WaitForCancellation => {
                    cancellation.cancelled().await;
                    self.cancellations
                        .lock()
                        .await
                        .push(request.child_id.0.clone());
                    let response_correlation = dispatch.meta.correlation_id.clone();
                    let response = MessageEnvelope {
                        meta: EnvelopeMeta {
                            coordinator_id: dispatch.meta.coordinator_id.clone(),
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            message_id: format!("{}:cancelled", dispatch.meta.message_id),
                            correlation_id: response_correlation.clone(),
                            sender: LogicalEndpoint::child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            recipient: LogicalEndpoint::coordinator_child(
                                dispatch.meta.coordinator_id.clone(),
                                request.child_id.clone(),
                            ),
                            sent_at: Utc::now(),
                            transport: CoordinatorTransport::InProcess,
                        },
                        payload: CoordinatorMessage::ChildCancelled {
                            reason: CancellationReason::ParentRequested,
                        },
                    };
                    self.correlations.lock().await.push((
                        request.child_id.0.clone(),
                        dispatch.meta.transport.clone(),
                        dispatch.meta.correlation_id.clone(),
                        response.meta.transport.clone(),
                        response_correlation,
                    ));
                    Ok(response)
                }
            }
        }
    }

    fn child(child_id: &str, launch_index: u32) -> ChildLaunchRequest {
        ChildLaunchRequest {
            child_id: ChildAgentId(child_id.to_string()),
            agent_name: child_id.to_string(),
            prompt: format!("prompt-{child_id}"),
            context: None,
            launch_index,
            execution: None,
        }
    }

    fn child_with_execution(
        child_id: &str,
        launch_index: u32,
        execution: ChildExecutionSpec,
    ) -> ChildLaunchRequest {
        let mut request = child(child_id, launch_index);
        request.execution = Some(execution);
        request
    }

    fn child_outcome_ids(children: &[CoordinatorChildOutcome]) -> Vec<String> {
        children
            .iter()
            .map(|child| match child {
                CoordinatorChildOutcome::Succeeded { child_id, .. }
                | CoordinatorChildOutcome::Failed { child_id, .. }
                | CoordinatorChildOutcome::Cancelled { child_id, .. } => child_id.0.clone(),
            })
            .collect()
    }

    #[tokio::test]
    async fn coordinator_transitions_to_completed_after_successful_fan_in() {
        let coordinator = Coordinator::new();
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            (
                "child-a".to_string(),
                StubBehavior::Success {
                    output: "alpha",
                    delay_ms: 20,
                },
            ),
            (
                "child-b".to_string(),
                StubBehavior::Success {
                    output: "beta",
                    delay_ms: 5,
                },
            ),
        ])));

        let outcome = coordinator
            .run(
                CoordinatorLaunchRequest {
                    parent_session_id: Some("parent-1".to_string()),
                    children: vec![child("child-a", 0), child("child-b", 1)],
                    fan_in: FanInPolicy::AllMustSucceed,
                },
                runner,
            )
            .await
            .expect("coordinator run should succeed");

        match outcome {
            CoordinatorOutcome::Completed { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["child-a", "child-b"]);
            }
            other => panic!("expected completed outcome, got {other:?}"),
        }

        assert_eq!(
            coordinator
                .current_state()
                .expect("state should be readable"),
            CoordinatorState::Completed
        );
    }

    #[test]
    fn terminal_coordinator_state_is_immutable() {
        let coordinator = Coordinator::new();
        coordinator
            .transition(CoordinatorState::Dispatching)
            .unwrap();
        coordinator
            .transition(CoordinatorState::Supervising)
            .unwrap();
        coordinator.transition(CoordinatorState::Completed).unwrap();

        let error = coordinator
            .transition(CoordinatorState::Failed)
            .unwrap_err();
        assert_eq!(error, CoordinatorError::AlreadyTerminalState);
        assert_eq!(
            coordinator.current_state().unwrap(),
            CoordinatorState::Completed
        );
    }

    #[test]
    fn supervising_requires_cancelling_before_cancelled_terminal() {
        assert!(CoordinatorState::Supervising.allows_transition_to(&CoordinatorState::Cancelling));
        assert!(!CoordinatorState::Supervising.allows_transition_to(&CoordinatorState::Cancelled));
    }

    #[test]
    fn duplicate_child_identity_is_rejected() {
        let coordinator = Coordinator::new();
        let request = child("duplicate", 0);
        coordinator.admit_child(&request).unwrap();
        let error = coordinator.admit_child(&request).unwrap_err();
        assert_eq!(
            error,
            CoordinatorError::DuplicateChild("duplicate".to_string())
        );
        assert_eq!(
            coordinator.ordered_child_ids().unwrap(),
            vec![ChildAgentId("duplicate".to_string())]
        );
    }

    #[tokio::test]
    async fn aggregate_results_preserve_launch_order() {
        let coordinator = Coordinator::new();
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            (
                "slow-first".to_string(),
                StubBehavior::Success {
                    output: "slow",
                    delay_ms: 30,
                },
            ),
            (
                "fast-second".to_string(),
                StubBehavior::Success {
                    output: "fast",
                    delay_ms: 5,
                },
            ),
        ])));

        let outcome = coordinator
            .run(
                CoordinatorLaunchRequest {
                    parent_session_id: None,
                    children: vec![child("slow-first", 0), child("fast-second", 1)],
                    fan_in: FanInPolicy::AllMustSucceed,
                },
                runner,
            )
            .await
            .unwrap();

        match outcome {
            CoordinatorOutcome::Completed { children, .. } => {
                assert_eq!(
                    child_outcome_ids(&children),
                    vec!["slow-first", "fast-second"]
                );
            }
            other => panic!("expected completed outcome, got {other:?}"),
        }
    }

    #[test]
    fn mailbox_transport_response_is_accepted_for_owning_run() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();

        let dispatch = coordinator.next_envelope(
            Some(child_id.clone()),
            "corr-mailbox",
            CoordinatorMessage::DispatchChild(child("child-a", 0)),
        );
        coordinator.apply_envelope(&dispatch).unwrap();

        let started = coordinator.next_envelope(
            Some(child_id.clone()),
            "corr-mailbox",
            CoordinatorMessage::ChildStarted { session_id: None },
        );
        coordinator.apply_envelope(&started).unwrap();

        let envelope = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(child_id.clone()),
                sequence: started.meta.sequence + 1,
                message_id: "mailbox-msg-1".to_string(),
                correlation_id: "corr-mailbox".to_string(),
                sender: crate::agent::mailbox::LogicalEndpoint::Child {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: child_id.clone(),
                },
                recipient: crate::agent::mailbox::LogicalEndpoint::Coordinator {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                },
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-a".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };

        coordinator.apply_envelope(&envelope).unwrap();
        assert_eq!(
            coordinator.child_record(&child_id).unwrap().unwrap().state,
            ChildState::Completed
        );
    }

    #[test]
    fn misaddressed_mailbox_envelope_is_rejected() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();

        let error = coordinator
            .apply_envelope(&MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                    sequence: 1,
                    message_id: "mailbox-msg-bad".to_string(),
                    correlation_id: "corr-bad".to_string(),
                    sender: crate::agent::mailbox::LogicalEndpoint::Child {
                        coordinator_id: coordinator.coordinator_id().to_string(),
                        child_id: child_id.clone(),
                    },
                    recipient: crate::agent::mailbox::LogicalEndpoint::Child {
                        coordinator_id: coordinator.coordinator_id().to_string(),
                        child_id: child_id.clone(),
                    },
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::Mailbox,
                },
                payload: CoordinatorMessage::ChildCompleted {
                    result: ChildExecutionResult {
                        session_id: "session-a".to_string(),
                        tool_result: ToolResult {
                            success: true,
                            output: "done".to_string(),
                            error: None,
                            structured: None,
                        },
                        status: ChildTerminalStatus::Succeeded,
                    },
                },
            })
            .unwrap_err();

        assert!(matches!(error, CoordinatorError::InvalidEnvelope(_)));
    }

    #[test]
    fn duplicate_terminal_mailbox_envelope_is_idempotent() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();
        coordinator
            .apply_envelope(&coordinator.next_envelope(
                Some(child_id.clone()),
                "corr-dup",
                CoordinatorMessage::DispatchChild(child("child-a", 0)),
            ))
            .unwrap();
        coordinator
            .apply_envelope(&coordinator.next_envelope(
                Some(child_id.clone()),
                "corr-dup",
                CoordinatorMessage::ChildStarted { session_id: None },
            ))
            .unwrap();

        let terminal = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(child_id.clone()),
                sequence: 3,
                message_id: "msg-dup".to_string(),
                correlation_id: "corr-dup".to_string(),
                sender: crate::agent::mailbox::LogicalEndpoint::Child {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: child_id.clone(),
                },
                recipient: crate::agent::mailbox::LogicalEndpoint::Coordinator {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                },
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-a".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };

        coordinator.apply_envelope(&terminal).unwrap();
        coordinator.apply_envelope(&terminal).unwrap();

        let outcomes = coordinator.ordered_outcomes().unwrap();
        assert_eq!(child_outcome_ids(&outcomes), vec!["child-a"]);
    }

    #[test]
    fn conflicting_mailbox_replay_fails_closed() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();
        coordinator
            .apply_envelope(&coordinator.next_envelope(
                Some(child_id.clone()),
                "corr-conflict",
                CoordinatorMessage::DispatchChild(child("child-a", 0)),
            ))
            .unwrap();
        coordinator
            .apply_envelope(&coordinator.next_envelope(
                Some(child_id.clone()),
                "corr-conflict",
                CoordinatorMessage::ChildStarted { session_id: None },
            ))
            .unwrap();

        let completed = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(child_id.clone()),
                sequence: 3,
                message_id: "msg-conflict".to_string(),
                correlation_id: "corr-conflict".to_string(),
                sender: crate::agent::mailbox::LogicalEndpoint::Child {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: child_id.clone(),
                },
                recipient: crate::agent::mailbox::LogicalEndpoint::Coordinator {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                },
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-a".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };
        coordinator.apply_envelope(&completed).unwrap();

        let conflicting = MessageEnvelope {
            payload: CoordinatorMessage::ChildFailed {
                error: ChildExecutionError {
                    session_id: Some("session-a".to_string()),
                    error: "boom".to_string(),
                    tool_result: None,
                },
            },
            ..completed.clone()
        };

        let error = coordinator.apply_envelope(&conflicting).unwrap_err();
        assert!(matches!(error, CoordinatorError::InvalidEnvelope(_)));
    }

    #[test]
    fn duplicate_mailbox_delivery_does_not_change_aggregate_ordering() {
        let coordinator = Coordinator::new();
        let first_child = ChildAgentId("child-a".to_string());
        let second_child = ChildAgentId("child-b".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();
        coordinator.admit_child(&child("child-b", 1)).unwrap();

        for child_id in [first_child.clone(), second_child.clone()] {
            coordinator
                .apply_envelope(&coordinator.next_envelope(
                    Some(child_id.clone()),
                    format!("dispatch:{}", child_id.0),
                    CoordinatorMessage::DispatchChild(child(
                        &child_id.0,
                        if child_id == first_child { 0 } else { 1 },
                    )),
                ))
                .unwrap();
            coordinator
                .apply_envelope(&coordinator.next_envelope(
                    Some(child_id.clone()),
                    format!("dispatch:{}", child_id.0),
                    CoordinatorMessage::ChildStarted { session_id: None },
                ))
                .unwrap();
        }

        let second_terminal = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(second_child.clone()),
                sequence: 5,
                message_id: "msg-second".to_string(),
                correlation_id: "corr-second".to_string(),
                sender: crate::agent::mailbox::LogicalEndpoint::Child {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: second_child.clone(),
                },
                recipient: crate::agent::mailbox::LogicalEndpoint::Coordinator {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(second_child.clone()),
                },
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-b".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done-b".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };
        let first_terminal = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(first_child.clone()),
                sequence: 6,
                message_id: "msg-first".to_string(),
                correlation_id: "corr-first".to_string(),
                sender: crate::agent::mailbox::LogicalEndpoint::Child {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: first_child.clone(),
                },
                recipient: crate::agent::mailbox::LogicalEndpoint::Coordinator {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(first_child.clone()),
                },
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-a".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done-a".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };

        coordinator.apply_envelope(&second_terminal).unwrap();
        coordinator.apply_envelope(&first_terminal).unwrap();
        coordinator.apply_envelope(&second_terminal).unwrap();

        let outcomes = coordinator.ordered_outcomes().unwrap();
        assert_eq!(child_outcome_ids(&outcomes), vec!["child-a", "child-b"]);
    }

    #[test]
    fn envelope_sequence_and_correlation_are_monotonic() {
        let coordinator = Coordinator::new();
        let first = coordinator.next_envelope(
            Some(ChildAgentId("child-a".to_string())),
            "corr-1",
            CoordinatorMessage::DispatchChild(child("child-a", 0)),
        );
        let second = coordinator.next_envelope(
            Some(ChildAgentId("child-b".to_string())),
            first.meta.correlation_id.clone(),
            CoordinatorMessage::DispatchChild(child("child-b", 1)),
        );

        assert!(second.meta.sequence > first.meta.sequence);
        assert_eq!(second.meta.correlation_id, first.meta.correlation_id);
    }

    /// Compile-time assertion: coordinator transport remains constrained to the
    /// explicit local/bridge variants we model today.
    #[test]
    fn coordinator_transport_limits_surface_to_supported_variants() {
        fn assert_allowed_transport(t: CoordinatorTransport) -> bool {
            matches!(
                t,
                CoordinatorTransport::InProcess
                    | CoordinatorTransport::Mailbox
                    | CoordinatorTransport::RemoteBridge
            )
        }
        assert!(
            assert_allowed_transport(CoordinatorTransport::InProcess),
            "CoordinatorTransport must allow in-process transport"
        );
        assert!(
            assert_allowed_transport(CoordinatorTransport::Mailbox),
            "CoordinatorTransport must allow mailbox transport"
        );
        assert!(
            assert_allowed_transport(CoordinatorTransport::RemoteBridge),
            "CoordinatorTransport must allow remote bridge transport"
        );
    }

    #[test]
    fn invalid_envelope_fails_closed() {
        let coordinator = Coordinator::new();
        coordinator.admit_child(&child("child-a", 0)).unwrap();
        let error = coordinator
            .apply_envelope(&MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: None,
                    sequence: 2,
                    message_id: "invalid-envelope".to_string(),
                    correlation_id: String::new(),
                    sender: LogicalEndpoint::coordinator(coordinator.coordinator_id().to_string()),
                    recipient: LogicalEndpoint::coordinator(
                        coordinator.coordinator_id().to_string(),
                    ),
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::InProcess,
                },
                payload: CoordinatorMessage::ChildCompleted {
                    result: ChildExecutionResult {
                        session_id: "session-a".to_string(),
                        tool_result: ToolResult {
                            success: true,
                            output: "done".to_string(),
                            error: None,
                            structured: None,
                        },
                        status: ChildTerminalStatus::Succeeded,
                    },
                },
            })
            .unwrap_err();

        assert!(matches!(error, CoordinatorError::InvalidEnvelope(_)));
    }

    #[test]
    fn admit_child_normalizes_requested_vs_enforced_execution_metadata() {
        let coordinator = Coordinator::new();
        coordinator
            .admit_child(&child_with_execution(
                "child-a",
                0,
                ChildExecutionSpec {
                    transport: Some(CoordinatorTransport::Mailbox),
                    sandbox_mode: Some("workspace_write".to_string()),
                    tool_allowlist: vec!["read".to_string()],
                    provider_override: Some("anthropic".to_string()),
                    model_override: Some("claude".to_string()),
                    working_directory: Some("/tmp/project".to_string()),
                    read_only_project_access: true,
                    ..ChildExecutionSpec::default()
                },
            ))
            .unwrap();

        let record = coordinator
            .child_record(&ChildAgentId("child-a".to_string()))
            .unwrap()
            .unwrap();
        let metadata = record.execution.expect("normalized execution metadata");

        assert_eq!(metadata.requested.transport, CoordinatorTransport::Mailbox);
        assert_eq!(metadata.enforced.transport, CoordinatorTransport::Mailbox);
        assert!(metadata.enforced.process_local_handle_authority);
        assert!(metadata.enforced.mailbox_backed_delivery);
        assert!(!metadata.enforced.repository_isolation_enforced);
        assert!(!metadata.enforced.worktree_isolation_enforced);
        assert!(!metadata.enforced.sandbox_clone_enforced);
        assert!(!metadata.enforced.remote_bridge_connected);
        assert_eq!(
            metadata.enforced.approval_broker_mode,
            ApprovalBrokerMode::ParentOwnedOnly
        );
        assert_eq!(
            metadata.requested.sandbox_mode.as_deref(),
            Some("workspace_write")
        );
        assert_eq!(
            metadata.requested.working_directory.as_deref(),
            Some("/tmp/project")
        );
        assert!(metadata.requested.read_only_project_access);
    }

    #[test]
    fn admit_child_rejects_remote_bridge_requests_fail_closed() {
        let coordinator = Coordinator::new();

        let error = coordinator
            .admit_child(&child_with_execution(
                "child-a",
                0,
                ChildExecutionSpec {
                    transport: Some(CoordinatorTransport::RemoteBridge),
                    ..ChildExecutionSpec::default()
                },
            ))
            .unwrap_err();

        assert!(matches!(
            error,
            CoordinatorError::LaunchContractRejected(
                LaunchContractRejection::UnsupportedTransport {
                    requested: CoordinatorTransport::RemoteBridge
                }
            )
        ));
    }

    #[test]
    fn admit_child_rejects_unsupported_isolation_requests_fail_closed() {
        let coordinator = Coordinator::new();

        let repository_error = coordinator
            .admit_child(&child_with_execution(
                "child-a",
                0,
                ChildExecutionSpec {
                    repository_id: Some("repo-1".to_string()),
                    ..ChildExecutionSpec::default()
                },
            ))
            .unwrap_err();
        assert!(matches!(
            repository_error,
            CoordinatorError::LaunchContractRejected(
                LaunchContractRejection::UnsupportedIsolation { field, requested }
            ) if field == "repository_id" && requested == "repo-1"
        ));

        let sandbox_error = coordinator
            .admit_child(&child_with_execution(
                "child-b",
                1,
                ChildExecutionSpec {
                    sandbox_mode: Some("clone".to_string()),
                    ..ChildExecutionSpec::default()
                },
            ))
            .unwrap_err();
        assert!(matches!(
            sandbox_error,
            CoordinatorError::LaunchContractRejected(
                LaunchContractRejection::UnsupportedIsolation { field, requested }
            ) if field == "sandbox_mode" && requested == "clone"
        ));
    }

    #[test]
    fn admit_child_rejects_unsupported_permission_broker_requests_fail_closed() {
        let coordinator = Coordinator::new();

        let error = coordinator
            .admit_child(&child_with_execution(
                "child-a",
                0,
                ChildExecutionSpec {
                    permission_broker: Some("child_owned".to_string()),
                    ..ChildExecutionSpec::default()
                },
            ))
            .unwrap_err();

        assert!(matches!(
            error,
            CoordinatorError::LaunchContractRejected(
                LaunchContractRejection::UnsupportedPermissionBroker { reason }
            ) if reason == "child_owned"
        ));
    }

    #[test]
    fn cancel_envelope_moves_child_into_cancelling_until_terminal_resolution() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();
        coordinator
            .apply_envelope(&MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                    sequence: 1,
                    message_id: "dispatch-msg".to_string(),
                    correlation_id: "corr-a".to_string(),
                    sender: LogicalEndpoint::coordinator(coordinator.coordinator_id().to_string()),
                    recipient: LogicalEndpoint::child(
                        coordinator.coordinator_id().to_string(),
                        child_id.clone(),
                    ),
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::InProcess,
                },
                payload: CoordinatorMessage::DispatchChild(child("child-a", 0)),
            })
            .unwrap();

        coordinator
            .apply_envelope(&MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id: coordinator.coordinator_id().to_string(),
                    child_id: Some(child_id.clone()),
                    sequence: 2,
                    message_id: "cancel-msg".to_string(),
                    correlation_id: "corr-a".to_string(),
                    sender: LogicalEndpoint::coordinator(coordinator.coordinator_id().to_string()),
                    recipient: LogicalEndpoint::child(
                        coordinator.coordinator_id().to_string(),
                        child_id.clone(),
                    ),
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::InProcess,
                },
                payload: CoordinatorMessage::CancelChild {
                    reason: CancellationReason::ParentRequested,
                },
            })
            .unwrap();

        let record = coordinator.child_record(&child_id).unwrap().unwrap();
        assert_eq!(record.state, ChildState::Cancelling);

        let latest_event = coordinator.event_log().unwrap().pop().unwrap();
        assert!(matches!(
            latest_event,
            CoordinatorEvent::ChildStateChanged {
                state: ChildState::Cancelling,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_redelivery_does_not_duplicate_visible_events() {
        let coordinator = Coordinator::new();
        let child_id = ChildAgentId("child-a".to_string());
        coordinator.admit_child(&child("child-a", 0)).unwrap();

        let completed = MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator.coordinator_id().to_string(),
                child_id: Some(child_id.clone()),
                sequence: 5,
                message_id: "terminal-msg".to_string(),
                correlation_id: "corr-a".to_string(),
                sender: LogicalEndpoint::child(
                    coordinator.coordinator_id().to_string(),
                    child_id.clone(),
                ),
                recipient: LogicalEndpoint::coordinator_child(
                    coordinator.coordinator_id().to_string(),
                    child_id.clone(),
                ),
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::ChildCompleted {
                result: ChildExecutionResult {
                    session_id: "session-a".to_string(),
                    tool_result: ToolResult {
                        success: true,
                        output: "done".to_string(),
                        error: None,
                        structured: None,
                    },
                    status: ChildTerminalStatus::Succeeded,
                },
            },
        };

        coordinator.apply_envelope(&completed).unwrap();
        let event_count_after_first = coordinator.event_log().unwrap().len();
        coordinator.apply_envelope(&completed).unwrap();
        let events = coordinator.event_log().unwrap();

        assert_eq!(events.len(), event_count_after_first);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    CoordinatorEvent::ChildStateChanged {
                        state: ChildState::Completed,
                        child_id,
                        ..
                    } if child_id == "child-a"
                ))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn fatal_child_failure_cancels_siblings() {
        let coordinator = Coordinator::new();
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            (
                "failing".to_string(),
                StubBehavior::Failure {
                    error: "boom",
                    delay_ms: 5,
                },
            ),
            ("waiting".to_string(), StubBehavior::WaitForCancellation),
        ])));

        let outcome = coordinator
            .run(
                CoordinatorLaunchRequest {
                    parent_session_id: Some("parent-2".to_string()),
                    children: vec![child("failing", 0), child("waiting", 1)],
                    fan_in: FanInPolicy::AllMustSucceed,
                },
                runner.clone(),
            )
            .await
            .unwrap();

        match outcome {
            CoordinatorOutcome::Failed { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["failing", "waiting"]);
            }
            other => panic!("expected failed outcome, got {other:?}"),
        }

        assert_eq!(runner.cancellations().await, vec!["waiting"]);
    }

    #[tokio::test]
    async fn parent_cancellation_propagates_to_active_children() {
        let coordinator = Arc::new(Coordinator::new());
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            ("child-a".to_string(), StubBehavior::WaitForCancellation),
            ("child-b".to_string(), StubBehavior::WaitForCancellation),
        ])));
        let cancellation = CancellationToken::new();
        let trigger = cancellation.clone();

        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            let runner = runner.clone();
            async move {
                coordinator
                    .run_with_cancellation(
                        CoordinatorLaunchRequest {
                            parent_session_id: Some("parent-3".to_string()),
                            children: vec![child("child-a", 0), child("child-b", 1)],
                            fan_in: FanInPolicy::AllMustSucceed,
                        },
                        runner,
                        cancellation,
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        trigger.cancel();

        let outcome = task.await.unwrap().unwrap();
        match outcome {
            CoordinatorOutcome::Cancelled { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["child-a", "child-b"]);
            }
            other => panic!("expected cancelled outcome, got {other:?}"),
        }

        let mut cancellations = runner.cancellations().await;
        cancellations.sort();
        assert_eq!(cancellations, vec!["child-a", "child-b"]);
    }

    #[tokio::test]
    async fn parent_can_inspect_child_lifecycle_progression_during_live_run() {
        let coordinator = Arc::new(Coordinator::new());
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let runner = Arc::new(StubRunner::new(BTreeMap::from([(
            "child-a".to_string(),
            StubBehavior::GatedSuccess {
                output: "alpha",
                started: started.clone(),
                release: release.clone(),
            },
        )])));

        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            let runner = runner.clone();
            async move {
                coordinator
                    .run(
                        CoordinatorLaunchRequest {
                            parent_session_id: Some("parent-inspect".to_string()),
                            children: vec![child("child-a", 0)],
                            fan_in: FanInPolicy::AllMustSucceed,
                        },
                        runner,
                    )
                    .await
            }
        });

        started.notified().await;

        let child_record = coordinator
            .child_record(&ChildAgentId("child-a".to_string()))
            .expect("registry should be readable")
            .expect("child should be registered");
        assert_eq!(child_record.state, ChildState::Running);
        assert_eq!(child_record.launch_index, 0);
        assert_eq!(
            coordinator.current_state().unwrap(),
            CoordinatorState::Supervising
        );
        assert!(
            !task.is_finished(),
            "coordinator should still be supervising active work"
        );

        release.notify_waiters();

        let outcome = task.await.unwrap().unwrap();
        match outcome {
            CoordinatorOutcome::Completed { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["child-a"]);
            }
            other => panic!("expected completed outcome, got {other:?}"),
        }

        let final_record = coordinator
            .child_record(&ChildAgentId("child-a".to_string()))
            .expect("registry should be readable")
            .expect("child should remain inspectable");
        assert_eq!(final_record.state, ChildState::Completed);
        assert_eq!(final_record.session_id.as_deref(), Some("session-child-a"));
    }

    #[tokio::test]
    async fn live_run_preserves_in_process_transport_and_end_to_end_correlation() {
        let coordinator = Coordinator::new();
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            (
                "child-a".to_string(),
                StubBehavior::Success {
                    output: "alpha",
                    delay_ms: 5,
                },
            ),
            (
                "child-b".to_string(),
                StubBehavior::Success {
                    output: "beta",
                    delay_ms: 10,
                },
            ),
        ])));

        let outcome = coordinator
            .run(
                CoordinatorLaunchRequest {
                    parent_session_id: Some("parent-correlation".to_string()),
                    children: vec![child("child-a", 0), child("child-b", 1)],
                    fan_in: FanInPolicy::AllMustSucceed,
                },
                runner.clone(),
            )
            .await
            .expect("coordinator run should succeed");

        match outcome {
            CoordinatorOutcome::Completed { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["child-a", "child-b"]);
            }
            other => panic!("expected completed outcome, got {other:?}"),
        }

        let correlations = runner.correlations().await;
        assert_eq!(correlations.len(), 2);
        for (
            child_id,
            dispatch_transport,
            dispatch_correlation,
            response_transport,
            response_correlation,
        ) in correlations
        {
            assert_eq!(
                dispatch_transport,
                CoordinatorTransport::InProcess,
                "dispatch for {child_id} must stay in-process"
            );
            assert_eq!(
                response_transport,
                CoordinatorTransport::InProcess,
                "response for {child_id} must stay in-process"
            );
            assert_eq!(
                response_correlation, dispatch_correlation,
                "response for {child_id} must correlate to the original dispatch envelope"
            );
            assert!(
                dispatch_correlation.starts_with("dispatch:"),
                "unexpected correlation id for {child_id}: {dispatch_correlation}"
            );
        }
    }

    #[tokio::test]
    async fn fan_in_does_not_report_success_before_all_required_children_finish() {
        let coordinator = Arc::new(Coordinator::new());
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let runner = Arc::new(StubRunner::new(BTreeMap::from([
            (
                "child-a".to_string(),
                StubBehavior::Success {
                    output: "alpha",
                    delay_ms: 5,
                },
            ),
            (
                "child-b".to_string(),
                StubBehavior::GatedSuccess {
                    output: "beta",
                    started: started.clone(),
                    release: release.clone(),
                },
            ),
        ])));

        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            let runner = runner.clone();
            async move {
                coordinator
                    .run(
                        CoordinatorLaunchRequest {
                            parent_session_id: Some("parent-fanin".to_string()),
                            children: vec![child("child-a", 0), child("child-b", 1)],
                            fan_in: FanInPolicy::AllMustSucceed,
                        },
                        runner,
                    )
                    .await
            }
        });

        started.notified().await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(
            coordinator.current_state().unwrap(),
            CoordinatorState::Supervising
        );
        assert!(
            !task.is_finished(),
            "coordinator must not report success before gated child finishes"
        );

        release.notify_waiters();

        let outcome = task.await.unwrap().unwrap();
        match outcome {
            CoordinatorOutcome::Completed { children, .. } => {
                assert_eq!(child_outcome_ids(&children), vec!["child-a", "child-b"]);
            }
            other => panic!("expected completed outcome, got {other:?}"),
        }
        assert_eq!(
            coordinator.current_state().unwrap(),
            CoordinatorState::Completed
        );
    }

    // ── Slice 2: SupervisedOrchestrationService tests ─────────────────────

    fn make_request(children: Vec<(&'static str, &'static str)>) -> CoordinatorLaunchRequest {
        CoordinatorLaunchRequest {
            parent_session_id: None,
            children: children
                .into_iter()
                .enumerate()
                .map(|(i, (id, name))| ChildLaunchRequest {
                    child_id: ChildAgentId(id.to_string()),
                    agent_name: name.to_string(),
                    prompt: format!("task for {name}"),
                    context: None,
                    launch_index: u32::try_from(i).unwrap_or(u32::MAX),
                    execution: None,
                })
                .collect(),
            fan_in: FanInPolicy::AllMustSucceed,
        }
    }

    fn stub_runner_with(behaviors: &[(&'static str, StubBehavior)]) -> Arc<StubRunner> {
        let map = behaviors
            .iter()
            .cloned()
            .map(|(id, b)| (id.to_string(), b))
            .collect::<BTreeMap<_, _>>();
        Arc::new(StubRunner::new(map))
    }

    #[tokio::test]
    async fn launch_returns_receipt_with_handle() {
        let svc = SupervisedOrchestrationService::new();
        let runner = stub_runner_with(&[(
            "child-a",
            StubBehavior::Success {
                output: "ok",
                delay_ms: 5,
            },
        )]);
        let request = make_request(vec![("child-a", "AgentA")]);

        let receipt = svc.launch(request, runner).await.unwrap();

        // Handle must be a non-empty UUID-like string.
        assert!(!receipt.handle.0.is_empty());
        // Snapshot handle must match receipt handle.
        assert_eq!(receipt.snapshot.handle, receipt.handle);
        // Immediately after launch the coordinator should be in a live state.
        assert!(
            receipt.snapshot.state != CoordinatorStateView::Completed
                || receipt.snapshot.outcome.is_some(),
            "snapshot state should reflect live or terminal, not an uninitialized default"
        );
    }

    #[tokio::test]
    async fn inspect_active_run_returns_snapshot() {
        let svc = SupervisedOrchestrationService::new();
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());

        let runner = stub_runner_with(&[(
            "child-a",
            StubBehavior::GatedSuccess {
                output: "done",
                started: started.clone(),
                release: release.clone(),
            },
        )]);
        let request = make_request(vec![("child-a", "AgentA")]);

        let receipt = svc.launch(request, runner).await.unwrap();
        let handle = receipt.handle.clone();

        // Wait until child has started so the coordinator is live.
        started.notified().await;

        let snapshot = svc.inspect(&handle).unwrap();
        let snapshot = snapshot.expect("expected Some(snapshot) for an active run");
        assert_eq!(snapshot.handle, handle);

        // Clean up — release the gated child.
        release.notify_waiters();
    }

    #[tokio::test]
    async fn inspect_unknown_handle_returns_error() {
        let svc = SupervisedOrchestrationService::new();
        let bogus = OrchestrationHandle("does-not-exist".to_string());

        let result = svc.inspect(&bogus);

        assert!(
            matches!(result, Ok(None)),
            "expected Ok(None) for unknown handle, got {result:?}"
        );
    }

    #[tokio::test]
    async fn cancel_active_run_resolves_terminal() {
        let svc = SupervisedOrchestrationService::new();
        let runner = stub_runner_with(&[("child-a", StubBehavior::WaitForCancellation)]);
        let request = make_request(vec![("child-a", "AgentA")]);

        let receipt = svc.launch(request, runner).await.unwrap();
        let handle = receipt.handle.clone();

        // Give the child time to reach WaitForCancellation state.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let result = svc.cancel(&handle).await.unwrap();
        assert!(
            result.is_some(),
            "expected a CancelResult for an active run"
        );

        let cancel_result = result.unwrap();
        assert_eq!(cancel_result.disposition, CancelDisposition::Accepted);
    }

    #[tokio::test]
    async fn cancel_already_terminal_returns_already_terminal_disposition() {
        let svc = SupervisedOrchestrationService::new();
        let runner = stub_runner_with(&[(
            "child-a",
            StubBehavior::Success {
                output: "done",
                delay_ms: 0,
            },
        )]);
        let request = make_request(vec![("child-a", "AgentA")]);

        // run_to_completion drives the run to terminal before we try to cancel.
        let runner_arc: Arc<dyn CoordinatorChildRunner> = runner;
        let _outcome = svc
            .run_to_completion(request, runner_arc.clone())
            .await
            .unwrap();

        // Pick the single registered handle.
        let handle = {
            let registry = svc.registry.read().unwrap();
            registry.keys().next().cloned().unwrap()
        };

        let result = svc.cancel(&handle).await.unwrap();
        assert!(result.is_some());
        let cancel_result = result.unwrap();
        assert_eq!(
            cancel_result.disposition,
            CancelDisposition::AlreadyTerminal
        );
    }

    #[tokio::test]
    async fn run_to_completion_returns_outcome() {
        let svc = SupervisedOrchestrationService::new();
        let runner = stub_runner_with(&[(
            "child-a",
            StubBehavior::Success {
                output: "finished",
                delay_ms: 0,
            },
        )]);
        let request = make_request(vec![("child-a", "AgentA")]);

        let runner_arc: Arc<dyn CoordinatorChildRunner> = runner;
        let outcome = svc.run_to_completion(request, runner_arc).await.unwrap();

        match outcome {
            CoordinatorOutcome::Completed { .. } => {}
            other => panic!("expected Completed, got {other:?}"),
        }
    }
}
