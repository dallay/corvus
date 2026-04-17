use crate::agent::code_session::{CodeSessionResult, CodeSessionStatus};
use crate::agent::dispatcher::{
    evaluate_tool_risk_with_policy_for_origin, DispatchAction, NativeToolDispatcher,
    ParsedToolCall, ToolDispatcher, ToolExecutionResult, XmlToolDispatcher,
};
use crate::agent::memory_loader::{CerebroMemoryLoader, DefaultMemoryLoader, MemoryLoader};
use crate::agent::mission::{
    MissionCheckpoint, MissionCoordinator, MissionOutcome, MissionPlan, MissionResumeMetadata,
    MissionState, MissionTerminationReason,
};
use crate::agent::prompt::{
    PromptContext, SystemPromptBuilder, COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS,
};
use crate::bootstrap;
use crate::config::Config;
use crate::cost::{
    BudgetCheck, BudgetEvaluation, CostService, CostTracker, MissionBudgetScope, TokenUsage,
    UsagePeriod,
};
use crate::memory::{Memory, MemoryCategory};
use crate::observability::{redact_observer_payload, Observer, ObserverEvent};
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider};
use crate::security::{
    AuditEvent, AuditEventType, AuditLogger, CodeSessionAuditLog, CommandExecutionLog,
    ExecutionOrigin, SecurityPolicy,
};
use crate::tools::{Tool, ToolSpec};
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use chrono::Utc;
use futures_util::future::join_all;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PRE_FLIGHT_ESTIMATED_OUTPUT_TOKENS: u64 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnOutcome {
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnEvent {
    Prepared,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnContext {
    pub session_id: Option<String>,
    pub execution_mode: crate::config::ExecutionMode,
}

impl TurnContext {
    pub fn with_session(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            execution_mode: crate::config::ExecutionMode::Standard,
        }
    }
}

#[derive(Debug)]
struct StepOutcome {
    final_text: Option<String>,
    approval_required: Option<serde_json::Value>,
    policy_blocked: Option<serde_json::Value>,
    tools_called: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnResult {
    pub session_id: Option<String>,
    pub execution_mode: crate::config::ExecutionMode,
    pub final_text: Option<String>,
    pub terminal_outcome: AgentTurnOutcome,
    pub approval_required: Option<serde_json::Value>,
    pub policy_blocked: Option<serde_json::Value>,
    pub event_log: Vec<AgentTurnEvent>,
    pub tools_called: Vec<String>,
}

#[derive(Debug, Clone)]
struct ActiveMissionBudget {
    mission_id: String,
    baseline_total_cost_usd: f64,
    limit_usd: f64,
}

#[allow(clippy::struct_excessive_bools)]
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    tool_specs: Vec<ToolSpec>,
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
    security_policy: Arc<SecurityPolicy>,
    audit_logger: Option<Arc<AuditLogger>>,
    audit_strict: bool,
    prompt_builder: SystemPromptBuilder,
    tool_dispatcher: Box<dyn ToolDispatcher>,
    memory_loader: Box<dyn MemoryLoader>,
    config: crate::config::AgentConfig,
    mission_config: crate::config::MissionConfig,
    model_name: String,
    temperature: f64,
    workspace_dir: std::path::PathBuf,
    identity_config: crate::config::IdentityConfig,
    skills: Vec<crate::skills::Skill>,
    auto_save: bool,
    history: Vec<ConversationMessage>,
    classification_config: crate::config::QueryClassificationConfig,
    available_hints: Vec<String>,
    cost_tracker: Option<Arc<CostTracker>>,
    cost_config: crate::config::CostConfig,
    mission_execution_context: bool,
    active_mission_budget: Option<ActiveMissionBudget>,
    code_mode: bool,
    code_session_delegated: bool,
}

#[derive(Debug)]
pub enum AgentExecutionError {
    IterationBudgetExceeded {
        max_iterations: usize,
    },
    CostBudgetExceeded {
        current_usd: f64,
        limit_usd: f64,
        period: UsagePeriod,
    },
}

impl fmt::Display for AgentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentExecutionError::IterationBudgetExceeded { max_iterations } => write!(
                f,
                "Agent exceeded maximum tool iterations ({max_iterations})"
            ),
            AgentExecutionError::CostBudgetExceeded {
                current_usd,
                limit_usd,
                period,
            } => write!(
                f,
                "Budget exceeded: ${current_usd:.4} spent against ${limit_usd:.2} {period:?} limit. \
                 Set cost.enabled=false or increase limits to proceed."
            ),
        }
    }
}

impl std::error::Error for AgentExecutionError {}

