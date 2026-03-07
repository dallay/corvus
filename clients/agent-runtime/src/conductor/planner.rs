use crate::conductor::classifier::{Confidence, TaskClassifier};
use crate::conductor::{StepId, TaskDomain, TaskRequest};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct PlannedStep {
    pub id: StepId,
    pub domain: TaskDomain,
    pub description: String,
    pub depends_on: Vec<StepId>,
}

#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub steps: Vec<PlannedStep>,
}

#[async_trait]
pub trait PlanModel: Send + Sync {
    async fn decompose(&self, request: &TaskRequest, prompt: &str) -> Result<TaskPlan>;
}

pub struct NoopPlanModel;

#[async_trait]
impl PlanModel for NoopPlanModel {
    async fn decompose(&self, _request: &TaskRequest, _prompt: &str) -> Result<TaskPlan> {
        anyhow::bail!("noop plan model should not be used in slow-path planning");
    }
}

#[derive(Debug, Clone)]
pub struct PlannerConfigView {
    pub max_planning_time_ms: u64,
    pub fast_path_budget_ms: u64,
    pub prompt_path: Option<PathBuf>,
}

impl Default for PlannerConfigView {
    fn default() -> Self {
        Self {
            max_planning_time_ms: 30_000,
            fast_path_budget_ms: 10,
            prompt_path: None,
        }
    }
}

pub trait PlanValidator: Send + Sync {
    fn validate(&self, plan: &TaskPlan) -> Result<()>;
}

pub struct DefaultPlanValidator;

impl PlanValidator for DefaultPlanValidator {
    fn validate(&self, plan: &TaskPlan) -> Result<()> {
        validate_plan(plan)
    }
}

pub trait PromptSource: Send + Sync {
    fn load_prompt(&self) -> Result<String>;
}

pub struct FilePromptSource {
    prompt_path: Option<PathBuf>,
}

impl FilePromptSource {
    pub fn new(prompt_path: Option<PathBuf>) -> Self {
        Self { prompt_path }
    }
}

pub struct WatchedPromptSource {
    watcher: crate::conductor::prompt_watcher::PromptHotReload,
}

impl WatchedPromptSource {
    pub fn new(prompt_path: &Path) -> Result<Self> {
        Ok(Self {
            watcher: crate::conductor::prompt_watcher::PromptHotReload::new(prompt_path)?,
        })
    }
}

impl PromptSource for WatchedPromptSource {
    fn load_prompt(&self) -> Result<String> {
        Ok(self.watcher.latest_prompt())
    }
}

impl PromptSource for FilePromptSource {
    fn load_prompt(&self) -> Result<String> {
        match &self.prompt_path {
            Some(path) => fs::read_to_string(path)
                .map_err(anyhow::Error::from)
                .map(|contents| contents.trim().to_string()),
            None => Ok(String::new()),
        }
    }
}

pub struct Planner {
    config: PlannerConfigView,
    classifier: Box<dyn TaskClassifier>,
    model: Box<dyn PlanModel>,
    validator: Box<dyn PlanValidator>,
    prompt_source: Box<dyn PromptSource>,
}

impl Planner {
    pub fn new(
        config: PlannerConfigView,
        classifier: Box<dyn TaskClassifier>,
        model: Box<dyn PlanModel>,
    ) -> Self {
        let prompt_source: Box<dyn PromptSource> = if let Some(path) = config.prompt_path.clone() {
            match WatchedPromptSource::new(&path) {
                Ok(source) => Box::new(source),
                Err(_) => Box::new(FilePromptSource::new(Some(path))),
            }
        } else {
            Box::new(FilePromptSource::new(None))
        };
        Self {
            config,
            classifier,
            model,
            validator: Box::new(DefaultPlanValidator),
            prompt_source,
        }
    }

    pub fn with_components(
        config: PlannerConfigView,
        classifier: Box<dyn TaskClassifier>,
        model: Box<dyn PlanModel>,
        validator: Box<dyn PlanValidator>,
        prompt_source: Box<dyn PromptSource>,
    ) -> Self {
        Self {
            config,
            classifier,
            model,
            validator,
            prompt_source,
        }
    }

    pub async fn classify(&self, request: &TaskRequest) -> Result<(TaskDomain, Confidence)> {
        let result = self.classifier.classify(&request.description).await?;
        Ok((result.domain, result.confidence))
    }

    pub async fn plan(&self, request: &TaskRequest) -> Result<TaskPlan> {
        let fast_path_started = Instant::now();
        let classification = self.classifier.classify(&request.description).await?;
        if classification.confidence == Confidence::High
            && classification.domain != TaskDomain::Composite
        {
            let plan = TaskPlan {
                steps: vec![PlannedStep {
                    id: StepId::new("step_fastpath_1").expect("hardcoded id should be valid"),
                    domain: classification.domain,
                    description: request.description.clone(),
                    depends_on: Vec::new(),
                }],
            };
            if fast_path_started.elapsed()
                > Duration::from_millis(self.config.fast_path_budget_ms.max(1))
            {
                anyhow::bail!("fast-path planning exceeded budget");
            }
            return Ok(plan);
        }

        let prompt = self.prompt_source.load_prompt()?;
        let model_future = self.model.decompose(request, &prompt);
        let plan = timeout(
            Duration::from_millis(self.config.max_planning_time_ms),
            model_future,
        )
        .await
        .map_err(|_| anyhow::anyhow!("planning timed out"))??;

        self.validator
            .validate(&plan)
            .map_err(|error| anyhow::anyhow!("malformed plan: {error}"))?;
        Ok(plan)
    }
}

pub fn validate_plan(plan: &TaskPlan) -> Result<()> {
    if plan.steps.is_empty() {
        anyhow::bail!("plan must contain at least one step");
    }

    let mut ids = HashSet::new();
    for step in &plan.steps {
        if step.domain == TaskDomain::Composite {
            anyhow::bail!("composite domain cannot be dispatched as a step");
        }
        if !ids.insert(step.id.as_str().to_string()) {
            anyhow::bail!("duplicate step id: {}", step.id.as_str());
        }
    }

    for step in &plan.steps {
        for dep in &step.depends_on {
            if !ids.contains(dep.as_str()) {
                anyhow::bail!("unknown dependency: {}", dep.as_str());
            }
        }
    }

    detect_cycle(plan)?;
    Ok(())
}

fn detect_cycle(plan: &TaskPlan) -> Result<()> {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for step in &plan.steps {
        adjacency.insert(
            step.id.as_str().to_string(),
            step.depends_on
                .iter()
                .map(|dep| dep.as_str().to_string())
                .collect(),
        );
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in adjacency.keys() {
        if has_cycle(node, &adjacency, &mut visiting, &mut visited) {
            anyhow::bail!("Dependency cycle detected");
        }
    }
    Ok(())
}

fn has_cycle(
    node: &str,
    adjacency: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node.to_string()) {
        return true;
    }

    if let Some(neighbors) = adjacency.get(node) {
        for neighbor in neighbors {
            if has_cycle(neighbor, adjacency, visiting, visited) {
                return true;
            }
        }
    }

    visiting.remove(node);
    visited.insert(node.to_string());
    false
}
