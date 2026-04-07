use super::tracker::CostTracker;
use super::types::{
    BudgetEvaluation, BudgetState, CostAuditEvent, CostGovernanceSummary, CostHistory,
    CostOverrideRecord, CostOverrideRequest, CostResetRequest, CostResetResult, CostSummary,
    MissionBudgetScope, UsagePeriod,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Thin runtime-facing orchestration layer over the tracker.
#[derive(Clone)]
pub struct CostService {
    tracker: Option<Arc<CostTracker>>,
}

impl CostService {
    pub fn new(tracker: Arc<CostTracker>) -> Self {
        Self {
            tracker: Some(tracker),
        }
    }

    pub fn disabled() -> Self {
        Self { tracker: None }
    }

    pub fn current_summary(&self, now: DateTime<Utc>) -> Result<CostGovernanceSummary> {
        let Some(tracker) = &self.tracker else {
            return Ok(CostGovernanceSummary {
                session_id: "disabled".to_string(),
                usage: CostSummary {
                    session_cost_usd: 0.0,
                    daily_cost_usd: 0.0,
                    monthly_cost_usd: 0.0,
                    total_tokens: 0,
                    request_count: 0,
                    by_model: std::collections::HashMap::new(),
                },
                budget_state: BudgetState::Allowed,
                active_period: None,
                scope_statuses: Vec::new(),
                active_override: None,
            });
        };

        let snapshot = tracker.snapshot(now)?;
        let usage = snapshot.usage;
        let scope_statuses = snapshot.scope_statuses;
        let active_override = snapshot.active_override;

        let active_scope = scope_statuses.iter().max_by(|left, right| {
            budget_state_rank(left.state)
                .cmp(&budget_state_rank(right.state))
                .then_with(|| left.percent_used.total_cmp(&right.percent_used))
        });

        Ok(CostGovernanceSummary {
            session_id: snapshot.session_id,
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
        let Some(tracker) = &self.tracker else {
            return Ok(CostHistory {
                period,
                points: Vec::new(),
                totals: super::types::CostHistoryTotals {
                    cost_usd: 0.0,
                    tokens: 0,
                    requests: 0,
                },
            });
        };

        tracker.history_window(period, window, now)
    }

    pub fn history_range(
        &self,
        period: UsagePeriod,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<CostHistory> {
        let Some(tracker) = &self.tracker else {
            return Ok(CostHistory {
                period,
                points: Vec::new(),
                totals: super::types::CostHistoryTotals {
                    cost_usd: 0.0,
                    tokens: 0,
                    requests: 0,
                },
            });
        };

        tracker.history_range(period, start, end)
    }

    pub fn reset(&self, request: CostResetRequest, now: DateTime<Utc>) -> Result<CostResetResult> {
        let tracker = self
            .tracker
            .as_ref()
            .ok_or_else(|| anyhow!("Cost tracker is unavailable"))?;
        tracker.reset(request, now)
    }

    pub fn apply_override(
        &self,
        request: CostOverrideRequest,
        now: DateTime<Utc>,
    ) -> Result<CostOverrideRecord> {
        let tracker = self
            .tracker
            .as_ref()
            .ok_or_else(|| anyhow!("Cost tracker is unavailable"))?;
        tracker.apply_override(request, now)
    }

    pub fn audit_trail(&self, limit: usize) -> Result<Vec<CostAuditEvent>> {
        let tracker = self
            .tracker
            .as_ref()
            .ok_or_else(|| anyhow!("Cost tracker is unavailable"))?;
        tracker.audit_trail(limit)
    }

    pub fn evaluate_request(
        &self,
        estimated_cost_usd: f64,
        mission_scope: Option<MissionBudgetScope>,
        now: DateTime<Utc>,
    ) -> Result<BudgetEvaluation> {
        let Some(tracker) = &self.tracker else {
            return Ok(BudgetEvaluation::Proceed {
                check: super::types::BudgetCheck::Allowed,
                override_applied: None,
                reservation: None,
            });
        };

        let (check, override_applied, reservation) =
            tracker.reserve_budget_for_request(estimated_cost_usd, mission_scope.as_ref(), now)?;

        if matches!(check, super::types::BudgetCheck::Exceeded { .. }) && override_applied.is_none()
        {
            return Ok(BudgetEvaluation::Blocked { check });
        }

        Ok(BudgetEvaluation::Proceed {
            check,
            override_applied,
            reservation,
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
