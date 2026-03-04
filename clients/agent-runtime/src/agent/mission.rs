use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionState {
    ObjectiveAccepted,
    Planned,
    Active,
    Replanning,
    Completed,
    Terminated,
}

impl MissionState {
    pub fn allows_transition_to(&self, target: &Self) -> bool {
        use MissionState::{Active, Completed, ObjectiveAccepted, Planned, Replanning, Terminated};

        matches!(
            (self, target),
            (ObjectiveAccepted | Replanning, Planned)
                | (
                    ObjectiveAccepted | Planned | Active | Replanning,
                    Terminated
                )
                | (Planned, Active)
                | (Active, Replanning | Completed)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, MissionState::Completed | MissionState::Terminated)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionTerminationReason {
    BudgetExhausted,
    SlaExceeded,
    PolicyDenied,
    ApprovalDenied,
    GuardrailViolation,
    Unrecoverable,
    GovernanceConstraintViolated,
    InvalidStateTransition,
    AlreadyTerminalState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionFailureMetadata {
    pub checkpoint_index: u32,
    pub reason: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MissionResumeMetadata {
    pub last_successful_checkpoint: Option<u32>,
    pub latest_failure: Option<MissionFailureMetadata>,
}

#[derive(Debug, Clone)]
pub struct MissionGovernance {
    pub max_runtime_ms: u64,
    pub max_steps: u32,
    pub max_estimated_cost_cents: u32,
    pub elapsed_ms: u64,
    pub completed_steps: u32,
    pub accumulated_cost_cents: u32,
}

impl MissionGovernance {
    pub fn validate(&self) -> Result<(), MissionTerminationReason> {
        if self.max_runtime_ms == 0 || self.max_steps == 0 || self.max_estimated_cost_cents == 0 {
            return Err(MissionTerminationReason::GovernanceConstraintViolated);
        }
        Ok(())
    }

    pub fn from_config_strict(
        config: &crate::config::MissionConfig,
    ) -> Result<Self, MissionTerminationReason> {
        let governance = Self {
            max_runtime_ms: config.max_runtime_ms,
            max_steps: config.max_steps,
            max_estimated_cost_cents: config.max_estimated_cost_cents,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };
        governance.validate()?;
        Ok(governance)
    }

    pub fn from_json_strict(raw: &str) -> Result<Self, MissionTerminationReason> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        let object = value
            .as_object()
            .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;

        let enabled = object
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;
        let max_runtime_ms = parse_positive_u64(object.get("max_runtime_ms"))?;
        let max_steps = parse_positive_u32(object.get("max_steps"))?;
        let max_estimated_cost_cents = parse_positive_u32(object.get("max_estimated_cost_cents"))?;

        let config = crate::config::MissionConfig {
            enabled,
            max_runtime_ms,
            max_steps,
            max_estimated_cost_cents,
        };

        Self::from_config_strict(&config)
    }
}

#[derive(Debug, Clone)]
pub struct MissionCheckpoint {
    pub index: u32,
    pub objective_fragment: String,
}

#[derive(Debug, Clone)]
pub struct MissionPlan {
    pub objective: String,
    pub checkpoints: Vec<MissionCheckpoint>,
    pub resume: MissionResumeMetadata,
}

#[derive(Debug, Clone)]
pub struct MissionOutcome {
    pub mission_id: String,
    pub state: MissionState,
    pub termination: Option<MissionTerminationReason>,
    pub checkpoints_completed: u32,
    pub resume_metadata: MissionResumeMetadata,
}

pub struct MissionCoordinator {
    pub state: Arc<Mutex<MissionState>>,
    pub governance: MissionGovernance,
    pub accumulated_cost_cents: Arc<Mutex<u32>>,
    pub accumulated_steps: Arc<Mutex<u32>>,
    pub elapsed_ms: Arc<Mutex<u64>>,
    pub latest_successful_checkpoint: Arc<Mutex<Option<u32>>>,
    pub latest_failure: Arc<Mutex<Option<MissionFailureMetadata>>>,
}

impl MissionCoordinator {
    pub fn new(governance: MissionGovernance) -> Self {
        Self {
            state: Arc::new(Mutex::new(MissionState::ObjectiveAccepted)),
            governance,
            accumulated_cost_cents: Arc::new(Mutex::new(0)),
            accumulated_steps: Arc::new(Mutex::new(0)),
            elapsed_ms: Arc::new(Mutex::new(0)),
            latest_successful_checkpoint: Arc::new(Mutex::new(None)),
            latest_failure: Arc::new(Mutex::new(None)),
        }
    }

    pub fn plan_for_objective(objective: &str) -> MissionPlan {
        let checkpoints: Vec<MissionCheckpoint> = mission_fragments(objective)
            .into_iter()
            .enumerate()
            .map(|(index, fragment)| MissionCheckpoint {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                objective_fragment: fragment,
            })
            .collect();

        MissionPlan {
            objective: objective.trim().to_string(),
            checkpoints,
            resume: MissionResumeMetadata::default(),
        }
    }

    pub fn transition(
        &self,
        target: MissionState,
    ) -> Result<MissionState, MissionTerminationReason> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| MissionTerminationReason::Unrecoverable)?;

        if state.is_terminal() {
            return if *state == target {
                Ok(state.clone())
            } else {
                Err(MissionTerminationReason::AlreadyTerminalState)
            };
        }

        if !state.allows_transition_to(&target) {
            return Err(MissionTerminationReason::InvalidStateTransition);
        }

        *state = target;
        Ok(state.clone())
    }

    pub fn current_state(&self) -> Result<MissionState, MissionTerminationReason> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| MissionTerminationReason::Unrecoverable)
    }

    pub fn record_checkpoint_success(
        &self,
        checkpoint_index: u32,
    ) -> Result<(), MissionTerminationReason> {
        let mut latest = self
            .latest_successful_checkpoint
            .lock()
            .map_err(|_| MissionTerminationReason::Unrecoverable)?;
        *latest = Some(checkpoint_index);
        drop(latest);

        let mut completed_steps = self
            .accumulated_steps
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        *completed_steps = completed_steps
            .checked_add(1)
            .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;
        Ok(())
    }

    pub fn record_checkpoint_failure(
        &self,
        checkpoint_index: u32,
        reason: impl Into<String>,
        recoverable: bool,
    ) -> Result<(), MissionTerminationReason> {
        let mut latest_failure = self
            .latest_failure
            .lock()
            .map_err(|_| MissionTerminationReason::Unrecoverable)?;
        *latest_failure = Some(MissionFailureMetadata {
            checkpoint_index,
            reason: reason.into(),
            recoverable,
        });
        Ok(())
    }

    pub fn latest_successful_checkpoint(&self) -> Result<Option<u32>, MissionTerminationReason> {
        self.latest_successful_checkpoint
            .lock()
            .map(|value| *value)
            .map_err(|_| MissionTerminationReason::Unrecoverable)
    }

    pub fn resume_metadata(&self) -> Result<MissionResumeMetadata, MissionTerminationReason> {
        let last_successful_checkpoint = self.latest_successful_checkpoint()?;
        let latest_failure = self
            .latest_failure
            .lock()
            .map(|value| value.clone())
            .map_err(|_| MissionTerminationReason::Unrecoverable)?;

        Ok(MissionResumeMetadata {
            last_successful_checkpoint,
            latest_failure,
        })
    }

    pub fn should_replan(&self, error_message: &str) -> bool {
        let normalized = error_message.to_ascii_lowercase();
        normalized.contains("timeout")
            || normalized.contains("temporary")
            || normalized.contains("retry")
            || normalized.contains("rate limit")
    }

    pub fn validate_governance(&self) -> Result<(), MissionTerminationReason> {
        self.governance.validate()?;
        let (elapsed_ms, completed_steps, accumulated_cost_cents) = self.accounting_snapshot()?;
        if governance_exceeded(
            elapsed_ms,
            completed_steps,
            accumulated_cost_cents,
            &self.governance,
            false,
        )
        .is_some()
        {
            return Err(MissionTerminationReason::GovernanceConstraintViolated);
        }
        Ok(())
    }

    pub fn enforce_pre_checkpoint(&self) -> Result<(), MissionTerminationReason> {
        self.governance.validate()?;
        let (elapsed_ms, completed_steps, accumulated_cost_cents) = self.accounting_snapshot()?;
        if let Some(reason) = governance_exceeded(
            elapsed_ms,
            completed_steps,
            accumulated_cost_cents,
            &self.governance,
            true,
        ) {
            return Err(reason);
        }
        Ok(())
    }

    pub fn record_checkpoint_accounting(
        &self,
        elapsed_ms_delta: u64,
        cost_cents_delta: u32,
    ) -> Result<(), MissionTerminationReason> {
        let mut elapsed_ms = self
            .elapsed_ms
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        *elapsed_ms = elapsed_ms
            .checked_add(elapsed_ms_delta)
            .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;
        drop(elapsed_ms);

        let mut accumulated_cost_cents = self
            .accumulated_cost_cents
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        *accumulated_cost_cents = accumulated_cost_cents
            .checked_add(cost_cents_delta)
            .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;
        drop(accumulated_cost_cents);

        self.enforce_post_checkpoint()
    }

    pub fn enforce_post_checkpoint(&self) -> Result<(), MissionTerminationReason> {
        self.governance.validate()?;
        let (elapsed_ms, completed_steps, accumulated_cost_cents) = self.accounting_snapshot()?;
        if let Some(reason) = governance_exceeded(
            elapsed_ms,
            completed_steps,
            accumulated_cost_cents,
            &self.governance,
            false,
        ) {
            return Err(reason);
        }
        Ok(())
    }

    fn accounting_snapshot(&self) -> Result<(u64, u32, u32), MissionTerminationReason> {
        let elapsed_ms = *self
            .elapsed_ms
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        let completed_steps = *self
            .accumulated_steps
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        let accumulated_cost_cents = *self
            .accumulated_cost_cents
            .lock()
            .map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)?;
        Ok((elapsed_ms, completed_steps, accumulated_cost_cents))
    }
}

