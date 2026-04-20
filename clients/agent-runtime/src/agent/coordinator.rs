//! In-process coordinator foundations for Track 4 Slice 1.
//!
//! This module is intentionally scoped to supervised in-process orchestration only.
//! Mailbox persistence, remote bridge transport, worktree isolation, and permission
//! escalation flows remain deferred to later Track 4 slices.

use crate::agent::code_session::{CodeSessionResult, CodeSessionStatus};
use crate::agent::{Agent, AgentExecutionError};
use crate::config::{Config, DelegateAgentConfig};
use crate::tools::ToolResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorTransport {
    InProcess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanInPolicy {
    AllMustSucceed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChildAgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildState {
    Registered,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ChildState {
    fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancellationReason {
    ParentRequested,
    SiblingFailed { child_id: ChildAgentId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub last_sequence: u64,
    pub terminal_reason: Option<ChildTerminationReason>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeMeta {
    pub coordinator_id: String,
    pub child_id: Option<ChildAgentId>,
    pub sequence: u64,
    pub correlation_id: String,
    pub sent_at: DateTime<Utc>,
    pub transport: CoordinatorTransport,
}

#[derive(Debug, Clone)]
pub struct MessageEnvelope<T> {
    pub meta: EnvelopeMeta,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinatorLaunchRequest {
    pub parent_session_id: Option<String>,
    pub children: Vec<ChildLaunchRequest>,
    pub fan_in: FanInPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildLaunchRequest {
    pub child_id: ChildAgentId,
    pub agent_name: String,
    pub prompt: String,
    pub context: Option<String>,
    pub launch_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildTerminalStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ChildExecutionResult {
    pub session_id: String,
    pub tool_result: ToolResult,
    pub status: ChildTerminalStatus,
}

#[derive(Debug, Clone)]
pub struct ChildExecutionError {
    pub session_id: Option<String>,
    pub error: String,
    pub tool_result: Option<ToolResult>,
}

#[derive(Debug, Clone)]
pub enum CoordinatorMessage {
    DispatchChild(ChildLaunchRequest),
    CancelChild { reason: CancellationReason },
    ChildStarted { session_id: Option<String> },
    ChildProgress { summary: String },
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
                coordinator_id: dispatch.meta.coordinator_id,
                child_id: Some(request.child_id),
                sequence: dispatch.meta.sequence,
                correlation_id: dispatch.meta.correlation_id,
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
                state: ChildState::Registered,
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

    pub fn next_envelope(
        &self,
        child_id: Option<ChildAgentId>,
        correlation_id: impl Into<String>,
        payload: CoordinatorMessage,
    ) -> MessageEnvelope<CoordinatorMessage> {
        MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: self.coordinator_id.clone(),
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
                record.state = ChildState::Registered;
            }
            CoordinatorMessage::CancelChild { reason } => {
                if !record.state.is_terminal() {
                    record.state = ChildState::Running;
                    record.summary = Some(format!("cancelling: {reason:?}"));
                }
            }
            CoordinatorMessage::ChildStarted { session_id } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::Running;
                record.session_id = session_id.clone();
            }
            CoordinatorMessage::ChildProgress { summary } => {
                if record.state.is_terminal() {
                    return Err(CoordinatorError::AlreadyTerminalState);
                }
                record.state = ChildState::Running;
                record.summary = Some(summary.clone());
            }
            CoordinatorMessage::ChildCompleted { result } => {
                self.record_terminal(
                    record,
                    child_id,
                    TerminalUpdate {
                        outcome: CoordinatorChildOutcome::Succeeded {
                            child_id: record.child_id.clone(),
                            launch_index: record.launch_index,
                            result: result.clone(),
                        },
                        session_id: result.session_id.clone(),
                        state: ChildState::Succeeded,
                        terminal_reason: ChildTerminationReason::Completed,
                        summary: result.tool_result.output.clone(),
                    },
                )?;
            }
            CoordinatorMessage::ChildFailed { error } => {
                self.record_terminal(
                    record,
                    child_id,
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
                    child_id,
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

        Ok(())
    }

    fn record_terminal(
        &self,
        record: &mut ChildRecord,
        child_id: ChildAgentId,
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
            .insert(child_id, update.outcome);
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
        if envelope.meta.transport != CoordinatorTransport::InProcess {
            return Err(CoordinatorError::InvalidEnvelope(
                "unsupported transport".to_string(),
            ));
        }
        if envelope.meta.child_id.is_none() {
            return Err(CoordinatorError::InvalidEnvelope(
                "missing child id".to_string(),
            ));
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
                            coordinator_id: dispatch.meta.coordinator_id,
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            correlation_id: response_correlation.clone(),
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
                            coordinator_id: dispatch.meta.coordinator_id,
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            correlation_id: response_correlation.clone(),
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
                            coordinator_id: dispatch.meta.coordinator_id,
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            correlation_id: response_correlation.clone(),
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
                            coordinator_id: dispatch.meta.coordinator_id,
                            child_id: Some(request.child_id.clone()),
                            sequence: dispatch.meta.sequence,
                            correlation_id: response_correlation.clone(),
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
        }
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

    /// Compile-time assertion: CoordinatorTransport must ONLY contain InProcess for this slice.
    /// RemoteBridge, CrossProcess, MailboxPersistence, WorktreeIsolation are deferred to Track 4.
    #[test]
    fn coordinator_slice_defers_non_in_process_transport_and_deferred_scope() {
        // Exhaustive match ensures CI fails if new transport variants are added.
        // This is a compile-time guard - if CoordinatorTransport gets a new variant,
        // the match below will fail to compile, alerting developers that Track 4
        // deferral assumptions need updating.
        fn assert_only_in_process(t: CoordinatorTransport) -> Option<()> {
            match t {
                CoordinatorTransport::InProcess => Some(()),
            }
        }
        assert!(
            assert_only_in_process(CoordinatorTransport::InProcess).is_some(),
            "CoordinatorTransport must only have InProcess for this slice"
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
                    correlation_id: String::new(),
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
        assert_eq!(final_record.state, ChildState::Succeeded);
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
}
