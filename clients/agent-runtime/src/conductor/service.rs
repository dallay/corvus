use crate::conductor::events;
use crate::conductor::performers::{PerformerContext, PerformerPool};
use crate::conductor::planner::Planner;
use crate::conductor::task_store::{StepRecord, TaskRecord, TaskStore};
use crate::conductor::{
    ConductorEventEnvelope, PlannedStepForExecution, RiskLevel, StepId, StepStatus, TaskDomain,
    TaskId, TaskRequest, TaskStatus,
};
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

const DISPATCH_DOMAINS: [TaskDomain; 4] = [
    TaskDomain::Coding,
    TaskDomain::Research,
    TaskDomain::Browser,
    TaskDomain::System,
];

#[derive(Debug, Clone)]
pub struct ReadyStep {
    pub task_id: TaskId,
    pub step_id: StepId,
    pub domain: TaskDomain,
    pub status: StepStatus,
    pub enqueued_epoch_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfigView {
    pub global_max: usize,
    pub coding_max: usize,
    pub research_max: usize,
    pub browser_max: usize,
    pub system_max: usize,
    pub intake_capacity: usize,
    pub hard_intake_capacity: usize,
}

impl Default for SchedulerConfigView {
    fn default() -> Self {
        Self {
            global_max: 10,
            coding_max: 3,
            research_max: 5,
            browser_max: 2,
            system_max: 2,
            intake_capacity: 1024,
            hard_intake_capacity: 4096,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    Queued,
    QueuedWithBackpressure,
    Saturated,
}

#[derive(Debug, Clone)]
pub struct TickDispatch {
    pub dispatched: Vec<ReadyStep>,
}

pub struct ConductorServiceCore {
    config: SchedulerConfigView,
    queues: HashMap<TaskDomain, VecDeque<ReadyStep>>,
    domain_cursor: usize,
}

impl ConductorServiceCore {
    pub fn new(config: SchedulerConfigView) -> Self {
        let mut queues = HashMap::new();
        queues.insert(TaskDomain::Coding, VecDeque::new());
        queues.insert(TaskDomain::Research, VecDeque::new());
        queues.insert(TaskDomain::Browser, VecDeque::new());
        queues.insert(TaskDomain::System, VecDeque::new());

        Self {
            config,
            queues,
            domain_cursor: 0,
        }
    }

    pub fn submit(&mut self, step: ReadyStep) -> SubmitOutcome {
        if self.queue_depth() >= self.config.hard_intake_capacity {
            return SubmitOutcome::Saturated;
        }
        let under_pressure = self.queue_depth() >= self.config.intake_capacity;
        self.enqueue(step);
        if under_pressure {
            SubmitOutcome::QueuedWithBackpressure
        } else {
            SubmitOutcome::Queued
        }
    }

    pub fn enqueue(&mut self, step: ReadyStep) {
        if let Some(queue) = self.queues.get_mut(&step.domain) {
            queue.push_back(step);
        }
    }

    pub fn queue_depth(&self) -> usize {
        self.queues.values().map(VecDeque::len).sum()
    }

    pub fn mini_tick(&mut self, now_epoch_ms: u64) -> TickDispatch {
        self.run_tick(now_epoch_ms, false)
    }

    pub fn full_tick(&mut self, now_epoch_ms: u64) -> TickDispatch {
        self.run_tick(now_epoch_ms, true)
    }

    fn run_tick(&mut self, now_epoch_ms: u64, _with_notify: bool) -> TickDispatch {
        self.reconcile();
        let dispatched = self.schedule_and_dispatch(now_epoch_ms);
        self.notify();
        TickDispatch { dispatched }
    }

    fn reconcile(&mut self) {}

    fn notify(&self) {}

    fn schedule_and_dispatch(&mut self, now_epoch_ms: u64) -> Vec<ReadyStep> {
        let mut dispatched = Vec::new();
        let mut running_global = 0usize;
        let mut running_domain: HashMap<TaskDomain, usize> = HashMap::new();

        loop {
            if running_global >= self.config.global_max {
                break;
            }

            let mut progressed = false;
            for offset in 0..DISPATCH_DOMAINS.len() {
                if running_global >= self.config.global_max {
                    break;
                }

                let index = (self.domain_cursor + offset) % DISPATCH_DOMAINS.len();
                let domain = DISPATCH_DOMAINS[index];

                let domain_limit = self.domain_limit(domain);
                let domain_running = *running_domain.get(&domain).unwrap_or(&0);
                if domain_running >= domain_limit {
                    continue;
                }

                let ready = self.pop_eligible(domain, now_epoch_ms);
                if let Some(mut step) = ready {
                    if matches!(step.status, StepStatus::RetryQueued { .. }) {
                        step.status = StepStatus::Queued;
                    }
                    running_global += 1;
                    running_domain.insert(domain, domain_running + 1);
                    dispatched.push(step);
                    self.domain_cursor = (index + 1) % DISPATCH_DOMAINS.len();
                    progressed = true;
                    break;
                }
            }

            if !progressed {
                break;
            }
        }

        dispatched
    }

    fn pop_eligible(&mut self, domain: TaskDomain, now_epoch_ms: u64) -> Option<ReadyStep> {
        let queue = self.queues.get_mut(&domain)?;
        let mut pending = VecDeque::new();
        let mut selected = None;

        while let Some(step) = queue.pop_front() {
            if selected.is_none() && is_eligible(&step.status, now_epoch_ms) {
                selected = Some(step);
            } else {
                pending.push_back(step);
            }
        }

        *queue = pending;
        selected
    }

    fn domain_limit(&self, domain: TaskDomain) -> usize {
        match domain {
            TaskDomain::Coding => self.config.coding_max,
            TaskDomain::Research => self.config.research_max,
            TaskDomain::Browser => self.config.browser_max,
            TaskDomain::System => self.config.system_max,
            TaskDomain::Composite => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTerminal {
    Completed,
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct RuntimeReport {
    pub task_id: TaskId,
    pub terminal: RuntimeTerminal,
}

pub trait RuntimeUpdateSink: Send + Sync {
    fn planning_failed(&self, _task_id: &TaskId, _error: &str) {}
    fn step_progress(
        &self,
        _task_id: &TaskId,
        _step_id: &StepId,
        _status: &StepStatus,
        _remaining_steps: usize,
    ) {
    }
}

#[derive(Default)]
pub struct NoopRuntimeUpdateSink;

impl RuntimeUpdateSink for NoopRuntimeUpdateSink {}

pub struct ConductorRuntime {
    planner: Arc<Planner>,
    store: Arc<TaskStore>,
    pool: Arc<PerformerPool>,
    context: Arc<PerformerContext>,
    step_timeout: Duration,
    sink: Arc<dyn RuntimeUpdateSink>,
}

impl ConductorRuntime {
    pub fn new(
        planner: Arc<Planner>,
        store: Arc<TaskStore>,
        pool: Arc<PerformerPool>,
        context: Arc<PerformerContext>,
        step_timeout: Duration,
        sink: Arc<dyn RuntimeUpdateSink>,
    ) -> Self {
        Self {
            planner,
            store,
            pool,
            context,
            step_timeout,
            sink,
        }
    }

    pub async fn submit_and_run(&self, request: TaskRequest) -> Result<RuntimeReport> {
        let task_id = TaskId::new(format!("task_{}", uuid::Uuid::new_v4().simple()))?;

        events::publish(&ConductorEventEnvelope::TaskAccepted {
            task_id: task_id.clone(),
        });
        events::publish(&ConductorEventEnvelope::TaskStateChanged {
            task_id: task_id.clone(),
            status: TaskStatus::Received,
        });
        events::publish(&ConductorEventEnvelope::TaskStateChanged {
            task_id: task_id.clone(),
            status: TaskStatus::Planning,
        });

        let plan = match self.planner.plan(&request).await {
            Ok(plan) => plan,
            Err(error) => {
                let message = error.to_string();
                self.sink.planning_failed(&task_id, &message);
                events::publish(&ConductorEventEnvelope::TaskStateChanged {
                    task_id: task_id.clone(),
                    status: TaskStatus::Failed {
                        error: message.clone(),
                    },
                });
                return Ok(RuntimeReport {
                    task_id,
                    terminal: RuntimeTerminal::Failed { error: message },
                });
            }
        };

        let mut steps = HashMap::new();
        for planned in &plan.steps {
            steps.insert(
                planned.id.clone(),
                StepRecord {
                    id: planned.id.clone(),
                    domain: planned.domain,
                    depends_on: planned.depends_on.clone(),
                    status: StepStatus::Queued,
                },
            );
        }

        self.store.insert_task(TaskRecord {
            id: task_id.clone(),
            description: request.description,
            status: TaskStatus::Active,
            steps,
        })?;

        events::publish(&ConductorEventEnvelope::TaskStateChanged {
            task_id: task_id.clone(),
            status: TaskStatus::Active,
        });

        let mut completed_steps = 0usize;
        for planned in &plan.steps {
            if !self
                .store
                .dependencies_completed(&task_id, &planned.depends_on)?
            {
                continue;
            }

            self.store
                .transition_step(&task_id, &planned.id, StepStatus::Running)?;
            events::publish(&ConductorEventEnvelope::StepStateChanged {
                task_id: task_id.clone(),
                step_id: planned.id.clone(),
                status: StepStatus::Running,
            });

            let step = PlannedStepForExecution {
                task_id: task_id.clone(),
                step_id: planned.id.clone(),
                domain: planned.domain,
                description: planned.description.clone(),
                command: planned.description.clone(),
                risk: RiskLevel::Low,
            };

            let timeout_err = format!("step timed out: {}", planned.id.as_str());
            let next = match timeout(
                self.step_timeout,
                tokio::spawn({
                    let pool = Arc::clone(&self.pool);
                    let context = Arc::clone(&self.context);
                    let step = step.clone();
                    async move { pool.execute_step(&step, context.as_ref()).await }
                }),
            )
            .await
            {
                Err(_) => StepStatus::Failed { error: timeout_err },
                Ok(join_result) => match join_result {
                    Err(join_error) => StepStatus::Failed {
                        error: format!("performer panic: {join_error}"),
                    },
                    Ok(Err(error)) => StepStatus::Failed {
                        error: error.to_string(),
                    },
                    Ok(Ok(status)) => status,
                },
            };

            self.store
                .transition_step(&task_id, &planned.id, next.clone())?;
            let remaining = plan.steps.len().saturating_sub(completed_steps + 1);
            self.sink
                .step_progress(&task_id, &planned.id, &next, remaining);
            events::publish(&ConductorEventEnvelope::StepStateChanged {
                task_id: task_id.clone(),
                step_id: planned.id.clone(),
                status: next.clone(),
            });

            match next {
                StepStatus::Completed => {
                    completed_steps += 1;
                }
                StepStatus::Failed { ref error } => {
                    self.store
                        .propagate_dependency_failure(&task_id, &planned.id, error)?;
                    events::publish(&ConductorEventEnvelope::TaskStateChanged {
                        task_id: task_id.clone(),
                        status: TaskStatus::Failed {
                            error: error.clone(),
                        },
                    });
                    return Ok(RuntimeReport {
                        task_id,
                        terminal: RuntimeTerminal::Failed {
                            error: error.clone(),
                        },
                    });
                }
                StepStatus::Cancelled { ref reason } => {
                    self.store.set_task_status(
                        &task_id,
                        TaskStatus::Cancelled {
                            reason: reason.clone(),
                        },
                    )?;
                    events::publish(&ConductorEventEnvelope::TaskStateChanged {
                        task_id: task_id.clone(),
                        status: TaskStatus::Cancelled {
                            reason: reason.clone(),
                        },
                    });
                    return Ok(RuntimeReport {
                        task_id,
                        terminal: RuntimeTerminal::Failed {
                            error: reason.clone(),
                        },
                    });
                }
                _ => {
                    let error = "step did not reach terminal state".to_string();
                    self.store.set_task_status(
                        &task_id,
                        TaskStatus::Failed {
                            error: error.clone(),
                        },
                    )?;
                    events::publish(&ConductorEventEnvelope::TaskStateChanged {
                        task_id: task_id.clone(),
                        status: TaskStatus::Failed {
                            error: error.clone(),
                        },
                    });
                    return Ok(RuntimeReport {
                        task_id,
                        terminal: RuntimeTerminal::Failed { error },
                    });
                }
            }
        }

        self.store
            .set_task_status(&task_id, TaskStatus::Completed)?;
        events::publish(&ConductorEventEnvelope::TaskStateChanged {
            task_id: task_id.clone(),
            status: TaskStatus::Completed,
        });
        Ok(RuntimeReport {
            task_id,
            terminal: RuntimeTerminal::Completed,
        })
    }
}

fn is_eligible(status: &StepStatus, now_epoch_ms: u64) -> bool {
    match status {
        StepStatus::Queued | StepStatus::Scheduled => true,
        StepStatus::RetryQueued {
            retry_after_epoch_ms,
            ..
        } => *retry_after_epoch_ms <= now_epoch_ms,
        _ => false,
    }
}