impl From<crate::config::MissionConfig> for MissionGovernance {
    fn from(config: crate::config::MissionConfig) -> Self {
        Self {
            max_runtime_ms: config.max_runtime_ms,
            max_steps: config.max_steps,
            max_estimated_cost_cents: config.max_estimated_cost_cents,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        }
    }
}

fn mission_fragments(objective: &str) -> Vec<String> {
    let objective = objective.trim();
    if objective.is_empty() {
        return vec![];
    }

    let checkpoint_tokens: Vec<String> = objective
        .split("->")
        .map(str::trim)
        .filter(|fragment| !fragment.is_empty())
        .map(ToString::to_string)
        .collect();

    if !checkpoint_tokens.is_empty() {
        return checkpoint_tokens;
    }

    let line_tokens: Vec<String> = objective
        .lines()
        .map(str::trim)
        .filter(|fragment| !fragment.is_empty())
        .map(ToString::to_string)
        .collect();

    if !line_tokens.is_empty() {
        return line_tokens;
    }

    vec![objective.to_string()]
}

fn parse_positive_u64(value: Option<&serde_json::Value>) -> Result<u64, MissionTerminationReason> {
    let parsed = value
        .and_then(serde_json::Value::as_u64)
        .ok_or(MissionTerminationReason::GovernanceConstraintViolated)?;
    if parsed == 0 {
        return Err(MissionTerminationReason::GovernanceConstraintViolated);
    }
    Ok(parsed)
}