pub struct AgentBuilder {
    provider: Option<Box<dyn Provider>>,
    tools: Option<Vec<Box<dyn Tool>>>,
    memory: Option<Arc<dyn Memory>>,
    observer: Option<Arc<dyn Observer>>,
    security_policy: Option<Arc<SecurityPolicy>>,
    audit_logger: Option<Arc<AuditLogger>>,
    audit_strict: Option<bool>,
    prompt_builder: Option<SystemPromptBuilder>,
    tool_dispatcher: Option<Box<dyn ToolDispatcher>>,
    memory_loader: Option<Box<dyn MemoryLoader>>,
    config: Option<crate::config::AgentConfig>,
    mission_config: Option<crate::config::MissionConfig>,
    model_name: Option<String>,
    temperature: Option<f64>,
    workspace_dir: Option<std::path::PathBuf>,
    identity_config: Option<crate::config::IdentityConfig>,
    skills: Option<Vec<crate::skills::Skill>>,
    auto_save: Option<bool>,
    classification_config: Option<crate::config::QueryClassificationConfig>,
    available_hints: Option<Vec<String>>,
    cost_tracker: Option<Arc<CostTracker>>,
    cost_config: Option<crate::config::CostConfig>,
    code_mode: bool,
    code_session_delegated: bool,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            provider: None,
            tools: None,
            memory: None,
            observer: None,
            security_policy: None,
            audit_logger: None,
            audit_strict: None,
            prompt_builder: None,
            tool_dispatcher: None,
            memory_loader: None,
            config: None,
            mission_config: None,
            model_name: None,
            temperature: None,
            workspace_dir: None,
            identity_config: None,
            skills: None,
            auto_save: None,
            classification_config: None,
            available_hints: None,
            cost_tracker: None,
            cost_config: None,
            code_mode: false,
            code_session_delegated: false,
        }
    }

    pub fn provider(mut self, provider: Box<dyn Provider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn tools(mut self, tools: Vec<Box<dyn Tool>>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn observer(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn security_policy(mut self, security_policy: Arc<SecurityPolicy>) -> Self {
        self.security_policy = Some(security_policy);
        self
    }

    pub fn audit_logger(mut self, audit_logger: Option<Arc<AuditLogger>>) -> Self {
        self.audit_logger = audit_logger;
        self
    }

    pub fn audit_strict(mut self, audit_strict: bool) -> Self {
        self.audit_strict = Some(audit_strict);
        self
    }

    pub fn prompt_builder(mut self, prompt_builder: SystemPromptBuilder) -> Self {
        self.prompt_builder = Some(prompt_builder);
        self
    }

    pub fn tool_dispatcher(mut self, tool_dispatcher: Box<dyn ToolDispatcher>) -> Self {
        self.tool_dispatcher = Some(tool_dispatcher);
        self
    }

    pub fn memory_loader(mut self, memory_loader: Box<dyn MemoryLoader>) -> Self {
        self.memory_loader = Some(memory_loader);
        self
    }

    pub fn config(mut self, config: crate::config::AgentConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn model_name(mut self, model_name: String) -> Self {
        self.model_name = Some(model_name);
        self
    }

    pub fn mission_config(mut self, mission_config: crate::config::MissionConfig) -> Self {
        self.mission_config = Some(mission_config);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn workspace_dir(mut self, workspace_dir: std::path::PathBuf) -> Self {
        self.workspace_dir = Some(workspace_dir);
        self
    }

    pub fn identity_config(mut self, identity_config: crate::config::IdentityConfig) -> Self {
        self.identity_config = Some(identity_config);
        self
    }

    pub fn skills(mut self, skills: Vec<crate::skills::Skill>) -> Self {
        self.skills = Some(skills);
        self
    }

    pub fn auto_save(mut self, auto_save: bool) -> Self {
        self.auto_save = Some(auto_save);
        self
    }

    pub fn classification_config(
        mut self,
        classification_config: crate::config::QueryClassificationConfig,
    ) -> Self {
        self.classification_config = Some(classification_config);
        self
    }

    pub fn available_hints(mut self, available_hints: Vec<String>) -> Self {
        self.available_hints = Some(available_hints);
        self
    }

    pub fn code_mode(mut self, code_mode: bool) -> Self {
        self.code_mode = code_mode;
        self
    }

    pub fn code_session_delegated(mut self, delegated: bool) -> Self {
        self.code_session_delegated = delegated;
        self
    }

    pub fn cost_tracker(mut self, cost_tracker: Option<Arc<CostTracker>>) -> Self {
        self.cost_tracker = cost_tracker;
        self
    }

    pub fn cost_config(mut self, cost_config: crate::config::CostConfig) -> Self {
        self.cost_config = Some(cost_config);
        self
    }

    pub fn build(self) -> Result<Agent> {
        let tools = self
            .tools
            .ok_or_else(|| anyhow::anyhow!("tools are required"))?;
        let tool_specs = tools.iter().map(|tool| tool.spec()).collect();

        Ok(Agent {
            provider: self
                .provider
                .ok_or_else(|| anyhow::anyhow!("provider is required"))?,
            tools,
            tool_specs,
            memory: self
                .memory
                .ok_or_else(|| anyhow::anyhow!("memory is required"))?,
            observer: self
                .observer
                .ok_or_else(|| anyhow::anyhow!("observer is required"))?,
            security_policy: self
                .security_policy
                .unwrap_or_else(|| Arc::new(SecurityPolicy::default())),
            audit_logger: self.audit_logger,
            audit_strict: self.audit_strict.unwrap_or(false),
            prompt_builder: self
                .prompt_builder
                .unwrap_or_else(SystemPromptBuilder::with_defaults),
            tool_dispatcher: self
                .tool_dispatcher
                .ok_or_else(|| anyhow::anyhow!("tool_dispatcher is required"))?,
            memory_loader: self
                .memory_loader
                .unwrap_or_else(|| Box::new(DefaultMemoryLoader::default())),
            config: self.config.unwrap_or_default(),
            mission_config: self.mission_config.unwrap_or_default(),
            model_name: self
                .model_name
                .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into()),
            temperature: self.temperature.unwrap_or(0.7),
            workspace_dir: self
                .workspace_dir
                .unwrap_or_else(|| std::path::PathBuf::from(".")),
            identity_config: self.identity_config.unwrap_or_default(),
            skills: self.skills.unwrap_or_default(),
            auto_save: self.auto_save.unwrap_or(false),
            history: Vec::new(),
            classification_config: self.classification_config.unwrap_or_default(),
            available_hints: self.available_hints.unwrap_or_default(),
            cost_tracker: self.cost_tracker,
            cost_config: self.cost_config.unwrap_or_default(),
            mission_execution_context: false,
            active_mission_budget: None,
            code_mode: self.code_mode,
            code_session_delegated: self.code_session_delegated,
        })
    }
}

impl Agent {
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    pub(crate) fn from_config_with_profile(config: &Config, profile: &str) -> Result<Self> {
        let bootstrap = bootstrap::BootstrapContext::from_config_with_profile(config, profile)?;

        Self::from_bootstrap(config, bootstrap)
    }

    pub(crate) fn code_from_config(config: &Config) -> Result<Self> {
        Self::code_from_config_with_delegated(config, false)
    }

    pub(crate) fn code_from_config_with_delegated(
        config: &Config,
        delegated: bool,
    ) -> Result<Self> {
        let bootstrap = bootstrap::BootstrapContext::from_config_with_profile(config, "code")?;
        let mut agent = Self::from_bootstrap(config, bootstrap)?;
        agent.code_mode = true;
        agent.code_session_delegated = delegated;
        Ok(agent)
    }

    pub fn history(&self) -> &[ConversationMessage] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub(crate) fn apply_next_request_budget_override(
        &self,
        actor: impl Into<String>,
        reason: Option<String>,
    ) -> Result<crate::cost::CostOverrideRecord> {
        if !self.cost_config.enabled {
            anyhow::bail!("Cost tracking is disabled for this session")
        }

        let Some(tracker) = &self.cost_tracker else {
            anyhow::bail!("Cost tracking is enabled, but the runtime cost tracker is unavailable")
        };

        let cost_service = CostService::new(Arc::clone(tracker));
        let now = Utc::now();
        let summary = cost_service.current_summary(now)?;
        let override_record = cost_service.apply_override(
            crate::cost::CostOverrideRequest {
                actor: actor.into(),
                scope: crate::cost::CostOverrideScope::NextRequest,
                reason,
                expires_at: None,
            },
            now,
        )?;
        self.emit_budget_override_event(
            &override_record,
            crate::observability::BudgetOverrideAction::Granted,
            summary.budget_state,
            summary.active_period,
        );
        Ok(override_record)
    }

    fn budget_surface(&self) -> &'static str {
        if self.code_mode {
            "code_session"
        } else if self.mission_execution_context {
            "mission"
        } else {
            "agent_loop"
        }
    }

    fn emit_budget_warning_event(
        &self,
        current_usd: f64,
        projected_usd: f64,
        limit_usd: f64,
        percent_used: f64,
        period: UsagePeriod,
        turn_context: Option<&TurnContext>,
    ) {
        if let Some(tracker) = &self.cost_tracker {
            self.observer.record_event(&ObserverEvent::BudgetWarning(
                crate::observability::BudgetThresholdEvent {
                    budget_state: crate::cost::BudgetState::Warning,
                    period,
                    current_usd,
                    projected_usd,
                    limit_usd,
                    percent_used,
                    session_id: turn_context
                        .and_then(|context| context.session_id.clone())
                        .unwrap_or_else(|| tracker.session_id().to_string()),
                    surface: Some(self.budget_surface().to_string()),
                },
            ));
        }
    }

    fn emit_budget_exceeded_event(
        &self,
        current_usd: f64,
        projected_usd: f64,
        limit_usd: f64,
        percent_used: f64,
        period: UsagePeriod,
        turn_context: Option<&TurnContext>,
    ) {
        if let Some(tracker) = &self.cost_tracker {
            self.observer.record_event(&ObserverEvent::BudgetExceeded(
                crate::observability::BudgetThresholdEvent {
                    budget_state: crate::cost::BudgetState::Exceeded,
                    period,
                    current_usd,
                    projected_usd,
                    limit_usd,
                    percent_used,
                    session_id: turn_context
                        .and_then(|context| context.session_id.clone())
                        .unwrap_or_else(|| tracker.session_id().to_string()),
                    surface: Some(self.budget_surface().to_string()),
                },
            ));
        }
    }

    fn emit_budget_override_event(
        &self,
        override_record: &crate::cost::CostOverrideRecord,
        action: crate::observability::BudgetOverrideAction,
        previous_state: crate::cost::BudgetState,
        period: Option<UsagePeriod>,
    ) {
        self.observer.record_event(&ObserverEvent::BudgetOverride(
            crate::observability::BudgetOverrideEvent {
                action,
                actor: override_record.actor.clone(),
                scope: override_record.scope,
                reason: override_record.reason.clone(),
                session_id: override_record.session_id.clone(),
                previous_state,
                period,
                override_id: Some(override_record.id.clone()),
                surface: Some(self.budget_surface().to_string()),
            },
        ));
    }

    pub(crate) fn session_cost_summary(
        &self,
        now: chrono::DateTime<Utc>,
    ) -> Result<Option<crate::cost::CostGovernanceSummary>> {
        if !self.cost_config.enabled {
            return Ok(None);
        }

        let Some(tracker) = &self.cost_tracker else {
            return Ok(None);
        };

        let cost_service = CostService::new(Arc::clone(tracker));
        cost_service.current_summary(now).map(Some)
    }

    pub(crate) fn record_agent_start_event(&self, provider: &str, model: &str) {
        self.observer.record_event(&ObserverEvent::AgentStart {
            provider: provider.to_string(),
            model: model.to_string(),
        });
    }

    pub(crate) fn record_agent_end_event(&self, provider: &str, model: &str, duration: Duration) {
        let (tokens_used, cost_usd) = self
            .cost_tracker
            .as_ref()
            .and_then(|tracker| tracker.get_summary().ok())
            .map(|summary| (Some(summary.total_tokens), Some(summary.session_cost_usd)))
            .unwrap_or((None, None));

        self.observer.record_event(&ObserverEvent::AgentEnd {
            provider: provider.to_string(),
            model: model.to_string(),
            duration,
            tokens_used,
            cost_usd,
        });
    }

    pub fn from_config(config: &Config) -> Result<Self> {
        let bootstrap = bootstrap::BootstrapContext::from_config(config)?;

        Self::from_bootstrap(config, bootstrap)
    }

    fn from_bootstrap(config: &Config, bootstrap: bootstrap::BootstrapContext) -> Result<Self> {
        let model_name = config
            .default_model
            .as_deref()
            .unwrap_or("anthropic/claude-sonnet-4-20250514")
            .to_string();

        let provider: Box<dyn Provider> = bootstrap::create_routed_provider(config, &model_name)?;

        Self::from_bootstrap_with_provider(config, bootstrap, provider)
    }

    pub(crate) fn from_bootstrap_with_provider(
        config: &Config,
        bootstrap: bootstrap::BootstrapContext,
        provider: Box<dyn Provider>,
    ) -> Result<Self> {
        let model_name = config
            .default_model
            .as_deref()
            .unwrap_or("anthropic/claude-sonnet-4-20250514")
            .to_string();

        let dispatcher_choice = config.agent.tool_dispatcher.as_str();
        let tool_dispatcher: Box<dyn ToolDispatcher> = match dispatcher_choice {
            "native" => Box::new(NativeToolDispatcher),
            "xml" => Box::new(XmlToolDispatcher),
            _ if provider.supports_native_tools() => Box::new(NativeToolDispatcher),
            _ => Box::new(XmlToolDispatcher),
        };

        let available_hints: Vec<String> =
            config.model_routes.iter().map(|r| r.hint.clone()).collect();

        let cerebro_configured = crate::memory::cerebro_configured(&config.memory);
        let memory_loader: Box<dyn MemoryLoader> = if cerebro_configured {
            Box::new(CerebroMemoryLoader::new(
                config.memory.cerebro.clone(),
                5,
                config.memory.min_relevance_score,
            ))
        } else {
            Box::new(DefaultMemoryLoader::new(
                5,
                config.memory.min_relevance_score,
            ))
        };

        Agent::builder()
            .provider(provider)
            .tools(bootstrap.tools)
            .memory(bootstrap.memory)
            .observer(bootstrap.observer)
            .security_policy(bootstrap.security)
            .audit_logger(Self::audit_logger_from_config(config)?)
            .audit_strict(config.security.audit.strict)
            .tool_dispatcher(tool_dispatcher)
            .memory_loader(memory_loader)
            .prompt_builder(SystemPromptBuilder::with_defaults())
            .config(config.agent.clone())
            .mission_config(config.mission.clone())
            .model_name(model_name)
            .temperature(config.default_temperature)
            .workspace_dir(config.workspace_dir.clone())
            .classification_config(config.query_classification.clone())
            .available_hints(available_hints)
            .identity_config(config.identity.clone())
            .skills(crate::skills::load_skills(&config.workspace_dir))
            .auto_save(config.memory.auto_save)
            .cost_tracker(bootstrap.cost_tracker)
            .cost_config(config.cost.clone())
            .build()
    }

    /// Build the audit logger from config, honoring strict initialization.
    fn audit_logger_from_config(config: &Config) -> Result<Option<Arc<AuditLogger>>> {
        let audit_config = config.security.audit.clone();
        if !audit_config.enabled {
            return Ok(None);
        }
        let corvus_dir = config
            .config_path
            .parent()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| config.workspace_dir.clone());
        match AuditLogger::new(audit_config.clone(), corvus_dir) {
            Ok(logger) => Ok(Some(Arc::new(logger))),
            Err(error) => {
                if audit_config.strict {
                    anyhow::bail!("Failed to initialize audit logger: {error}");
                }
                tracing::warn!("Failed to initialize audit logger: {error}");
                Ok(None)
            }
        }
    }

    fn trim_history(&mut self) {
        let max = self.config.max_history_messages;
        if self.history.len() <= max {
            return;
        }

        let mut system_messages = Vec::new();
        let mut other_messages = Vec::new();

        for msg in self.history.drain(..) {
            match &msg {
                ConversationMessage::Chat(chat) if chat.role == "system" => {
                    system_messages.push(msg);
                }
                _ => other_messages.push(msg),
            }
        }

        if other_messages.len() > max {
            let drop_count = other_messages.len() - max;
            other_messages.drain(0..drop_count);
        }

        self.history = system_messages;
        self.history.extend(other_messages);
    }

    fn build_system_prompt(&self) -> Result<String> {
        let instructions = self.tool_dispatcher.prompt_instructions(&self.tools);
        let ctx = PromptContext {
            workspace_dir: &self.workspace_dir,
            model_name: &self.model_name,
            tools: &self.tools,
            skills: &self.skills,
            identity_config: Some(&self.identity_config),
            dispatcher_instructions: &instructions,
            bootstrap_max_chars: if self.config.compact_context {
                Some(COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS)
            } else {
                None
            },
            code_mode: self.code_mode,
        };
        self.prompt_builder.build(&ctx)
    }

    async fn enforce_strict_memory_validation(
        &self,
        user_message: &str,
        candidate: String,
    ) -> String {
        crate::agent::validation::enforce_strict_validation(
            self.memory.as_ref(),
            self.provider.as_ref(),
            &self.model_name,
            self.temperature,
            user_message,
            candidate,
        )
        .await
    }

    async fn execute_tool_call(&self, call: &ParsedToolCall) -> ToolExecutionResult {
        let start = Instant::now();
        if call.name.starts_with("mcp.") {
            tracing::debug!(tool = %call.name, "Agent executing MCP tool call");
        }

        let Some(tool) = self.tools.iter().find(|tool| tool.name() == call.name) else {
            return self.finalize_tool_execution(
                call,
                start.elapsed(),
                format!("Unknown tool: {}", call.name),
                false,
                DispatchAction::Execute,
            );
        };

        let (result, success) = match tool.execute(call.arguments.clone()).await {
            Ok(result) => match self.handle_tool_result(call, &result, start.elapsed()) {
                Ok(output) => (output, result.success),
                Err(audit_error_result) => {
                    // Record the observer event before returning the audit error.
                    self.observer.record_event(&ObserverEvent::ToolCall {
                        tool: call.name.clone(),
                        duration: start.elapsed(),
                        success: audit_error_result.success,
                    });
                    return audit_error_result;
                }
            },
            Err(error) => {
                if call.name == "shell" {
                    let failed_result = crate::tools::ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Error executing {}: {error}", call.name)),
                        structured: None,
                    };
                    match self.handle_tool_result(call, &failed_result, start.elapsed()) {
                        Ok(output) => (output, false),
                        Err(audit_error_result) => {
                            self.observer.record_event(&ObserverEvent::ToolCall {
                                tool: call.name.clone(),
                                duration: start.elapsed(),
                                success: audit_error_result.success,
                            });
                            return audit_error_result;
                        }
                    }
                } else {
                    (format!("Error executing {}: {error}", call.name), false)
                }
            }
        };

        self.finalize_tool_execution(
            call,
            start.elapsed(),
            result,
            success,
            DispatchAction::Execute,
        )
    }

    #[allow(clippy::result_large_err)]
    fn handle_tool_result(
        &self,
        call: &ParsedToolCall,
        result: &crate::tools::ToolResult,
        duration: Duration,
    ) -> std::result::Result<String, ToolExecutionResult> {
        if call.name == "shell" {
            if let Err(error) = self.log_shell_audit_event(call, result, duration) {
                let base_output = if result.success {
                    result.output.clone()
                } else {
                    format!(
                        "Error: {}",
                        result
                            .error
                            .clone()
                            .unwrap_or_else(|| result.output.clone())
                    )
                };
                let audit_message = if let Some(original_error) = &result.error {
                    format!("Audit logging failed: {error}; original shell error: {original_error}")
                } else {
                    format!("Audit logging failed: {error}")
                };
                return Err(ToolExecutionResult {
                    name: call.name.clone(),
                    output: format!("{base_output}\n\n[AUDIT ERROR: {error}]"),
                    success: result.success,
                    tool_call_id: call.tool_call_id.clone(),
                    action: DispatchAction::ApprovalRequired(audit_message),
                });
            }
        } else if call.name == "browser" {
            self.log_browser_security_event(result);
        }

        if call.name.starts_with("mcp.") && !result.success {
            tracing::warn!(tool = %call.name, "MCP tool call returned failure status");
        }

        Ok(if result.success {
            result.output.clone()
        } else {
            format!(
                "Error: {}",
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| result.output.clone())
            )
        })
    }

    fn finalize_tool_execution(
        &self,
        call: &ParsedToolCall,
        duration: Duration,
        output: String,
        success: bool,
        action: DispatchAction,
    ) -> ToolExecutionResult {
        self.observer.record_event(&ObserverEvent::ToolCall {
            tool: call.name.clone(),
            duration,
            success,
        });

        ToolExecutionResult {
            name: call.name.clone(),
            output,
            success,
            tool_call_id: call.tool_call_id.clone(),
            action,
        }
    }

    fn log_shell_audit_event(
        &self,
        call: &ParsedToolCall,
        result: &crate::tools::ToolResult,
        duration: Duration,
    ) -> anyhow::Result<()> {
        let Some(logger) = &self.audit_logger else {
            return Ok(());
        };

        let raw_command = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let redacted_command = redact_observer_payload(raw_command);
        let approved = call
            .arguments
            .get("approved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let structured = result.structured.as_ref();
        let risk_level = structured
            .and_then(|v| v.get("risk_level"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let sandbox_backend = structured
            .and_then(|v| v.get("sandbox_backend"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let allowed = structured
            .and_then(|v| v.get("approved"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);

        if let Err(error) = logger.log_command_event(CommandExecutionLog {
            channel: "agent",
            command: &redacted_command,
            risk_level,
            approved,
            allowed,
            success: result.success,
            duration_ms,
            sandbox_backend,
        }) {
            if self.audit_strict {
                anyhow::bail!("Failed to write shell command audit event: {error}");
            }
            tracing::warn!("Failed to write shell command audit event: {error}");
        }

        Ok(())
    }

    fn log_browser_security_event(&self, result: &crate::tools::ToolResult) {
        let Some(logger) = &self.audit_logger else {
            return;
        };

        let Some(sidecar_health) = result
            .structured
            .as_ref()
            .and_then(|v| v.get("computer_use"))
            .and_then(|v| v.get("sidecar_health"))
            .cloned()
        else {
            return;
        };

        let status = sidecar_health
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let isolation_type = sidecar_health
            .get("isolation")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let runtime = sidecar_health
            .get("isolation")
            .and_then(|v| v.get("runtime"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let event = AuditEvent::new(AuditEventType::SecurityEvent)
            .with_actor("agent".to_string(), None, None)
            .with_action(
                format!(
                    "browser sidecar health status={status} isolation_type={isolation_type} runtime={runtime}"
                ),
                "low".to_string(),
                false,
                true,
            );

        if let Err(error) = logger.log(&event) {
            if self.audit_strict {
                tracing::error!("Failed to write browser sidecar security audit event: {error}");
            } else {
                tracing::warn!("Failed to write browser sidecar security audit event: {error}");
            }
        }
    }

    async fn execute_tools(&self, calls: &[ParsedToolCall]) -> Vec<ToolExecutionResult> {
        if !self.config.parallel_tools {
            let mut results = Vec::with_capacity(calls.len());
            for call in calls {
                results.push(self.execute_tool_call(call).await);
            }
            return results;
        }

        join_all(calls.iter().map(|call| self.execute_tool_call(call))).await
    }

    /// Resolve a model identifier, stripping "hint:" prefix if present.
    /// This ensures we get the concrete model ID for cost tracking purposes.
    fn resolve_model_for_pricing(&self, model: &str) -> String {
        // If model starts with "hint:", strip it to get the base hint name
        // The provider routing logic will handle the actual resolution,
        // but for pricing we just need a consistent identifier
        if let Some(hint) = model.strip_prefix("hint:") {
            // For now, return the hint portion without the prefix
            // In a future enhancement, this could call into the provider's resolve logic
            hint.to_string()
        } else {
            model.to_string()
        }
    }

    /// Look up model pricing from CostConfig, returning (input_per_million, output_per_million).
    fn model_pricing(&self, model: &str) -> (f64, f64) {
        if let Some(pricing) = self.cost_config.prices.get(model) {
            return (pricing.input, pricing.output);
        }
        // Try matching by suffix (e.g. "claude-sonnet-4-20250514" matches
        // "anthropic/claude-sonnet-4-20250514")
        for (key, pricing) in &self.cost_config.prices {
            if key.ends_with(model) || model.ends_with(key.as_str()) {
                return (pricing.input, pricing.output);
            }
        }
        (0.0, 0.0)
    }

    fn history_char_count(&self) -> usize {
        self.history
            .iter()
            .map(|msg| match msg {
                ConversationMessage::Chat(chat) => chat.content.len(),
                ConversationMessage::AssistantToolCalls { text, tool_calls } => {
                    let text_len = text.as_ref().map_or(0, String::len);
                    // Include serialized tool_calls in the count (name + arguments)
                    let tool_calls_len: usize = tool_calls
                        .iter()
                        .map(|call| call.name.len() + call.arguments.len())
                        .sum();
                    text_len + tool_calls_len
                }
                ConversationMessage::ToolResults(results) => {
                    results.iter().map(|r| r.content.len()).sum()
                }
            })
            .sum()
    }

    /// Estimate the total character count for the request payload, including tool specs if applicable.
    fn estimate_request_char_count(&self) -> usize {
        let mut total = self.history_char_count();

        // Include tool specs if they would be sent in the request
        if self.tool_dispatcher.should_send_tool_specs() {
            // Serialize tool_specs to estimate their size in the request
            if let Ok(json) = serde_json::to_string(&self.tool_specs) {
                total += json.len();
            }
        }

        total
    }

    /// Estimate the cost for an upcoming LLM call based on conversation size.
    fn estimate_request_cost(&self, model: &str) -> f64 {
        // Resolve hint: prefix to get the actual model for pricing lookup
        let resolved_model = self.resolve_model_for_pricing(model);
        let (input_price, output_price) = self.model_pricing(&resolved_model);
        if input_price == 0.0 && output_price == 0.0 {
            return 0.0;
        }
        // Rough estimate: count chars in full request (history + tool specs if applicable), ~4 chars per token
        let input_chars = self.estimate_request_char_count();
        let estimated_input_tokens = (input_chars / 4) as u64;
        let estimated_output_tokens = PRE_FLIGHT_ESTIMATED_OUTPUT_TOKENS;
        let input_cost = (estimated_input_tokens as f64 / 1_000_000.0) * input_price;
        let output_cost = (estimated_output_tokens as f64 / 1_000_000.0) * output_price;
        input_cost + output_cost
    }

    /// Run pre-flight budget check. Returns Ok(()) if allowed or warning,
    /// returns Err if budget exceeded.
    fn enforce_budget_check(
        &self,
        model: &str,
        turn_context: &TurnContext,
    ) -> Result<Option<crate::cost::CostBudgetReservation>> {
        let Some(tracker) = &self.cost_tracker else {
            return Ok(None);
        };

        let cost_service = CostService::new(Arc::clone(tracker));
        let estimated_cost = self.estimate_request_cost(model);
        match cost_service.evaluate_request(
            estimated_cost,
            self.current_mission_budget_scope()?,
            Utc::now(),
        )? {
            BudgetEvaluation::Proceed {
                check: BudgetCheck::Allowed,
                override_applied,
                reservation,
            } => {
                if let Some(override_applied) = override_applied {
                    self.emit_budget_override_event(
                        &override_applied,
                        crate::observability::BudgetOverrideAction::Consumed,
                        crate::cost::BudgetState::Allowed,
                        None,
                    );
                }
                Ok(reservation)
            }
            BudgetEvaluation::Proceed {
                check:
                    BudgetCheck::Warning {
                        current_usd,
                        projected_usd,
                        limit_usd,
                        percent_used,
                        period,
                        ..
                    },
                override_applied,
                reservation,
            } => {
                if let Some(override_applied) = override_applied {
                    self.emit_budget_override_event(
                        &override_applied,
                        crate::observability::BudgetOverrideAction::Consumed,
                        crate::cost::BudgetState::Warning,
                        Some(period),
                    );
                }
                self.emit_budget_warning_event(
                    current_usd,
                    projected_usd,
                    limit_usd,
                    percent_used,
                    period,
                    Some(turn_context),
                );
                Ok(reservation)
            }
            BudgetEvaluation::Proceed {
                check:
                    BudgetCheck::Exceeded {
                        current_usd: _,
                        projected_usd: _,
                        limit_usd: _,
                        percent_used: _,
                        period,
                        ..
                    },
                override_applied,
                reservation,
            } => {
                if let Some(override_applied) = override_applied {
                    self.emit_budget_override_event(
                        &override_applied,
                        crate::observability::BudgetOverrideAction::Consumed,
                        crate::cost::BudgetState::Exceeded,
                        Some(period),
                    );
                }
                Ok(reservation)
            }
            BudgetEvaluation::Blocked {
                check:
                    BudgetCheck::Exceeded {
                        current_usd,
                        projected_usd,
                        limit_usd,
                        percent_used,
                        period,
                        ..
                    },
            } => {
                self.emit_budget_exceeded_event(
                    current_usd,
                    projected_usd,
                    limit_usd,
                    percent_used,
                    period,
                    Some(turn_context),
                );
                Err(AgentExecutionError::CostBudgetExceeded {
                    current_usd,
                    limit_usd,
                    period,
                }
                .into())
            }
            BudgetEvaluation::Blocked { check } => Err(anyhow::anyhow!(
                "Budget evaluation blocked request unexpectedly: {check:?}"
            )),
        }
    }

    fn current_mission_budget_scope(&self) -> Result<Option<MissionBudgetScope>> {
        let Some(active_budget) = &self.active_mission_budget else {
            return Ok(None);
        };

        let Some(tracker) = &self.cost_tracker else {
            return Ok(None);
        };

        let current_total_cost_usd = tracker.cumulative_total_cost_usd();
        if current_total_cost_usd < active_budget.baseline_total_cost_usd {
            anyhow::bail!(
                "mission budget baseline exceeded current cumulative total; runtime cost state regressed"
            );
        }

        Ok(Some(MissionBudgetScope {
            mission_id: active_budget.mission_id.clone(),
            current_usd: current_total_cost_usd - active_budget.baseline_total_cost_usd,
            limit_usd: active_budget.limit_usd,
        }))
    }

    fn begin_active_mission_budget(&mut self, mission_id: &str) -> Result<()> {
        let Some(tracker) = &self.cost_tracker else {
            self.active_mission_budget = None;
            return Ok(());
        };

        self.active_mission_budget = Some(ActiveMissionBudget {
            mission_id: mission_id.to_string(),
            baseline_total_cost_usd: tracker.cumulative_total_cost_usd(),
            limit_usd: f64::from(self.mission_config.max_estimated_cost_cents) / 100.0,
        });
        Ok(())
    }

    fn end_active_mission_budget(&mut self) {
        self.active_mission_budget = None;
    }

    /// Record token usage after a successful LLM call using estimated tokens.
    fn record_estimated_usage(
        &mut self,
        model: &str,
        response: &ChatResponse,
        turn_context: &TurnContext,
        reservation: Option<&crate::cost::CostBudgetReservation>,
    ) {
        let Some(tracker) = &self.cost_tracker else {
            return;
        };
        // Resolve hint: prefix to get the actual model for pricing lookup
        let resolved_model = self.resolve_model_for_pricing(model);
        let (input_price, output_price) = self.model_pricing(&resolved_model);

        // Estimate input tokens from full request size (history + tool specs if applicable)
        let input_chars = self.estimate_request_char_count();
        let estimated_input_tokens = (input_chars / 4) as u64;

        // Estimate output tokens from response length (text + tool_calls)
        let text_chars = response.text.as_deref().map_or(0, str::len);
        let tool_calls_chars: usize = response
            .tool_calls
            .iter()
            .map(|call| call.name.len() + call.arguments.len())
            .sum();
        let output_chars = text_chars + tool_calls_chars;
        let actual_output_tokens = std::cmp::max((output_chars / 4) as u64, 1);

        if (actual_output_tokens as f64) > (PRE_FLIGHT_ESTIMATED_OUTPUT_TOKENS as f64 * 1.5) {
            tracing::warn!(
                model,
                request_context = turn_context.session_id.as_deref().unwrap_or("agent-turn"),
                estimated_output_tokens = PRE_FLIGHT_ESTIMATED_OUTPUT_TOKENS,
                actual_output_tokens,
                "LLM output tokens significantly exceeded the pre-flight estimate"
            );
        }

        let usage = TokenUsage::new(
            &resolved_model,
            estimated_input_tokens,
            actual_output_tokens,
            input_price,
            output_price,
        );

        if let Err(error) = tracker.record_usage(usage) {
            tracing::warn!("Failed to record cost usage: {error}");
            return;
        }

        if let Some(reservation) = reservation {
            tracker.commit_budget_reservation(&reservation.id);
        }
    }

    fn classify_model(&self, user_message: &str) -> String {
        if let Some(hint) = super::classifier::classify(&self.classification_config, user_message) {
            if self.available_hints.contains(&hint) {
                tracing::info!(hint = hint.as_str(), "Auto-classified query");
                return format!("hint:{hint}");
            }
        }
        self.model_name.clone()
    }

    pub async fn prepare_turn(&mut self, user_message: &str) -> Result<String> {
        self.prepare_turn_with_context(user_message, &TurnContext::default())
            .await
    }

    async fn prepare_turn_with_context(
        &mut self,
        user_message: &str,
        turn_context: &TurnContext,
    ) -> Result<String> {
        if self.history.is_empty() {
            let system_prompt = self.build_system_prompt()?;
            self.history
                .push(ConversationMessage::Chat(ChatMessage::system(
                    system_prompt,
                )));
        }

        if self.auto_save {
            let _ = self
                .memory
                .store(
                    "user_msg",
                    user_message,
                    MemoryCategory::Conversation,
                    turn_context.session_id.as_deref(),
                )
                .await;
        }

        let context = self
            .memory_loader
            .load_context(
                self.memory.as_ref(),
                user_message,
                turn_context.session_id.as_deref(),
            )
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(error = %error, "Memory context load failed");
                String::new()
            });

        let enriched = if context.is_empty() {
            user_message.to_string()
        } else {
            format!("{context}{user_message}")
        };

        self.history
            .push(ConversationMessage::Chat(ChatMessage::user(enriched)));

        Ok(self.classify_model(user_message))
    }

    pub async fn step(
        &mut self,
        effective_model: &str,
        user_message: &str,
    ) -> Result<Option<String>> {
        self.step_with_context(effective_model, user_message, &TurnContext::default())
            .await
            .map(|outcome| outcome.final_text)
    }

    async fn finalize_text_response(
        &mut self,
        user_message: &str,
        text: String,
        response_text: Option<String>,
        turn_context: &TurnContext,
    ) -> Result<Option<String>> {
        let final_text = if text.is_empty() {
            response_text.unwrap_or_default()
        } else {
            text
        };
        let final_text = self
            .enforce_strict_memory_validation(user_message, final_text)
            .await;

        self.history
            .push(ConversationMessage::Chat(ChatMessage::assistant(
                final_text.clone(),
            )));
        self.trim_history();

        if self.auto_save {
            let summary = truncate_with_ellipsis(&final_text, 100);
            let _ = self
                .memory
                .store(
                    "assistant_resp",
                    &summary,
                    MemoryCategory::Daily,
                    turn_context.session_id.as_deref(),
                )
                .await;
        }

        Ok(Some(final_text))
    }

    fn record_tool_response(
        &mut self,
        text: String,
        response_text: Option<String>,
        response_tool_calls: &[ParsedToolCall],
    ) {
        if response_tool_calls.is_empty() {
            if !text.is_empty() {
                self.history
                    .push(ConversationMessage::Chat(ChatMessage::assistant(text)));
            }
            return;
        }

        self.history.push(ConversationMessage::AssistantToolCalls {
            text: response_text,
            tool_calls: response_tool_calls
                .iter()
                .enumerate()
                .map(|(index, call)| crate::providers::ToolCall {
                    id: call
                        .tool_call_id
                        .clone()
                        .unwrap_or_else(|| Self::tool_call_key(index, call)),
                    name: call.name.clone(),
                    arguments: call.arguments.to_string(),
                })
                .collect(),
        });
    }

    async fn execute_gated_tool_calls(
        &mut self,
        calls: &[ParsedToolCall],
    ) -> Vec<ToolExecutionResult> {
        let execution_origin = if self.mission_execution_context {
            ExecutionOrigin::Mission
        } else {
            ExecutionOrigin::Standard
        };
        let mut approved_calls = Vec::new();
        let mut approved_call_keys = Vec::new();
        let mut results_by_call_id = HashMap::new();

        for (index, call) in calls.iter().enumerate() {
            let key = Self::tool_call_key(index, call);
            match evaluate_tool_risk_with_policy_for_origin(
                &call.name,
                &self.security_policy,
                execution_origin,
            ) {
                DispatchAction::ApprovalRequired(reason) => {
                    results_by_call_id.insert(key, Self::approval_required_result(call, reason));
                }
                DispatchAction::Blocked { code, reason } => {
                    results_by_call_id.insert(key, Self::blocked_result(call, &code, reason));
                }
                DispatchAction::Execute => {
                    approved_calls.push(call.clone());
                    approved_call_keys.push(key);
                }
            }
        }

        for (result, key) in self
            .execute_tools(&approved_calls)
            .await
            .into_iter()
            .zip(approved_call_keys)
        {
            results_by_call_id.insert(key, result);
        }

        calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                results_by_call_id
                    .remove(&Self::tool_call_key(index, call))
                    .expect("every call must have a corresponding result")
            })
            .collect()
    }

    fn tool_call_key(index: usize, call: &ParsedToolCall) -> String {
        call.tool_call_id
            .clone()
            .unwrap_or_else(|| format!("{}#{index}", call.name))
    }

    fn approval_denial_from_results(
        calls: &[ParsedToolCall],
        results: &[ToolExecutionResult],
    ) -> Option<serde_json::Value> {
        calls
            .iter()
            .zip(results)
            .find_map(|(call, result)| match &result.action {
                DispatchAction::ApprovalRequired(reason) => Some(serde_json::json!({
                    "code": "approval_required",
                    "tool": call.name,
                    "reason": reason,
                })),
                DispatchAction::Blocked { code, reason } => Some(serde_json::json!({
                    "code": code,
                    "tool": call.name,
                    "reason": reason,
                })),
                DispatchAction::Execute => None,
            })
    }

    fn approval_required_result(call: &ParsedToolCall, reason: String) -> ToolExecutionResult {
        ToolExecutionResult {
            name: call.name.clone(),
            output: crate::approval::structured_denial_text(&call.name, &reason),
            success: false,
            tool_call_id: call.tool_call_id.clone(),
            action: DispatchAction::ApprovalRequired(reason),
        }
    }

    fn blocked_result(call: &ParsedToolCall, code: &str, reason: String) -> ToolExecutionResult {
        ToolExecutionResult {
            name: call.name.clone(),
            output: crate::approval::structured_policy_denial_text(&call.name, code, &reason),
            success: false,
            tool_call_id: call.tool_call_id.clone(),
            action: DispatchAction::Blocked {
                code: code.to_string(),
                reason,
            },
        }
    }

    pub async fn turn(&mut self, user_message: &str) -> Result<String> {
        let session_id = if self.code_mode {
            Some(Self::code_session_id())
        } else {
            None
        };
        let turn_context = session_id
            .as_ref()
            .map(|session_id| TurnContext::with_session(session_id.clone()))
            .unwrap_or_default();

        let result = self.turn_with_context(user_message, turn_context).await;

        if let Some(session_id) = session_id.as_deref() {
            let code_result = match &result {
                Ok(turn_result) => CodeSessionResult::parse_from_output(
                    turn_result.final_text.as_deref().unwrap_or_default(),
                    session_id,
                ),
                Err(error) => Self::code_session_result_from_error(session_id, error),
            };
            self.record_code_session_result(&code_result)?;
        }

        result.map(|turn_result| turn_result.final_text.unwrap_or_default())
    }

    pub async fn turn_with_context(
        &mut self,
        user_message: &str,
        turn_context: TurnContext,
    ) -> Result<AgentTurnResult> {
        let effective_model = self
            .prepare_turn_with_context(user_message, &turn_context)
            .await?;
        let mut approval_required = None;
        let mut policy_blocked = None;
        let mut event_log = vec![AgentTurnEvent::Prepared];
        let mut all_tools_called: Vec<String> = Vec::new();

        for _ in 0..self.config.max_tool_iterations {
            let outcome = self
                .step_with_context(&effective_model, user_message, &turn_context)
                .await?;
            if approval_required.is_none() {
                approval_required = outcome.approval_required.clone();
            }
            if policy_blocked.is_none() {
                policy_blocked = outcome.policy_blocked.clone();
            }
            all_tools_called.extend(outcome.tools_called);
            if let Some(final_text) = outcome.final_text {
                event_log.push(AgentTurnEvent::Completed);
                return Ok(AgentTurnResult {
                    session_id: turn_context.session_id,
                    execution_mode: self.config.execution_mode,
                    final_text: Some(final_text),
                    terminal_outcome: AgentTurnOutcome::Completed,
                    approval_required,
                    policy_blocked,
                    event_log,
                    tools_called: all_tools_called,
                });
            }
        }

        Err(AgentExecutionError::IterationBudgetExceeded {
            max_iterations: self.config.max_tool_iterations,
        }
        .into())
    }

    /// Resolve the code-session identifier, preferring CORVUS_SESSION_ID when set.
    /// Falls back to a random UUID v4 for each session when unset.
    fn code_session_id() -> String {
        std::env::var("CORVUS_SESSION_ID").unwrap_or_else(|_| Uuid::new_v4().to_string())
    }

    fn code_session_result_from_error(
        session_id: &str,
        error: &anyhow::Error,
    ) -> CodeSessionResult {
        let raw = error.to_string();
        let redacted = redact_observer_payload(&raw);
        let status = if redacted.contains("maximum tool iterations")
            || redacted.contains("iteration budget")
            || redacted.contains("timeout")
        {
            CodeSessionStatus::BudgetExceeded
        } else if redacted.contains("approval") || redacted.contains("blocked") {
            CodeSessionStatus::Blocked
        } else {
            CodeSessionStatus::Error
        };

        let mut result = CodeSessionResult::from_error(
            session_id,
            status,
            format!("Session terminated: {redacted}"),
        );
        result.blockers.push(redacted);
        result
    }

    fn record_code_session_result(&self, result: &CodeSessionResult) -> Result<()> {
        let mut changed_files = result.changed_files.clone();
        changed_files.extend(result.files_changed.iter().map(|file| file.path.clone()));

        let mut commands: Vec<String> = result
            .commands
            .iter()
            .map(|cmd| redact_observer_payload(&cmd.command))
            .collect();
        commands.extend(
            result
                .commands_executed
                .iter()
                .map(|cmd| redact_observer_payload(cmd)),
        );

        let mut validations: Vec<String> = result
            .validations
            .iter()
            .map(|validation| {
                let status = if validation.success { "pass" } else { "fail" };
                let command = redact_observer_payload(&validation.command);
                format!("{status}:{command}")
            })
            .collect();
        validations.extend(result.validation_outcomes.iter().map(|validation| {
            let status = if validation.passed { "pass" } else { "fail" };
            let command = redact_observer_payload(&validation.command);
            format!("{status}:{command}")
        }));

        let blockers: Vec<String> = result
            .blockers
            .iter()
            .map(|b| redact_observer_payload(b))
            .collect();
        let pending_work: Vec<String> = result
            .pending_work
            .iter()
            .map(|p| redact_observer_payload(p))
            .collect();
        let summary = redact_observer_payload(&result.summary);

        let event = ObserverEvent::CodeSessionCompleted {
            session_id: result.session_id.clone(),
            status: result.status.as_str().to_string(),
            summary: summary.clone(),
            changed_files: changed_files.clone(),
            commands: commands.clone(),
            validations: validations.clone(),
            blockers: blockers.clone(),
            pending_work: pending_work.clone(),
            delegated: self.code_session_delegated,
        };

        self.observer.record_event(&event);

        if let Some(logger) = &self.audit_logger {
            if let Err(error) = logger.log_code_session_event(CodeSessionAuditLog {
                session_id: result.session_id.clone(),
                status: result.status.as_str().to_string(),
                summary,
                changed_files,
                commands,
                validations,
                blockers,
                pending_work,
                delegated: self.code_session_delegated,
            }) {
                if self.audit_strict {
                    anyhow::bail!("Failed to write code-session audit event: {error}");
                }
                tracing::warn!("Failed to write code-session audit event: {error}");
            }
        }

        Ok(())
    }

    fn mission_id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        format!("mission-{nanos}")
    }

    fn build_mission_coordinator(&self) -> Result<MissionCoordinator> {
        let governance: crate::agent::mission::MissionGovernance =
            self.mission_config.clone().into();

        if self.cost_config.enabled {
            let tracker = self.cost_tracker.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "mission runtime cost tracker unavailable while cost tracking is enabled"
                )
            })?;

            return MissionCoordinator::new_with_runtime_cost_tracker(
                governance,
                Arc::clone(tracker),
            )
            .map_err(Self::mission_error);
        }

        Ok(MissionCoordinator::new(governance))
    }

    fn build_mission_plan(&self, objective: &str, resume_from: Option<u32>) -> MissionPlan {
        let mut plan = MissionCoordinator::plan_for_objective(objective);
        plan.resume.last_successful_checkpoint = resume_from;
        plan
    }

    async fn execute_mission_checkpoint(
        &mut self,
        checkpoint: &crate::agent::mission::MissionCheckpoint,
    ) -> Result<String> {
        self.mission_execution_context = true;
        let result = self.turn(&checkpoint.objective_fragment).await;
        self.mission_execution_context = false;
        result
    }

    fn replan_after_failure(
        &self,
        plan: &MissionPlan,
        _failed_checkpoint: u32,
        _reason: &str,
    ) -> MissionPlan {
        plan.clone()
    }

    fn mission_error(reason: MissionTerminationReason) -> anyhow::Error {
        anyhow::anyhow!("mission lifecycle error: {reason:?}")
    }

    fn mission_reason_label(reason: &MissionTerminationReason) -> &'static str {
        match reason {
            MissionTerminationReason::BudgetExhausted => "budget_exhausted",
            MissionTerminationReason::SlaExceeded => "sla_exceeded",
            MissionTerminationReason::PolicyDenied => "policy_denied",
            MissionTerminationReason::ApprovalDenied => "approval_denied",
            MissionTerminationReason::GuardrailViolation => "guardrail_violation",
            MissionTerminationReason::Unrecoverable => "unrecoverable",
            MissionTerminationReason::GovernanceConstraintViolated => {
                "governance_constraint_violated"
            }
            MissionTerminationReason::InvalidStateTransition => "invalid_state_transition",
            MissionTerminationReason::AlreadyTerminalState => "already_terminal_state",
        }
    }

    fn mission_termination_reason_from_error(error_text: &str) -> MissionTerminationReason {
        if error_text.contains("mission_policy_denied") {
            return MissionTerminationReason::PolicyDenied;
        }
        if error_text.contains("approval_required") {
            return MissionTerminationReason::ApprovalDenied;
        }
        MissionTerminationReason::Unrecoverable
    }

    fn sanitize_observer_error(
        raw_message: &str,
        headers: &[(String, String)],
        diagnostic: &str,
    ) -> String {
        let runtime_redacted = crate::tools::redact_runtime_error(raw_message);
        let redacted_headers = crate::tools::http_request::redact_headers_for_display(headers);
        let redacted_headers_json = serde_json::to_string(&redacted_headers).unwrap_or_default();

        #[cfg(feature = "mcp-runtime")]
        let diagnostic_redacted = crate::tools::mcp::client::redact_diagnostic(
            diagnostic,
            headers
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );

        #[cfg(not(feature = "mcp-runtime"))]
        let diagnostic_redacted = diagnostic.to_string();

        let payload = format!(
            "{runtime_redacted}; headers={redacted_headers_json}; diagnostic={diagnostic_redacted}"
        );
        redact_observer_payload(&payload)
    }

    fn terminated_mission_outcome(
        &self,
        coordinator: &MissionCoordinator,
        mission_id: &str,
        reason: MissionTerminationReason,
        mission_start: Instant,
        checkpoint_index: Option<u32>,
        rollback: bool,
    ) -> Result<MissionOutcome> {
        let _ = coordinator.transition(MissionState::Terminated);
        let resume_metadata = coordinator.resume_metadata().map_err(Self::mission_error)?;
        let checkpoints_completed = resume_metadata
            .last_successful_checkpoint
            .map_or(0, |index| index + 1);
        let duration = mission_start.elapsed();
        let termination_reason = Self::mission_reason_label(&reason).to_string();

        self.observer
            .record_event(&ObserverEvent::MissionGuardrailViolation {
                mission_id: mission_id.to_string(),
                checkpoint_index,
                guardrail: "mission_governance".to_string(),
                termination_reason: termination_reason.clone(),
                detail: redact_observer_payload(&termination_reason),
            });
        self.observer
            .record_event(&ObserverEvent::MissionTerminated {
                mission_id: mission_id.to_string(),
                checkpoint_index,
                termination_reason,
                duration,
                rollback,
            });

        Ok(MissionOutcome {
            mission_id: mission_id.to_string(),
            state: MissionState::Terminated,
            termination: Some(reason),
            checkpoints_completed,
            resume_metadata,
        })
    }

    async fn run_mission_plan(
        &mut self,
        coordinator: &MissionCoordinator,
        mission_id: &str,
        mission_start: Instant,
        mut plan: MissionPlan,
    ) -> Result<MissionOutcome> {
        if let Err(reason) = coordinator.validate_governance() {
            return self.terminated_mission_outcome(
                coordinator,
                mission_id,
                reason,
                mission_start,
                None,
                false,
            );
        }

        if let Some(resume_index) = plan.resume.last_successful_checkpoint {
            coordinator
                .record_checkpoint_success(resume_index)
                .map_err(Self::mission_error)?;
        }

        coordinator
            .transition(MissionState::Planned)
            .map_err(Self::mission_error)?;
        coordinator
            .transition(MissionState::Active)
            .map_err(Self::mission_error)?;

        let start_index = plan
            .resume
            .last_successful_checkpoint
            .and_then(|index| usize::try_from(index).ok())
            .map_or(0, |index| index.saturating_add(1));

        let mut checkpoint_position = start_index;

        while checkpoint_position < plan.checkpoints.len() {
            let pending_checkpoint_index = plan
                .checkpoints
                .get(checkpoint_position)
                .map(|checkpoint| checkpoint.index);

            if let Err(reason) = coordinator.enforce_pre_checkpoint() {
                return self.terminated_mission_outcome(
                    coordinator,
                    mission_id,
                    reason,
                    mission_start,
                    pending_checkpoint_index,
                    false,
                );
            }

            let checkpoint = plan.checkpoints[checkpoint_position].clone();
            let (next_plan, outcome, increment) = self
                .process_mission_checkpoint(
                    coordinator,
                    mission_id,
                    mission_start,
                    plan,
                    &checkpoint,
                )
                .await?;

            plan = next_plan;
            if let Some(outcome) = outcome {
                return Ok(outcome);
            }

            if increment {
                checkpoint_position = checkpoint_position.saturating_add(1);
            }
        }

        coordinator
            .transition(MissionState::Completed)
            .map_err(Self::mission_error)?;
        let resume_metadata = coordinator.resume_metadata().map_err(Self::mission_error)?;
        let checkpoints_completed = resume_metadata
            .last_successful_checkpoint
            .map_or(0, |index| index + 1);
        self.observer
            .record_event(&ObserverEvent::MissionCompleted {
                mission_id: mission_id.to_string(),
                checkpoints_completed,
                duration: mission_start.elapsed(),
            });

        Ok(MissionOutcome {
            mission_id: mission_id.to_string(),
            state: MissionState::Completed,
            termination: None,
            checkpoints_completed,
            resume_metadata,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn process_mission_checkpoint(
        &mut self,
        coordinator: &MissionCoordinator,
        mission_id: &str,
        mission_start: Instant,
        mut plan: MissionPlan,
        checkpoint: &MissionCheckpoint,
    ) -> Result<(MissionPlan, Option<MissionOutcome>, bool)> {
        let checkpoint_start = Instant::now();
        self.observer
            .record_event(&ObserverEvent::MissionCheckpointProgress {
                mission_id: mission_id.to_string(),
                checkpoint_index: checkpoint.index,
                status: "started".to_string(),
                duration: std::time::Duration::ZERO,
            });

        match self.execute_mission_checkpoint(checkpoint).await {
            Ok(_) => {
                let checkpoint_elapsed_ms =
                    u64::try_from(checkpoint_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                if let Err(reason) =
                    coordinator.record_checkpoint_accounting(checkpoint_elapsed_ms, 0)
                {
                    return Ok((
                        plan,
                        Some(self.terminated_mission_outcome(
                            coordinator,
                            mission_id,
                            reason,
                            mission_start,
                            Some(checkpoint.index),
                            false,
                        )?),
                        false,
                    ));
                }

                coordinator
                    .record_checkpoint_success(checkpoint.index)
                    .map_err(Self::mission_error)?;
                self.observer
                    .record_event(&ObserverEvent::MissionCheckpointProgress {
                        mission_id: mission_id.to_string(),
                        checkpoint_index: checkpoint.index,
                        status: "completed".to_string(),
                        duration: checkpoint_start.elapsed(),
                    });
                Ok((plan, None, true))
            }
            Err(error) => {
                let checkpoint_elapsed_ms =
                    u64::try_from(checkpoint_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                if let Err(reason) =
                    coordinator.record_checkpoint_accounting(checkpoint_elapsed_ms, 0)
                {
                    return Ok((
                        plan,
                        Some(self.terminated_mission_outcome(
                            coordinator,
                            mission_id,
                            reason,
                            mission_start,
                            Some(checkpoint.index),
                            false,
                        )?),
                        false,
                    ));
                }

                let reason_text = error.to_string();
                self.observer.record_event(&ObserverEvent::Error {
                    component: "mission".to_string(),
                    message: Self::sanitize_observer_error(
                        &reason_text,
                        &[("X-Mission-Context".to_string(), "checkpoint".to_string())],
                        &reason_text,
                    ),
                });
                let recoverable = coordinator.should_replan(&reason_text);
                self.observer
                    .record_event(&ObserverEvent::MissionCheckpointProgress {
                        mission_id: mission_id.to_string(),
                        checkpoint_index: checkpoint.index,
                        status: "failed".to_string(),
                        duration: checkpoint_start.elapsed(),
                    });
                coordinator
                    .record_checkpoint_failure(checkpoint.index, reason_text.clone(), recoverable)
                    .map_err(Self::mission_error)?;

                if recoverable {
                    coordinator
                        .transition(MissionState::Replanning)
                        .map_err(Self::mission_error)?;
                    self.observer
                        .record_event(&ObserverEvent::MissionCheckpointProgress {
                            mission_id: mission_id.to_string(),
                            checkpoint_index: checkpoint.index,
                            status: "replanning".to_string(),
                            duration: std::time::Duration::ZERO,
                        });
                    plan = self.replan_after_failure(&plan, checkpoint.index, &reason_text);
                    coordinator
                        .transition(MissionState::Planned)
                        .map_err(Self::mission_error)?;
                    coordinator
                        .transition(MissionState::Active)
                        .map_err(Self::mission_error)?;
                    return Ok((plan, None, false));
                }

                Ok((
                    plan,
                    Some(self.terminated_mission_outcome(
                        coordinator,
                        mission_id,
                        Self::mission_termination_reason_from_error(&reason_text),
                        mission_start,
                        Some(checkpoint.index),
                        false,
                    )?),
                    false,
                ))
            }
        }
    }

    pub async fn run_mission(
        &mut self,
        objective: &str,
        resume_from: Option<u32>,
    ) -> Result<MissionOutcome> {
        if objective.trim().is_empty() {
            anyhow::bail!("mission objective must not be empty");
        }

        let mission_id = Self::mission_id();
        if !self.mission_config.enabled {
            if resume_from.is_some() {
                self.observer
                    .record_event(&ObserverEvent::MissionTerminated {
                        mission_id: mission_id.clone(),
                        checkpoint_index: resume_from,
                        termination_reason: "mission_disabled_rollback".to_string(),
                        duration: std::time::Duration::ZERO,
                        rollback: true,
                    });
            }
            let _ = self.turn(objective).await?;
            return Ok(MissionOutcome {
                mission_id,
                state: MissionState::Completed,
                termination: None,
                checkpoints_completed: 0,
                resume_metadata: MissionResumeMetadata::default(),
            });
        }

        self.observer.record_event(&ObserverEvent::MissionStarted {
            mission_id: mission_id.clone(),
            checkpoint_count: u32::try_from(
                MissionCoordinator::plan_for_objective(objective)
                    .checkpoints
                    .len(),
            )
            .unwrap_or(u32::MAX),
            resume_from,
        });

        self.begin_active_mission_budget(&mission_id)?;
        let coordinator = match self.build_mission_coordinator() {
            Ok(coordinator) => coordinator,
            Err(error) => {
                self.end_active_mission_budget();
                return Err(error);
            }
        };
        let plan = self.build_mission_plan(objective, resume_from);
        let result = self
            .run_mission_plan(&coordinator, &mission_id, Instant::now(), plan)
            .await;
        self.end_active_mission_budget();
        result
    }

    async fn step_with_context(
        &mut self,
        effective_model: &str,
        user_message: &str,
        turn_context: &TurnContext,
    ) -> Result<StepOutcome> {
        // Pre-flight budget check before LLM call
        let reservation = self.enforce_budget_check(effective_model, turn_context)?;

        let response = match self
            .provider
            .chat(
                ChatRequest {
                    messages: &self.tool_dispatcher.to_provider_messages(&self.history),
                    tools: if self.tool_dispatcher.should_send_tool_specs() {
                        Some(&self.tool_specs)
                    } else {
                        None
                    },
                    images: &[],
                },
                effective_model,
                self.temperature,
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                if let (Some(tracker), Some(reservation)) =
                    (&self.cost_tracker, reservation.as_ref())
                {
                    tracker.release_budget_reservation(&reservation.id);
                }
                return Err(error);
            }
        };

        // Record estimated usage after successful LLM call
        self.record_estimated_usage(
            effective_model,
            &response,
            turn_context,
            reservation.as_ref(),
        );

        let (text, calls) = self.tool_dispatcher.parse_response(&response);
        if calls.is_empty() {
            let final_text = self
                .finalize_text_response(user_message, text, response.text, turn_context)
                .await?;
            return Ok(StepOutcome {
                final_text,
                approval_required: None,
                policy_blocked: None,
                tools_called: vec![],
            });
        }

        if self.mission_execution_context
            && calls.iter().any(|call| {
                matches!(
                    evaluate_tool_risk_with_policy_for_origin(
                        &call.name,
                        &self.security_policy,
                        ExecutionOrigin::Mission,
                    ),
                    DispatchAction::ApprovalRequired(_) | DispatchAction::Blocked { .. }
                )
            })
        {
            anyhow::bail!("mission_policy_denied: delegated tool action denied")
        }

        let tools_called: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();
        self.record_tool_response(text, response.text, &calls);
        let gated_results = self.execute_gated_tool_calls(&calls).await;
        let policy_denial = Self::approval_denial_from_results(&calls, &gated_results);
        let (approval_required, policy_blocked) = match policy_denial {
            Some(value)
                if value.get("code").and_then(serde_json::Value::as_str)
                    == Some(crate::security::PLAN_MODE_BLOCKED_CODE) =>
            {
                (None, Some(value))
            }
            other => (other, None),
        };

        let formatted = self.tool_dispatcher.format_results(&gated_results);
        self.history.push(formatted);
        self.trim_history();

        Ok(StepOutcome {
            final_text: None,
            approval_required,
            policy_blocked,
            tools_called,
        })
    }

    pub async fn run_single(&mut self, message: &str) -> Result<String> {
        self.turn(message).await
    }

    pub async fn run_interactive(&mut self) -> Result<()> {
        println!("🦀 Corvus Interactive Mode");
        println!("Type /quit to exit.\n");

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let cli = crate::channels::CliChannel::new();

        let listen_handle = tokio::spawn(async move {
            let _ = crate::channels::Channel::listen(&cli, tx).await;
        });

        while let Some(msg) = rx.recv().await {
            let response = match self.turn(&msg.content).await {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("\nError: {e}\n");
                    continue;
                }
            };
            println!("\n{response}\n");
        }

        listen_handle.abort();
        Ok(())
    }
}

pub async fn run(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
    peripheral_overrides: Vec<String>,
) -> Result<()> {
    // Validate peripheral overrides - currently not supported
    if !peripheral_overrides.is_empty() {
        anyhow::bail!(
            "peripheral overrides are not currently supported; \
             found {} override(s): {:?}",
            peripheral_overrides.len(),
            peripheral_overrides
        );
    }

    let start = Instant::now();

    let mut effective_config = config;
    if let Some(p) = provider_override {
        effective_config.default_provider = Some(p);
    }
    if let Some(m) = model_override {
        effective_config.default_model = Some(m);
    }
    effective_config.default_temperature = temperature;

    let mut agent = Agent::from_config(&effective_config)?;

    let provider_name = effective_config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .to_string();
    let model_name = effective_config
        .default_model
        .as_deref()
        .unwrap_or("anthropic/claude-sonnet-4-20250514")
        .to_string();

    agent.record_agent_start_event(&provider_name, &model_name);

    if let Some(msg) = message {
        let response = agent.run_single(&msg).await?;
        println!("{response}");
    } else {
        agent.run_interactive().await?;
    }

    agent.record_agent_end_event(&provider_name, &model_name, start.elapsed());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_config;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use parking_lot::RwLock;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct MockProvider {
        responses: Mutex<Vec<crate::providers::ChatResponse>>,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            Ok("ok".into())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> Result<crate::providers::ChatResponse> {
            let mut guard = self.responses.lock();
            if guard.is_empty() {
                return Ok(crate::providers::ChatResponse {
                    text: Some("done".into()),
                    tool_calls: vec![],
                });
            }
            Ok(guard.remove(0))
        }
    }

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "echo"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<crate::tools::ToolResult> {
            Ok(crate::tools::ToolResult {
                success: true,
                output: "tool-out".into(),
                error: None,
                structured: None,
            })
        }
    }

    struct ValidationProvider {
        corrections: Mutex<Vec<anyhow::Result<String>>>,
    }

    #[async_trait]
    impl Provider for ValidationProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            let mut guard = self.corrections.lock();
            if guard.is_empty() {
                return Ok("ok".to_string());
            }
            guard.remove(0)
        }
    }

    struct ValidationMemory {
        results: Mutex<Vec<anyhow::Result<crate::memory::MemoryValidationResult>>>,
    }

    #[derive(Default)]
    struct RecordingObserver {
        events: RwLock<Vec<ObserverEvent>>,
    }

    impl RecordingObserver {
        fn snapshot(&self) -> Vec<ObserverEvent> {
            self.events.read().clone()
        }
    }

    impl Observer for RecordingObserver {
        fn record_event(&self, event: &ObserverEvent) {
            self.events.write().push(event.clone());
        }

        fn record_metric(&self, _metric: &crate::observability::ObserverMetric) {}

        fn name(&self) -> &str {
            "recording"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[async_trait]
    impl Memory for ValidationMemory {
        fn name(&self) -> &str {
            "validation-memory"
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
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<crate::memory::MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
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

        async fn validate_response(
            &self,
            _user_query: &str,
            _response: &str,
            _session_id: Option<&str>,
        ) -> anyhow::Result<crate::memory::MemoryValidationResult> {
            let mut guard = self.results.lock();
            if guard.is_empty() {
                return Ok(crate::memory::MemoryValidationResult::default());
            }
            guard.remove(0)
        }
    }

    fn build_validation_agent(provider: Box<dyn Provider>, memory: Arc<dyn Memory>) -> Agent {
        let observer: Arc<dyn Observer> = Arc::from(crate::observability::NoopObserver {});
        Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(memory)
            .observer(observer)
            .tool_dispatcher(Box::new(XmlToolDispatcher))
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn turn_without_tools_returns_text() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("hello".into()),
                tool_calls: vec![],
            }]),
        });

        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::memory::create_memory(&memory_cfg, std::path::Path::new("/tmp"), None).unwrap(),
        );

        let observer: Arc<dyn Observer> = Arc::from(crate::observability::NoopObserver {});
        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer)
            .tool_dispatcher(Box::new(XmlToolDispatcher))
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .build()
            .unwrap();

        let response = agent.turn("hi").await.unwrap();
        assert_eq!(response, "hello");
    }

    #[tokio::test]
    async fn code_profile_agent_uses_bootstrap_components_for_basic_turn() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.agent.profile = "full".into();

        let bootstrap =
            bootstrap::BootstrapContext::from_config_with_profile(&config, "code").unwrap();
        let tool_names: HashSet<&str> = bootstrap.tools.iter().map(|tool| tool.name()).collect();

        assert!(tool_names.contains("shell"));
        assert!(tool_names.contains("git_operations"));
        assert!(!tool_names.contains("schedule"));

        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("code-ready".into()),
                tool_calls: vec![],
            }]),
        });

        let mut agent = Agent::from_bootstrap_with_provider(&config, bootstrap, provider).unwrap();

        let response = agent.turn("review this patch").await.unwrap();
        assert_eq!(response, "code-ready");
    }

    #[tokio::test]
    async fn turn_with_native_dispatcher_handles_tool_results_variant() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![
                crate::providers::ChatResponse {
                    text: Some(String::new()),
                    tool_calls: vec![crate::providers::ToolCall {
                        id: "tc1".into(),
                        name: "echo".into(),
                        arguments: "{}".into(),
                    }],
                },
                crate::providers::ChatResponse {
                    text: Some("done".into()),
                    tool_calls: vec![],
                },
            ]),
        });

        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::memory::create_memory(&memory_cfg, std::path::Path::new("/tmp"), None).unwrap(),
        );

        let observer: Arc<dyn Observer> = Arc::from(crate::observability::NoopObserver {});
        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer)
            .tool_dispatcher(Box::new(NativeToolDispatcher))
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .build()
            .unwrap();

        let response = agent.turn("hi").await.unwrap();
        assert_eq!(response, "done");
        assert!(agent
            .history()
            .iter()
            .any(|msg| matches!(msg, ConversationMessage::ToolResults(_))));
    }

    #[tokio::test]
    async fn strict_validation_returns_corrected_text_when_second_pass_is_valid() {
        let provider = Box::new(ValidationProvider {
            corrections: Mutex::new(vec![Ok("corrected answer".to_string())]),
        });
        let memory: Arc<dyn Memory> = Arc::new(ValidationMemory {
            results: Mutex::new(vec![
                Ok(crate::memory::MemoryValidationResult {
                    valid: false,
                    violations: vec!["missing required domain term 'foo'".to_string()],
                }),
                Ok(crate::memory::MemoryValidationResult {
                    valid: true,
                    violations: Vec::new(),
                }),
            ]),
        });

        let agent = build_validation_agent(provider, memory);
        let output = agent
            .enforce_strict_memory_validation("what is foo", "draft".to_string())
            .await;

        assert_eq!(output, "corrected answer");
    }

    #[tokio::test]
    async fn strict_validation_returns_violation_text_when_correction_call_fails() {
        let provider = Box::new(ValidationProvider {
            corrections: Mutex::new(vec![Err(anyhow::anyhow!("provider unavailable"))]),
        });
        let memory: Arc<dyn Memory> = Arc::new(ValidationMemory {
            results: Mutex::new(vec![Ok(crate::memory::MemoryValidationResult {
                valid: false,
                violations: vec!["missing required domain term 'foo'".to_string()],
            })]),
        });

        let agent = build_validation_agent(provider, memory);
        let output = agent
            .enforce_strict_memory_validation("what is foo", "draft".to_string())
            .await;

        assert_eq!(
            output,
            "I cannot provide a validated answer because strict ontology checks failed:\n- missing required domain term 'foo'"
        );
    }

    #[tokio::test]
    async fn strict_validation_returns_second_pass_violations_when_still_invalid() {
        let provider = Box::new(ValidationProvider {
            corrections: Mutex::new(vec![Ok("candidate two".to_string())]),
        });
        let memory: Arc<dyn Memory> = Arc::new(ValidationMemory {
            results: Mutex::new(vec![
                Ok(crate::memory::MemoryValidationResult {
                    valid: false,
                    violations: vec!["missing required domain term 'foo'".to_string()],
                }),
                Ok(crate::memory::MemoryValidationResult {
                    valid: false,
                    violations: vec!["response contains forbidden domain term 'bar'".to_string()],
                }),
            ]),
        });

        let agent = build_validation_agent(provider, memory);
        let output = agent
            .enforce_strict_memory_validation("what is foo", "draft".to_string())
            .await;

        assert_eq!(
            output,
            "I cannot provide a validated answer because strict ontology checks still fail:\n- response contains forbidden domain term 'bar'"
        );
    }

    #[tokio::test]
    async fn strict_validation_returns_ontology_failed_when_initial_validation_errors() {
        let provider = Box::new(ValidationProvider {
            corrections: Mutex::new(Vec::new()),
        });
        let memory: Arc<dyn Memory> = Arc::new(ValidationMemory {
            results: Mutex::new(vec![Err(anyhow::anyhow!("validation backend down"))]),
        });

        let agent = build_validation_agent(provider, memory);
        let output = agent
            .enforce_strict_memory_validation("what is foo", "draft".to_string())
            .await;

        assert_eq!(
            output,
            "I cannot provide a validated answer right now because ontology validation failed."
        );
    }

    #[tokio::test]
    async fn strict_validation_returns_unavailable_when_second_validation_errors() {
        let provider = Box::new(ValidationProvider {
            corrections: Mutex::new(vec![Ok("candidate two".to_string())]),
        });
        let memory: Arc<dyn Memory> = Arc::new(ValidationMemory {
            results: Mutex::new(vec![
                Ok(crate::memory::MemoryValidationResult {
                    valid: false,
                    violations: vec!["missing required domain term 'foo'".to_string()],
                }),
                Err(anyhow::anyhow!("validation backend down")),
            ]),
        });

        let agent = build_validation_agent(provider, memory);
        let output = agent
            .enforce_strict_memory_validation("what is foo", "draft".to_string())
            .await;

        assert_eq!(
            output,
            "I cannot provide a validated answer because ontology checks are unavailable."
        );
    }

    // ── Cost wiring tests ─────────────────────────────────────────

    fn build_agent_with_cost_tracker(
        provider: Box<dyn Provider>,
        cost_enabled: bool,
        daily_limit: f64,
    ) -> (Agent, Option<Arc<crate::cost::CostTracker>>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: cost_enabled,
            daily_limit_usd: daily_limit,
            monthly_limit_usd: 1000.0,
            warn_at_percent: 80,
            allow_override: true,
            ..Default::default()
        };
        let tracker = if cost_enabled {
            Some(Arc::new(
                crate::cost::CostTracker::new(cost_config.clone(), tmp.path()).unwrap(),
            ))
        } else {
            None
        };
        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::memory::create_memory(&memory_cfg, std::path::Path::new("/tmp"), None).unwrap(),
        );
        let observer: Arc<dyn Observer> = Arc::from(crate::observability::NoopObserver {});
        let agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer)
            .tool_dispatcher(Box::new(XmlToolDispatcher))
            .workspace_dir(tmp.path().to_path_buf())
            .cost_tracker(tracker.clone())
            .cost_config(cost_config)
            .build()
            .unwrap();
        (agent, tracker, tmp)
    }

    fn build_agent_with_recording_observer(
        provider: Box<dyn Provider>,
        daily_limit: f64,
    ) -> (
        Agent,
        Arc<crate::cost::CostTracker>,
        Arc<RecordingObserver>,
        TempDir,
    ) {
        let tmp = TempDir::new().unwrap();
        let cost_config = crate::config::CostConfig {
            enabled: true,
            daily_limit_usd: daily_limit,
            monthly_limit_usd: 1000.0,
            warn_at_percent: 80,
            allow_override: true,
            ..Default::default()
        };
        let tracker =
            Arc::new(crate::cost::CostTracker::new(cost_config.clone(), tmp.path()).unwrap());
        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::memory::create_memory(&memory_cfg, std::path::Path::new("/tmp"), None).unwrap(),
        );
        let observer = Arc::new(RecordingObserver::default());
        let agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer.clone() as Arc<dyn Observer>)
            .tool_dispatcher(Box::new(XmlToolDispatcher))
            .workspace_dir(tmp.path().to_path_buf())
            .cost_tracker(Some(tracker.clone()))
            .cost_config(cost_config)
            .build()
            .unwrap();
        (agent, tracker, observer, tmp)
    }

    #[tokio::test]
    async fn cost_tracker_records_usage_after_llm_call() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("hello world response".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 100.0);
        let _ = agent.turn("hi").await.unwrap();

        let tracker = tracker.unwrap();
        let summary = tracker.get_summary().unwrap();
        assert_eq!(summary.request_count, 1, "should have recorded one usage");
        assert!(
            summary.session_cost_usd >= 0.0,
            "cost should be non-negative"
        );
    }

    #[tokio::test]
    async fn cost_tracker_none_when_disabled() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("hello".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, false, 100.0);
        assert!(tracker.is_none());
        // Should still work normally
        let response = agent.turn("hi").await.unwrap();
        assert_eq!(response, "hello");
    }

    #[tokio::test]
    async fn budget_exceeded_blocks_llm_call() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![
                crate::providers::ChatResponse {
                    text: Some("first".into()),
                    tool_calls: vec![],
                },
                crate::providers::ChatResponse {
                    text: Some("should not reach".into()),
                    tool_calls: vec![],
                },
            ]),
        });
        // Generous limit so first call succeeds; we inject big usage after
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 1.0);

        // First call succeeds and records usage
        let _ = agent.turn("hi").await.unwrap();

        let t = tracker.as_ref().unwrap();
        let summary = t.get_summary().unwrap();
        assert!(
            summary.request_count > 0,
            "first call should have recorded usage"
        );

        // Manually record a large usage to push over the limit
        let big_usage = crate::cost::TokenUsage::new(
            "anthropic/claude-sonnet-4-20250514",
            100_000,
            50_000,
            3.0,
            15.0,
        );
        t.record_usage(big_usage).unwrap();

        // Second call should be blocked by budget check
        let result = agent.turn("hi again").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Budget exceeded"),
            "expected budget exceeded error, got: {err_msg}"
        );
        assert!(
            !err_msg.contains("action budget exhausted"),
            "token-spend denial should not be labeled as action-rate exhaustion: {err_msg}"
        );
    }

    #[tokio::test]
    async fn mission_scope_blocks_metered_call_independently_from_session_budget() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("should not reach".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 100.0);
        let tracker = tracker.unwrap();
        agent.mission_config.max_estimated_cost_cents = 100;
        agent.begin_active_mission_budget("mission-a").unwrap();

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.05;
        tracker.record_usage(usage).unwrap();

        let result = agent.turn("mission checkpoint").await;
        agent.end_active_mission_budget();

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Budget exceeded"));
        assert!(error.contains("Mission limit"));
    }

    #[tokio::test]
    async fn mission_budget_scope_survives_session_reset() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("mission still tracked".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 100.0);
        let tracker = tracker.unwrap();
        agent.mission_config.max_estimated_cost_cents = 100;
        agent.begin_active_mission_budget("mission-a").unwrap();

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 0.75;
        tracker.record_usage(usage).unwrap();

        tracker
            .reset(
                crate::cost::CostResetRequest {
                    scope: crate::cost::CostResetScope::Session,
                    actor: "tester".into(),
                    reason: Some("session reset".into()),
                },
                chrono::Utc::now(),
            )
            .unwrap();

        let scope = agent.current_mission_budget_scope().unwrap().unwrap();
        assert!((scope.current_usd - 0.75).abs() < 0.0001);
        assert!((scope.limit_usd - 1.0).abs() < 0.0001);
    }

    #[tokio::test]
    async fn token_budget_denial_is_reported_separately_from_action_rate_governance() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("should not reach".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 1.0);
        let tracker = tracker.unwrap();

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        let result = agent.turn("hi").await;
        assert!(result.is_err());

        let error = result.unwrap_err().to_string();
        assert!(error.contains("Budget exceeded"));
        assert!(!error.contains("action budget exhausted"));
    }

    #[tokio::test]
    async fn next_request_override_allows_one_blocked_turn() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![
                crate::providers::ChatResponse {
                    text: Some("override succeeded".into()),
                    tool_calls: vec![],
                },
                crate::providers::ChatResponse {
                    text: Some("should block again".into()),
                    tool_calls: vec![],
                },
            ]),
        });
        let (mut agent, tracker, _tmp) = build_agent_with_cost_tracker(provider, true, 1.0);
        let tracker = tracker.unwrap();
        let service = crate::cost::CostService::new(tracker.clone());
        let now = chrono::Utc::now();

        let mut usage = crate::cost::TokenUsage::new(
            "anthropic/claude-sonnet-4-20250514",
            1_000,
            500,
            0.0,
            0.0,
        );
        usage.cost_usd = 1.1;
        usage.timestamp = now;
        tracker.record_usage(usage).unwrap();

        service
            .apply_override(
                crate::cost::CostOverrideRequest {
                    actor: "operator".to_string(),
                    scope: crate::cost::CostOverrideScope::NextRequest,
                    reason: Some("allow one follow-up".to_string()),
                    expires_at: Some(now + chrono::Duration::minutes(5)),
                },
                now,
            )
            .unwrap();

        let response = agent.turn("hi with override").await.unwrap();
        assert_eq!(response, "override succeeded");

        let result = agent.turn("hi after override").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn warning_threshold_emits_budget_warning_event() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("warning".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, observer, _tmp) =
            build_agent_with_recording_observer(provider, 1.0);

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 0.81;
        tracker.record_usage(usage).unwrap();

        let response = agent
            .turn_with_context("hi", TurnContext::with_session("webhook-123"))
            .await
            .unwrap();
        assert_eq!(response.final_text.as_deref(), Some("warning"));

        let events = observer.snapshot();
        assert!(events
            .iter()
            .any(|event| matches!(event, ObserverEvent::BudgetWarning(_))));
        assert!(events.iter().any(|event| matches!(event,
            ObserverEvent::BudgetWarning(event) if event.session_id == "webhook-123"
        )));
    }

    #[tokio::test]
    async fn blocked_turn_emits_budget_exceeded_event() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::providers::ChatResponse {
                text: Some("blocked".into()),
                tool_calls: vec![],
            }]),
        });
        let (mut agent, tracker, observer, _tmp) =
            build_agent_with_recording_observer(provider, 1.0);

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        let result = agent
            .turn_with_context("hi", TurnContext::with_session("webhook-456"))
            .await;
        assert!(result.is_err());

        let events = observer.snapshot();
        assert!(events
            .iter()
            .any(|event| matches!(event, ObserverEvent::BudgetExceeded(_))));
        assert!(events.iter().any(|event| matches!(event,
            ObserverEvent::BudgetExceeded(event) if event.session_id == "webhook-456"
        )));
    }

    #[test]
    fn local_override_emits_budget_override_event() {
        let provider = Box::new(MockProvider {
            responses: Mutex::new(Vec::new()),
        });
        let (agent, tracker, observer, _tmp) = build_agent_with_recording_observer(provider, 1.0);

        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        agent
            .apply_next_request_budget_override(
                "paired-admin-token",
                Some("token=super-secret".into()),
            )
            .unwrap();

        assert!(observer
            .snapshot()
            .into_iter()
            .any(|event| matches!(event, ObserverEvent::BudgetOverride(_))));
    }

    #[tokio::test]
    async fn bootstrap_creates_cost_tracker_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.cost.enabled = true;
        config.cost.daily_limit_usd = 50.0;

        let ctx = crate::bootstrap::BootstrapContext::from_config(&config).unwrap();
        assert!(
            ctx.cost_tracker.is_some(),
            "cost_tracker should be Some when cost.enabled=true"
        );
    }

    #[tokio::test]
    async fn bootstrap_no_cost_tracker_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);
        // cost.enabled defaults to false

        let ctx = crate::bootstrap::BootstrapContext::from_config(&config).unwrap();
        assert!(
            ctx.cost_tracker.is_none(),
            "cost_tracker should be None when cost.enabled=false"
        );
    }

    #[test]
    fn full_runtime_starts_without_manifest() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.default_provider = Some("anthropic".into());

        let agent = Agent::from_config(&config).unwrap();

        // Canonicalize the backend key for comparison since config may use aliases
        let expected_backend = corvus_memory::resolve_memory_backend_key(&config.memory.backend)
            .unwrap_or(config.memory.backend.as_str());
        assert_eq!(agent.memory.name(), expected_backend);
        assert!(!agent.tools.is_empty());
    }

    fn build_classification_test_agent(
        classification_config: crate::config::QueryClassificationConfig,
        available_hints: Vec<String>,
    ) -> Agent {
        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::memory::create_memory(&memory_cfg, std::path::Path::new("/tmp"), None).unwrap(),
        );

        Agent::builder()
            .provider(Box::new(MockProvider {
                responses: Mutex::new(Vec::new()),
            }))
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(Arc::new(crate::observability::NoopObserver))
            .tool_dispatcher(Box::new(XmlToolDispatcher))
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .model_name("gpt-4o-mini".to_string())
            .classification_config(classification_config)
            .available_hints(available_hints)
            .build()
            .unwrap()
    }

    #[test]
    fn classify_model_uses_default_model_when_classification_disabled() {
        let agent = build_classification_test_agent(
            crate::config::QueryClassificationConfig {
                enabled: false,
                rules: vec![crate::config::ClassificationRule {
                    hint: "reasoning".into(),
                    keywords: vec!["debug".into()],
                    ..Default::default()
                }],
            },
            vec!["reasoning".into()],
        );

        assert_eq!(agent.classify_model("help me debug this"), "gpt-4o-mini");
    }

    #[test]
    fn classify_model_uses_default_model_when_no_rule_matches() {
        let agent = build_classification_test_agent(
            crate::config::QueryClassificationConfig {
                enabled: true,
                rules: vec![crate::config::ClassificationRule {
                    hint: "reasoning".into(),
                    keywords: vec!["debug".into()],
                    ..Default::default()
                }],
            },
            vec!["reasoning".into()],
        );

        assert_eq!(agent.classify_model("what is the weather?"), "gpt-4o-mini");
    }
}
