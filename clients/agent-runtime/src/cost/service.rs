use super::tracker::CostTracker;
use super::types::{
    BudgetEvaluation, BudgetState, CostAuditEvent, CostGovernanceSummary, CostHistory,
    CostOverrideRecord, CostOverrideRequest, CostResetRequest, CostResetResult, MissionBudgetScope,
    UsagePeriod,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Thin runtime-facing orchestration layer over the tracker.
#[derive(Clone)]
pub struct CostService {
    tracker: Arc<CostTracker>,
}

impl CostService {
    pub fn new(tracker: Arc<CostTracker>) -> Self {
        Self { tracker }
    }

    pub fn current_summary(&self, now: DateTime<Utc>) -> Result<CostGovernanceSummary> {
        let usage = self.tracker.get_summary()?;
        let scope_statuses = self.tracker.scope_statuses()?;
        let active_override = self.tracker.active_override(now)?;

        let active_scope = scope_statuses.iter().max_by(|left, right| {
            budget_state_rank(left.state)
                .cmp(&budget_state_rank(right.state))
                .then_with(|| left.percent_used.total_cmp(&right.percent_used))
        });

        Ok(CostGovernanceSummary {
            session_id: self.tracker.session_id().to_string(),
            usage,
            budget_state: active_scope.map_or(BudgetState::Allowed, |status| status.state),
            active_period: active_scope.map(|status| status.period),
            scope_statuses,
            active_override,
        })
    }

    pub fn history_window(
        &self,
        period: UsagePeriod,
        window: usize,
        now: DateTime<Utc>,
    ) -> Result<CostHistory> {
        self.tracker.history_window(period, window, now)
    }

    pub fn history_range(
        &self,
        period: UsagePeriod,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<CostHistory> {
        self.tracker.history_range(period, start, end)
    }

    pub fn reset(&self, request: CostResetRequest, now: DateTime<Utc>) -> Result<CostResetResult> {
        self.tracker.reset(request, now)
    }

    pub fn apply_override(
        &self,
        request: CostOverrideRequest,
        now: DateTime<Utc>,
    ) -> Result<CostOverrideRecord> {
        self.tracker.apply_override(request, now)
    }

    pub fn audit_trail(&self, limit: usize) -> Result<Vec<CostAuditEvent>> {
        self.tracker.audit_trail(limit)
    }

    pub fn evaluate_request(
        &self,
        estimated_cost_usd: f64,
        mission_scope: Option<MissionBudgetScope>,
        now: DateTime<Utc>,
    ) -> Result<BudgetEvaluation> {
        let check = self
            .tracker
            .check_budget_with_mission_scope(estimated_cost_usd, mission_scope.as_ref())?;

        if matches!(check, super::types::BudgetCheck::Exceeded { .. }) {
            if self.tracker.config().allow_override {
                if let Some(override_applied) = self.tracker.consume_override_if_active(now)? {
                    return Ok(BudgetEvaluation::Proceed {
                        check,
                        override_applied: Some(override_applied),
                    });
                }
            }

            return Ok(BudgetEvaluation::Blocked { check });
        }

        Ok(BudgetEvaluation::Proceed {
            check,
            override_applied: None,
        })
    }
}

fn budget_state_rank(state: BudgetState) -> u8 {
    match state {
        BudgetState::Allowed => 0,
        BudgetState::Warning => 1,
        BudgetState::Exceeded => 2,
    }
}