fn parse_positive_u32(value: Option<&serde_json::Value>) -> Result<u32, MissionTerminationReason> {
    let parsed_u64 = parse_positive_u64(value)?;
    u32::try_from(parsed_u64).map_err(|_| MissionTerminationReason::GovernanceConstraintViolated)
}

fn governance_exceeded(
    elapsed_ms: u64,
    completed_steps: u32,
    accumulated_cost_cents: u32,
    governance: &MissionGovernance,
    inclusive: bool,
) -> Option<MissionTerminationReason> {
    let budget_exceeded = if inclusive {
        completed_steps >= governance.max_steps
            || accumulated_cost_cents >= governance.max_estimated_cost_cents
    } else {
        completed_steps > governance.max_steps
            || accumulated_cost_cents > governance.max_estimated_cost_cents
    };
    if budget_exceeded {
        return Some(MissionTerminationReason::BudgetExhausted);
    }

    let sla_exceeded = if inclusive {
        elapsed_ms >= governance.max_runtime_ms
    } else {
        elapsed_ms > governance.max_runtime_ms
    };
    if sla_exceeded {
        return Some(MissionTerminationReason::SlaExceeded);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn valid_transitions_follow_lifecycle_invariants() {
        let valid_cases = vec![
            (MissionState::ObjectiveAccepted, MissionState::Planned),
            (MissionState::ObjectiveAccepted, MissionState::Terminated),
            (MissionState::Planned, MissionState::Active),
            (MissionState::Planned, MissionState::Terminated),
            (MissionState::Active, MissionState::Replanning),
            (MissionState::Active, MissionState::Completed),
            (MissionState::Active, MissionState::Terminated),
            (MissionState::Replanning, MissionState::Planned),
            (MissionState::Replanning, MissionState::Terminated),
        ];

        for (from, to) in valid_cases {
            assert!(
                from.allows_transition_to(&to),
                "expected transition {:?} -> {:?} to be valid",
                from,
                to
            );
        }
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let invalid_cases = vec![
            (MissionState::ObjectiveAccepted, MissionState::Active),
            (MissionState::ObjectiveAccepted, MissionState::Completed),
            (MissionState::Planned, MissionState::ObjectiveAccepted),
            (MissionState::Planned, MissionState::Completed),
            (MissionState::Active, MissionState::Planned),
            (MissionState::Replanning, MissionState::Active),
            (MissionState::Completed, MissionState::Terminated),
            (MissionState::Terminated, MissionState::Planned),
        ];

        for (from, to) in invalid_cases {
            assert!(
                !from.allows_transition_to(&to),
                "expected transition {:?} -> {:?} to be invalid",
                from,
                to
            );
        }
    }

    #[test]
    fn coordinator_transition_enforces_guards() {
        let governance = MissionGovernance {
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };

        let coordinator = MissionCoordinator::new(governance);
        assert_eq!(
            coordinator.transition(MissionState::Planned).unwrap(),
            MissionState::Planned
        );
        assert_eq!(
            coordinator.transition(MissionState::Active).unwrap(),
            MissionState::Active
        );

        let error = coordinator
            .transition(MissionState::ObjectiveAccepted)
            .unwrap_err();
        assert_eq!(error, MissionTerminationReason::InvalidStateTransition);
    }

    #[test]
    fn coordinator_terminal_state_handling_is_deterministic() {
        let governance = MissionGovernance {
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };

        let coordinator = MissionCoordinator::new(governance);
        coordinator.transition(MissionState::Terminated).unwrap();

        assert_eq!(
            coordinator.transition(MissionState::Terminated).unwrap(),
            MissionState::Terminated
        );

        let error = coordinator.transition(MissionState::Planned).unwrap_err();
        assert_eq!(error, MissionTerminationReason::AlreadyTerminalState);
    }

    #[test]
    fn concurrent_transition_attempts_are_serialized_with_single_winner() {
        let governance = MissionGovernance {
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };

        let coordinator = Arc::new(MissionCoordinator::new(governance));
        coordinator.transition(MissionState::Planned).unwrap();
        coordinator.transition(MissionState::Active).unwrap();

        let to_completed = Arc::clone(&coordinator);
        let completed_handle =
            thread::spawn(move || to_completed.transition(MissionState::Completed));

        let to_replanning = Arc::clone(&coordinator);
        let replanning_handle =
            thread::spawn(move || to_replanning.transition(MissionState::Replanning));

        let completed_result = completed_handle.join().unwrap();
        let replanning_result = replanning_handle.join().unwrap();

        let success_count =
            usize::from(completed_result.is_ok()) + usize::from(replanning_result.is_ok());
        assert_eq!(
            success_count, 1,
            "exactly one concurrent transition must succeed"
        );

        let failure_reason = match (completed_result.err(), replanning_result.err()) {
            (Some(reason), None) | (None, Some(reason)) => reason,
            _ => panic!("expected one transition to fail deterministically"),
        };
        assert!(matches!(
            failure_reason,
            MissionTerminationReason::AlreadyTerminalState
                | MissionTerminationReason::InvalidStateTransition
        ));

        let final_state = coordinator.current_state().unwrap();
        assert!(
            matches!(
                final_state,
                MissionState::Completed | MissionState::Replanning
            ),
            "final state must be a valid lifecycle progression"
        );
    }

    #[test]
    fn governance_validation_fails_closed_for_non_positive_ceilings() {
        let governance = MissionGovernance {
            max_runtime_ms: 0,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };

        let error = governance.validate().unwrap_err();
        assert_eq!(
            error,
            MissionTerminationReason::GovernanceConstraintViolated
        );
    }

    #[test]
    fn planner_builds_ordered_checkpoints_from_objective() {
        let plan = MissionCoordinator::plan_for_objective("collect -> analyze -> report");

        assert_eq!(plan.checkpoints.len(), 3);
        assert_eq!(plan.checkpoints[0].index, 0);
        assert_eq!(plan.checkpoints[0].objective_fragment, "collect");
        assert_eq!(plan.checkpoints[1].index, 1);
        assert_eq!(plan.checkpoints[1].objective_fragment, "analyze");
        assert_eq!(plan.checkpoints[2].index, 2);
        assert_eq!(plan.checkpoints[2].objective_fragment, "report");
    }

    #[test]
    fn resume_metadata_tracks_latest_checkpoint_and_failure() {
        let governance = MissionGovernance {
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };

        let coordinator = MissionCoordinator::new(governance);
        coordinator.record_checkpoint_success(1).unwrap();
        coordinator
            .record_checkpoint_failure(2, "temporary upstream timeout", true)
            .unwrap();

        let metadata = coordinator.resume_metadata().unwrap();
        assert_eq!(metadata.last_successful_checkpoint, Some(1));
        assert_eq!(
            metadata.latest_failure,
            Some(MissionFailureMetadata {
                checkpoint_index: 2,
                reason: "temporary upstream timeout".to_string(),
                recoverable: true,
            })
        );
    }

    #[test]
    fn replan_classifier_flags_recoverable_errors() {
        let governance = MissionGovernance {
            max_runtime_ms: 300_000,
            max_steps: 10,
            max_estimated_cost_cents: 100,
            elapsed_ms: 0,
            completed_steps: 0,
            accumulated_cost_cents: 0,
        };
        let coordinator = MissionCoordinator::new(governance);

        assert!(coordinator.should_replan("Temporary timeout while executing checkpoint"));
        assert!(!coordinator.should_replan("permissions denied by policy"));
    }
}
