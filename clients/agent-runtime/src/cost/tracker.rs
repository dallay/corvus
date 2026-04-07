use super::types::{
    BudgetCheck, BudgetScopeStatus, CostAuditEvent, CostAuditKind, CostBudgetReservation,
    CostHistory, CostHistoryPoint, CostHistoryTotals, CostOverrideRecord, CostOverrideRequest,
    CostOverrideScope, CostRecord, CostResetRequest, CostResetResult, CostResetScope, CostSummary,
    CostTrackerSnapshot, MissionBudgetScope, ModelStats, TokenUsage, UsagePeriod,
};
use crate::config::schema::CostConfig;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use parking_lot::{Mutex, MutexGuard, RwLock};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cost tracker for API usage monitoring and budget enforcement.
pub struct CostTracker {
    config: RwLock<CostConfig>,
    storage: Arc<Mutex<CostStorage>>,
    audit_storage: Arc<Mutex<CostAuditStorage>>,
    session_id: String,
    session_costs: Arc<Mutex<Vec<CostRecord>>>,
    active_override: Arc<Mutex<Option<CostOverrideRecord>>>,
    pending_reservations: Arc<Mutex<HashMap<String, CostBudgetReservation>>>,
    cumulative_total_cost_usd: Arc<Mutex<f64>>,
}

const MAX_HISTORY_WINDOW_DAYS: usize = 366;
const MAX_HISTORY_WINDOW_MONTHS: usize = 60;
const REDACTED_AUDIT_VALUE: &str = "[REDACTED]";

impl CostTracker {
    /// Create a new cost tracker.
    pub fn new(config: CostConfig, workspace_dir: &Path) -> Result<Self> {
        let storage_path = resolve_storage_path(workspace_dir)?;
        let audit_path = resolve_audit_path(workspace_dir);

        let storage = CostStorage::new(&storage_path).with_context(|| {
            format!("Failed to open cost storage at {}", storage_path.display())
        })?;
        let cumulative_total_cost_usd = storage
            .read_records()?
            .into_iter()
            .map(|record| record.usage.cost_usd)
            .sum();
        let audit_storage = CostAuditStorage::new(&audit_path).with_context(|| {
            format!(
                "Failed to open cost audit storage at {}",
                audit_path.display()
            )
        })?;

        Ok(Self {
            config: RwLock::new(config),
            storage: Arc::new(Mutex::new(storage)),
            audit_storage: Arc::new(Mutex::new(audit_storage)),
            session_id: uuid::Uuid::new_v4().to_string(),
            session_costs: Arc::new(Mutex::new(Vec::new())),
            active_override: Arc::new(Mutex::new(None)),
            pending_reservations: Arc::new(Mutex::new(HashMap::new())),
            cumulative_total_cost_usd: Arc::new(Mutex::new(cumulative_total_cost_usd)),
        })
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    fn lock_storage(&self) -> MutexGuard<'_, CostStorage> {
        self.storage.lock()
    }

    fn lock_session_costs(&self) -> MutexGuard<'_, Vec<CostRecord>> {
        self.session_costs.lock()
    }

    fn lock_audit_storage(&self) -> MutexGuard<'_, CostAuditStorage> {
        self.audit_storage.lock()
    }

    fn lock_active_override(&self) -> MutexGuard<'_, Option<CostOverrideRecord>> {
        self.active_override.lock()
    }

    fn lock_pending_reservations(&self) -> MutexGuard<'_, HashMap<String, CostBudgetReservation>> {
        self.pending_reservations.lock()
    }

    fn lock_cumulative_total_cost_usd(&self) -> MutexGuard<'_, f64> {
        self.cumulative_total_cost_usd.lock()
    }

    fn redacted_audit_actor(actor: &str) -> Option<String> {
        (!actor.trim().is_empty()).then(|| REDACTED_AUDIT_VALUE.to_string())
    }

    fn redacted_audit_reason(reason: Option<&str>) -> Option<String> {
        reason
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(|_| REDACTED_AUDIT_VALUE.to_string())
    }

    fn append_audit_event_best_effort(&self, event: CostAuditEvent) {
        if let Err(error) = self.append_audit_event(event.clone()) {
            tracing::warn!(
                kind = ?event.kind,
                override_id = event.override_id.as_deref().unwrap_or("none"),
                session_id = event.session_id.as_deref().unwrap_or("none"),
                actor = event.actor.as_deref().unwrap_or("none"),
                "Failed to persist cost audit event: {error}"
            );
        }
    }

    pub fn config(&self) -> CostConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, next: CostConfig) {
        *self.config.write() = next;
    }

    /// Check if a request is within budget.
    pub fn check_budget(&self, estimated_cost_usd: f64) -> Result<BudgetCheck> {
        self.check_budget_with_mission_scope(estimated_cost_usd, None)
    }

    pub fn check_budget_with_mission_scope(
        &self,
        estimated_cost_usd: f64,
        mission_scope: Option<&MissionBudgetScope>,
    ) -> Result<BudgetCheck> {
        self.check_budget_with_pending_scope(estimated_cost_usd, mission_scope, 0.0, None)
    }

    fn check_budget_with_pending_scope(
        &self,
        estimated_cost_usd: f64,
        mission_scope: Option<&MissionBudgetScope>,
        pending_total_usd: f64,
        pending_mission_usd: Option<f64>,
    ) -> Result<BudgetCheck> {
        let config = self.config();

        if !config.enabled {
            return Ok(BudgetCheck::Allowed);
        }

        if !estimated_cost_usd.is_finite() || estimated_cost_usd < 0.0 {
            return Err(anyhow!(
                "Estimated cost must be a finite, non-negative value"
            ));
        }

        let mut storage = self.lock_storage();
        let (daily_cost, monthly_cost) = storage.get_aggregated_costs()?;
        drop(storage);

        let session_cost = self.current_session_cost_usd();
        let projected_daily = daily_cost + pending_total_usd + estimated_cost_usd;
        let projected_monthly = monthly_cost + pending_total_usd + estimated_cost_usd;
        let projected_session = session_cost + pending_total_usd + estimated_cost_usd;

        let mut checks = vec![
            build_budget_check(
                UsagePeriod::Session,
                session_cost,
                projected_session,
                config.session_limit_usd,
                config.warn_at_percent,
            ),
            build_budget_check(
                UsagePeriod::Day,
                daily_cost,
                projected_daily,
                config.daily_limit_usd,
                config.warn_at_percent,
            ),
            build_budget_check(
                UsagePeriod::Month,
                monthly_cost,
                projected_monthly,
                config.monthly_limit_usd,
                config.warn_at_percent,
            ),
        ];

        if let Some(mission_scope) = mission_scope {
            let pending_mission_usd = pending_mission_usd.unwrap_or(0.0);
            checks.push(build_budget_check(
                UsagePeriod::Mission,
                mission_scope.current_usd,
                mission_scope.current_usd + pending_mission_usd + estimated_cost_usd,
                mission_scope.limit_usd,
                config.warn_at_percent,
            ));
        }

        Ok(select_budget_check(checks))
    }

    pub fn snapshot(&self, now: DateTime<Utc>) -> Result<CostTrackerSnapshot> {
        self.expire_override_if_needed(now)?;
        let config = self.config();

        let mut storage = self.lock_storage();
        let (daily_cost, monthly_cost) = storage.get_aggregated_costs()?;
        let session_costs = self.lock_session_costs();
        let active_override = self.lock_active_override().clone();

        let session_cost: f64 = session_costs
            .iter()
            .map(|record| record.usage.cost_usd)
            .sum();
        let total_tokens: u64 = session_costs
            .iter()
            .map(|record| record.usage.total_tokens)
            .sum();
        let request_count = session_costs.len();
        let by_model = build_session_model_stats(&session_costs);

        let scope_statuses = if config.enabled {
            vec![
                build_scope_status(
                    UsagePeriod::Session,
                    session_cost,
                    config.session_limit_usd,
                    config.warn_at_percent,
                ),
                build_scope_status(
                    UsagePeriod::Day,
                    daily_cost,
                    config.daily_limit_usd,
                    config.warn_at_percent,
                ),
                build_scope_status(
                    UsagePeriod::Month,
                    monthly_cost,
                    config.monthly_limit_usd,
                    config.warn_at_percent,
                ),
            ]
        } else {
            Vec::new()
        };

        drop(session_costs);
        drop(storage);

        Ok(CostTrackerSnapshot {
            session_id: self.session_id.clone(),
            usage: CostSummary {
                session_cost_usd: session_cost,
                daily_cost_usd: daily_cost,
                monthly_cost_usd: monthly_cost,
                total_tokens,
                request_count,
                by_model,
            },
            scope_statuses,
            active_override,
        })
    }

    /// Record a usage event.
    pub fn record_usage(&self, usage: TokenUsage) -> Result<()> {
        self.record_usage_for_session(&self.session_id, usage)
    }

    pub fn record_usage_for_session(
        &self,
        session_id: impl Into<String>,
        usage: TokenUsage,
    ) -> Result<()> {
        if !self.config().enabled {
            return Ok(());
        }

        if !usage.cost_usd.is_finite() || usage.cost_usd < 0.0 {
            return Err(anyhow!(
                "Token usage cost must be a finite, non-negative value"
            ));
        }

        let session_id = session_id.into();
        let record = CostRecord::new(&session_id, usage);

        // Persist first for durability guarantees.
        {
            let mut storage = self.lock_storage();
            storage.add_record(record.clone())?;
        }

        *self.lock_cumulative_total_cost_usd() += record.usage.cost_usd;

        // Then update in-memory session snapshot.
        if session_id == self.session_id {
            let mut session_costs = self.lock_session_costs();
            session_costs.push(record);
        }

        Ok(())
    }

    pub fn cumulative_total_cost_usd(&self) -> f64 {
        *self.lock_cumulative_total_cost_usd()
    }

    /// Get the current cost summary.
    pub fn get_summary(&self) -> Result<CostSummary> {
        Ok(self.snapshot(Utc::now())?.usage)
    }

    /// Get the daily cost for a specific date.
    pub fn get_daily_cost(&self, date: NaiveDate) -> Result<f64> {
        let storage = self.lock_storage();
        storage.get_cost_for_date(date)
    }

    /// Get the monthly cost for a specific month.
    pub fn get_monthly_cost(&self, year: i32, month: u32) -> Result<f64> {
        let storage = self.lock_storage();
        storage.get_cost_for_month(year, month)
    }

    pub fn scope_statuses(&self) -> Result<Vec<BudgetScopeStatus>> {
        Ok(self.snapshot(Utc::now())?.scope_statuses)
    }

    pub fn reserve_budget_for_request(
        &self,
        estimated_cost_usd: f64,
        mission_scope: Option<&MissionBudgetScope>,
        now: DateTime<Utc>,
    ) -> Result<(
        BudgetCheck,
        Option<CostOverrideRecord>,
        Option<CostBudgetReservation>,
    )> {
        self.expire_override_if_needed(now)?;

        let pending_total_usd;
        let pending_mission_usd;
        {
            let pending = self.lock_pending_reservations();
            pending_total_usd = pending
                .values()
                .map(|reservation| reservation.estimated_cost_usd)
                .sum();
            pending_mission_usd = mission_scope.map(|scope| {
                pending
                    .values()
                    .filter(|reservation| {
                        reservation.mission_id.as_deref() == Some(scope.mission_id.as_str())
                    })
                    .map(|reservation| reservation.estimated_cost_usd)
                    .sum()
            });
        }

        let check = self.check_budget_with_pending_scope(
            estimated_cost_usd,
            mission_scope,
            pending_total_usd,
            pending_mission_usd,
        )?;

        let mut override_applied = None;
        if matches!(check, BudgetCheck::Exceeded { .. }) && self.config().allow_override {
            override_applied = self.consume_override_if_active(now)?;
        }

        let proceed = !matches!(check, BudgetCheck::Exceeded { .. }) || override_applied.is_some();
        let reservation = if proceed && estimated_cost_usd > 0.0 {
            let reservation = CostBudgetReservation {
                id: uuid::Uuid::new_v4().to_string(),
                estimated_cost_usd,
                mission_id: mission_scope.map(|scope| scope.mission_id.clone()),
                created_at: now,
            };
            self.lock_pending_reservations()
                .insert(reservation.id.clone(), reservation.clone());
            Some(reservation)
        } else {
            None
        };

        Ok((check, override_applied, reservation))
    }

    pub fn release_budget_reservation(&self, reservation_id: &str) {
        self.lock_pending_reservations().remove(reservation_id);
    }

    pub fn commit_budget_reservation(&self, reservation_id: &str) {
        self.release_budget_reservation(reservation_id);
    }

    fn current_session_cost_usd(&self) -> f64 {
        self.lock_session_costs()
            .iter()
            .map(|record| record.usage.cost_usd)
            .sum()
    }

    pub fn history_window(
        &self,
        period: UsagePeriod,
        window: usize,
        now: DateTime<Utc>,
    ) -> Result<CostHistory> {
        if window == 0 {
            return Err(anyhow!("History window must be greater than zero"));
        }

        let records = self.lock_storage().read_records()?;
        build_history_from_window(period, window, now, &records)
    }

    pub fn history_range(
        &self,
        period: UsagePeriod,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<CostHistory> {
        if start > end {
            return Err(anyhow!("History range start must be before end"));
        }

        let records = self.lock_storage().read_records()?;
        build_history_from_range(period, start, end, &records)
    }

    pub fn apply_override(
        &self,
        request: CostOverrideRequest,
        now: DateTime<Utc>,
    ) -> Result<CostOverrideRecord> {
        if !self.config().allow_override {
            return Err(anyhow!("Cost overrides are disabled by policy"));
        }

        self.expire_override_if_needed(now)?;

        let override_record = CostOverrideRecord {
            id: uuid::Uuid::new_v4().to_string(),
            actor: request.actor.clone(),
            scope: request.scope,
            reason: request.reason.clone(),
            requested_at: now,
            expires_at: request.expires_at,
            session_id: Some(self.session_id.clone()),
            remaining_uses: match request.scope {
                CostOverrideScope::NextRequest => 1,
            },
        };

        *self.lock_active_override() = Some(override_record.clone());

        self.append_audit_event_best_effort(CostAuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            kind: CostAuditKind::OverrideGranted,
            recorded_at: now,
            actor: Self::redacted_audit_actor(&request.actor),
            reason: Self::redacted_audit_reason(request.reason.as_deref()),
            period: None,
            override_scope: Some(request.scope),
            reset_scope: None,
            override_id: Some(override_record.id.clone()),
            session_id: Some(self.session_id.clone()),
            expires_at: override_record.expires_at,
            removed_cost_usd: None,
            removed_requests: None,
        });

        Ok(override_record)
    }

    pub fn active_override(&self, now: DateTime<Utc>) -> Result<Option<CostOverrideRecord>> {
        self.expire_override_if_needed(now)?;
        Ok(self.lock_active_override().clone())
    }

    pub fn consume_override_if_active(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<CostOverrideRecord>> {
        self.expire_override_if_needed(now)?;

        let consumed = {
            let mut active_override = self.lock_active_override();
            match active_override.as_mut() {
                Some(override_record) if override_record.remaining_uses > 0 => {
                    override_record.remaining_uses -= 1;
                    let consumed = override_record.clone();
                    if override_record.remaining_uses == 0 {
                        *active_override = None;
                    }
                    Some(consumed)
                }
                _ => None,
            }
        };

        if let Some(override_record) = consumed.clone() {
            self.append_audit_event_best_effort(CostAuditEvent {
                id: uuid::Uuid::new_v4().to_string(),
                kind: CostAuditKind::OverrideConsumed,
                recorded_at: now,
                actor: Self::redacted_audit_actor(&override_record.actor),
                reason: Self::redacted_audit_reason(override_record.reason.as_deref()),
                period: None,
                override_scope: Some(override_record.scope),
                reset_scope: None,
                override_id: Some(override_record.id.clone()),
                session_id: override_record.session_id.clone(),
                expires_at: override_record.expires_at,
                removed_cost_usd: None,
                removed_requests: None,
            });
        }

        Ok(consumed)
    }

    pub fn reset(&self, request: CostResetRequest, now: DateTime<Utc>) -> Result<CostResetResult> {
        let session_id = self.session_id.clone();
        let mut storage = self.lock_storage();
        let records = storage.read_records()?;
        let mut kept = Vec::with_capacity(records.len());
        let mut removed = Vec::new();

        for record in records {
            if matches_reset_scope(&record, request.scope, &session_id, now) {
                removed.push(record);
            } else {
                kept.push(record);
            }
        }

        storage.replace_records(&kept)?;
        drop(storage);

        {
            let mut session_costs = self.lock_session_costs();
            session_costs
                .retain(|record| !matches_reset_scope(record, request.scope, &session_id, now));
        }

        let removed_cost_usd: f64 = removed.iter().map(|record| record.usage.cost_usd).sum();
        let removed_requests = removed.len();

        let audit_event = CostAuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            kind: CostAuditKind::ResetApplied,
            recorded_at: now,
            actor: Self::redacted_audit_actor(&request.actor),
            reason: Self::redacted_audit_reason(request.reason.as_deref()),
            period: None,
            override_scope: None,
            reset_scope: Some(request.scope),
            override_id: None,
            session_id: Some(session_id),
            expires_at: None,
            removed_cost_usd: Some(removed_cost_usd),
            removed_requests: Some(removed_requests),
        };

        self.append_audit_event_best_effort(audit_event.clone());

        Ok(CostResetResult {
            scope: request.scope,
            removed_cost_usd,
            removed_requests,
            effective_at: now,
            audit_event,
        })
    }

    pub fn audit_trail(&self, limit: usize) -> Result<Vec<CostAuditEvent>> {
        self.lock_audit_storage().read_events(limit)
    }

    fn append_audit_event(&self, event: CostAuditEvent) -> Result<()> {
        self.lock_audit_storage().append(event)
    }

    fn expire_override_if_needed(&self, now: DateTime<Utc>) -> Result<()> {
        let expired = {
            let mut active_override = self.lock_active_override();
            match active_override.as_ref() {
                Some(override_record)
                    if override_record
                        .expires_at
                        .is_some_and(|expires_at| expires_at <= now) =>
                {
                    active_override.take()
                }
                _ => None,
            }
        };

        if let Some(override_record) = expired {
            self.append_audit_event_best_effort(CostAuditEvent {
                id: uuid::Uuid::new_v4().to_string(),
                kind: CostAuditKind::OverrideExpired,
                recorded_at: now,
                actor: Self::redacted_audit_actor(&override_record.actor),
                reason: Self::redacted_audit_reason(override_record.reason.as_deref()),
                period: None,
                override_scope: Some(override_record.scope),
                reset_scope: None,
                override_id: Some(override_record.id.clone()),
                session_id: override_record.session_id.clone(),
                expires_at: override_record.expires_at,
                removed_cost_usd: None,
                removed_requests: None,
            });
        }

        Ok(())
    }
}

fn resolve_storage_path(workspace_dir: &Path) -> Result<PathBuf> {
    let storage_path = workspace_dir.join("state").join("costs.jsonl");
    let legacy_path = workspace_dir.join(".corvus").join("costs.db");

    if !storage_path.exists() && legacy_path.exists() {
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        if let Err(error) = fs::rename(&legacy_path, &storage_path) {
            tracing::warn!(
                "Failed to move legacy cost storage from {} to {}: {error}; falling back to copy",
                legacy_path.display(),
                storage_path.display()
            );
            fs::copy(&legacy_path, &storage_path).with_context(|| {
                format!(
                    "Failed to copy legacy cost storage from {} to {}",
                    legacy_path.display(),
                    storage_path.display()
                )
            })?;
        }
    }

    Ok(storage_path)
}

fn resolve_audit_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join("cost-audit.jsonl")
}

fn build_budget_check(
    period: UsagePeriod,
    current_usd: f64,
    projected_usd: f64,
    limit_usd: f64,
    warn_at_percent: u8,
) -> BudgetCheck {
    if limit_usd <= 0.0 {
        return BudgetCheck::Allowed;
    }

    let percent_used = (projected_usd / limit_usd) * 100.0;
    if projected_usd > limit_usd {
        return BudgetCheck::Exceeded {
            current_usd,
            projected_usd,
            limit_usd,
            percent_used,
            period,
        };
    }

    let warn_threshold = f64::from(warn_at_percent.min(100));
    if percent_used >= warn_threshold {
        return BudgetCheck::Warning {
            current_usd,
            projected_usd,
            limit_usd,
            percent_used,
            period,
        };
    }

    BudgetCheck::Allowed
}

fn build_scope_status(
    period: UsagePeriod,
    current_usd: f64,
    limit_usd: f64,
    warn_at_percent: u8,
) -> BudgetScopeStatus {
    let check = build_budget_check(period, current_usd, current_usd, limit_usd, warn_at_percent);
    let percent_used = if limit_usd > 0.0 {
        (current_usd / limit_usd) * 100.0
    } else {
        0.0
    };

    BudgetScopeStatus {
        period,
        state: check.state(),
        current_usd,
        limit_usd,
        percent_used,
    }
}

fn select_budget_check<I>(checks: I) -> BudgetCheck
where
    I: IntoIterator<Item = BudgetCheck>,
{
    checks
        .into_iter()
        .max_by(|left, right| {
            budget_check_severity(left)
                .cmp(&budget_check_severity(right))
                .then_with(|| {
                    budget_check_percent_used(left).total_cmp(&budget_check_percent_used(right))
                })
        })
        .unwrap_or(BudgetCheck::Allowed)
}

fn budget_check_severity(check: &BudgetCheck) -> u8 {
    match check {
        BudgetCheck::Allowed => 0,
        BudgetCheck::Warning { .. } => 1,
        BudgetCheck::Exceeded { .. } => 2,
    }
}

fn budget_check_percent_used(check: &BudgetCheck) -> f64 {
    match check {
        BudgetCheck::Allowed => 0.0,
        BudgetCheck::Warning { percent_used, .. } | BudgetCheck::Exceeded { percent_used, .. } => {
            *percent_used
        }
    }
}

fn build_history_from_window(
    period: UsagePeriod,
    window: usize,
    now: DateTime<Utc>,
    records: &[CostRecord],
) -> Result<CostHistory> {
    match period {
        UsagePeriod::Day => {
            if window > MAX_HISTORY_WINDOW_DAYS {
                return Err(anyhow!("History window is too large"));
            }
            let start = now - Duration::days((window.saturating_sub(1)) as i64);
            build_history_from_range(period, start, now, records)
        }
        UsagePeriod::Month => {
            if window > MAX_HISTORY_WINDOW_MONTHS {
                return Err(anyhow!("History window is too large"));
            }
            let month_offset = i32::try_from(window.saturating_sub(1))
                .map_err(|_| anyhow!("History window is too large"))?;
            let (start_year, start_month) = shift_month(now.year(), now.month(), -month_offset);
            let start = Utc
                .with_ymd_and_hms(start_year, start_month, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| anyhow!("Invalid monthly history window start"))?;
            build_history_from_range(period, start, now, records)
        }
        UsagePeriod::Session => Err(anyhow!("Session history windows are not supported yet")),
        UsagePeriod::Mission => Err(anyhow!("Mission history windows are not supported yet")),
    }
}

fn build_history_from_range(
    period: UsagePeriod,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    records: &[CostRecord],
) -> Result<CostHistory> {
    match period {
        UsagePeriod::Day => build_daily_history(start, end, records),
        UsagePeriod::Month => build_monthly_history(start, end, records),
        UsagePeriod::Session => Err(anyhow!("Session history ranges are not supported yet")),
        UsagePeriod::Mission => Err(anyhow!("Mission history ranges are not supported yet")),
    }
}

fn build_daily_history(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    records: &[CostRecord],
) -> Result<CostHistory> {
    let start_date = start.date_naive();
    let end_date = end.date_naive();
    let bucket_count = (end_date - start_date).num_days();
    let mut points = Vec::new();
    let mut by_bucket: HashMap<NaiveDate, CostHistoryPoint> = HashMap::new();

    for index in 0..=bucket_count {
        let bucket_date = start_date + Duration::days(index);
        by_bucket.insert(
            bucket_date,
            CostHistoryPoint {
                bucket: bucket_date.format("%Y-%m-%d").to_string(),
                cost_usd: 0.0,
                tokens: 0,
                requests: 0,
            },
        );
    }

    for record in records {
        let bucket_date = record.usage.timestamp.date_naive();
        if bucket_date < start_date || bucket_date > end_date {
            continue;
        }

        if let Some(point) = by_bucket.get_mut(&bucket_date) {
            point.cost_usd += record.usage.cost_usd;
            point.tokens += record.usage.total_tokens;
            point.requests += 1;
        }
    }

    let mut dates: Vec<_> = by_bucket.into_iter().collect();
    dates.sort_by_key(|(date, _)| *date);
    let mut totals = CostHistoryTotals {
        cost_usd: 0.0,
        tokens: 0,
        requests: 0,
    };

    for (_, point) in dates {
        totals.cost_usd += point.cost_usd;
        totals.tokens += point.tokens;
        totals.requests += point.requests;
        points.push(point);
    }

    Ok(CostHistory {
        period: UsagePeriod::Day,
        points,
        totals,
    })
}

fn build_monthly_history(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    records: &[CostRecord],
) -> Result<CostHistory> {
    let mut points = Vec::new();
    let mut by_bucket: HashMap<(i32, u32), CostHistoryPoint> = HashMap::new();
    let mut year = start.year();
    let mut month = start.month();
    let end_key = (end.year(), end.month());

    loop {
        by_bucket.insert(
            (year, month),
            CostHistoryPoint {
                bucket: format!("{year:04}-{month:02}"),
                cost_usd: 0.0,
                tokens: 0,
                requests: 0,
            },
        );

        if (year, month) == end_key {
            break;
        }

        (year, month) = shift_month(year, month, 1);
    }

    for record in records {
        let bucket_key = (
            record.usage.timestamp.year(),
            record.usage.timestamp.month(),
        );
        if let Some(point) = by_bucket.get_mut(&bucket_key) {
            point.cost_usd += record.usage.cost_usd;
            point.tokens += record.usage.total_tokens;
            point.requests += 1;
        }
    }

    let mut buckets: Vec<_> = by_bucket.into_iter().collect();
    buckets.sort_by_key(|((year, month), _)| (*year, *month));
    let mut totals = CostHistoryTotals {
        cost_usd: 0.0,
        tokens: 0,
        requests: 0,
    };

    for (_, point) in buckets {
        totals.cost_usd += point.cost_usd;
        totals.tokens += point.tokens;
        totals.requests += point.requests;
        points.push(point);
    }

    Ok(CostHistory {
        period: UsagePeriod::Month,
        points,
        totals,
    })
}

fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let absolute = year * 12 + month as i32 - 1 + delta;
    let shifted_year = absolute.div_euclid(12);
    let shifted_month = absolute.rem_euclid(12) as u32 + 1;
    (shifted_year, shifted_month)
}

fn matches_reset_scope(
    record: &CostRecord,
    scope: CostResetScope,
    session_id: &str,
    now: DateTime<Utc>,
) -> bool {
    match scope {
        CostResetScope::Session => record.session_id == session_id,
        CostResetScope::Day => record.usage.timestamp.date_naive() == now.date_naive(),
        CostResetScope::Month => {
            record.usage.timestamp.year() == now.year()
                && record.usage.timestamp.month() == now.month()
        }
    }
}

fn build_session_model_stats(session_costs: &[CostRecord]) -> HashMap<String, ModelStats> {
    let mut by_model: HashMap<String, ModelStats> = HashMap::new();

    for record in session_costs {
        let entry = by_model
            .entry(record.usage.model.clone())
            .or_insert_with(|| ModelStats {
                model: record.usage.model.clone(),
                cost_usd: 0.0,
                total_tokens: 0,
                request_count: 0,
            });

        entry.cost_usd += record.usage.cost_usd;
        entry.total_tokens += record.usage.total_tokens;
        entry.request_count += 1;
    }

    by_model
}

/// Persistent storage for cost records.
struct CostStorage {
    path: PathBuf,
    daily_cost_usd: f64,
    monthly_cost_usd: f64,
    cached_day: NaiveDate,
    cached_year: i32,
    cached_month: u32,
}

impl CostStorage {
    /// Create or open cost storage.
    fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let now = Utc::now();
        let mut storage = Self {
            path: path.to_path_buf(),
            daily_cost_usd: 0.0,
            monthly_cost_usd: 0.0,
            cached_day: now.date_naive(),
            cached_year: now.year(),
            cached_month: now.month(),
        };

        storage.rebuild_aggregates(
            storage.cached_day,
            storage.cached_year,
            storage.cached_month,
        )?;

        Ok(storage)
    }

    fn for_each_record<F>(&self, mut on_record: F) -> Result<()>
    where
        F: FnMut(CostRecord),
    {
        if !self.path.exists() {
            return Ok(());
        }

        let file = File::open(&self.path)
            .with_context(|| format!("Failed to read cost storage from {}", self.path.display()))?;
        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            let raw_line = line.with_context(|| {
                format!(
                    "Failed to read line {} from cost storage {}",
                    line_number + 1,
                    self.path.display()
                )
            })?;

            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<CostRecord>(trimmed) {
                Ok(record) => on_record(record),
                Err(error) => {
                    tracing::warn!(
                        "Skipping malformed cost record at {}:{}: {error}",
                        self.path.display(),
                        line_number + 1
                    );
                }
            }
        }

        Ok(())
    }

    fn read_records(&self) -> Result<Vec<CostRecord>> {
        let mut records = Vec::new();
        self.for_each_record(|record| records.push(record))?;
        Ok(records)
    }

    fn rebuild_aggregates(&mut self, day: NaiveDate, year: i32, month: u32) -> Result<()> {
        let mut daily_cost = 0.0;
        let mut monthly_cost = 0.0;

        self.for_each_record(|record| {
            let timestamp = record.usage.timestamp.naive_utc();

            if timestamp.date() == day {
                daily_cost += record.usage.cost_usd;
            }

            if timestamp.year() == year && timestamp.month() == month {
                monthly_cost += record.usage.cost_usd;
            }
        })?;

        self.daily_cost_usd = daily_cost;
        self.monthly_cost_usd = monthly_cost;
        self.cached_day = day;
        self.cached_year = year;
        self.cached_month = month;

        Ok(())
    }

    fn ensure_period_cache_current(&mut self) -> Result<()> {
        let now = Utc::now();
        let day = now.date_naive();
        let year = now.year();
        let month = now.month();

        if day != self.cached_day || year != self.cached_year || month != self.cached_month {
            self.rebuild_aggregates(day, year, month)?;
        }

        Ok(())
    }

    /// Add a new record.
    fn add_record(&mut self, record: CostRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open cost storage at {}", self.path.display()))?;

        writeln!(file, "{}", serde_json::to_string(&record)?)
            .with_context(|| format!("Failed to write cost record to {}", self.path.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to sync cost storage at {}", self.path.display()))?;

        self.ensure_period_cache_current()?;

        let timestamp = record.usage.timestamp.naive_utc();
        if timestamp.date() == self.cached_day {
            self.daily_cost_usd += record.usage.cost_usd;
        }
        if timestamp.year() == self.cached_year && timestamp.month() == self.cached_month {
            self.monthly_cost_usd += record.usage.cost_usd;
        }

        Ok(())
    }

    fn replace_records(&mut self, records: &[CostRecord]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let temp_path = self.path.with_extension("jsonl.tmp");
        let mut file = File::create(&temp_path)
            .with_context(|| format!("Failed to create temp storage at {}", temp_path.display()))?;

        for record in records {
            writeln!(file, "{}", serde_json::to_string(record)?).with_context(|| {
                format!("Failed to write cost record to {}", temp_path.display())
            })?;
        }

        file.sync_all()
            .with_context(|| format!("Failed to sync temp storage at {}", temp_path.display()))?;
        fs::rename(&temp_path, &self.path).with_context(|| {
            format!(
                "Failed to replace cost storage from {} to {}",
                temp_path.display(),
                self.path.display()
            )
        })?;

        let now = Utc::now();
        self.rebuild_aggregates(now.date_naive(), now.year(), now.month())
    }

    /// Get aggregated costs for current day and month.
    fn get_aggregated_costs(&mut self) -> Result<(f64, f64)> {
        self.ensure_period_cache_current()?;
        Ok((self.daily_cost_usd, self.monthly_cost_usd))
    }

    /// Get cost for a specific date.
    fn get_cost_for_date(&self, date: NaiveDate) -> Result<f64> {
        let mut cost = 0.0;

        self.for_each_record(|record| {
            if record.usage.timestamp.naive_utc().date() == date {
                cost += record.usage.cost_usd;
            }
        })?;

        Ok(cost)
    }

    /// Get cost for a specific month.
    fn get_cost_for_month(&self, year: i32, month: u32) -> Result<f64> {
        let mut cost = 0.0;

        self.for_each_record(|record| {
            let timestamp = record.usage.timestamp.naive_utc();
            if timestamp.year() == year && timestamp.month() == month {
                cost += record.usage.cost_usd;
            }
        })?;

        Ok(cost)
    }
}

struct CostAuditStorage {
    path: PathBuf,
}

impl CostAuditStorage {
    fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn append(&mut self, event: CostAuditEvent) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open audit storage at {}", self.path.display()))?;

        writeln!(file, "{}", serde_json::to_string(&event)?)
            .with_context(|| format!("Failed to write audit event to {}", self.path.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to sync audit storage at {}", self.path.display()))?;
        Ok(())
    }

    fn read_events(&self, limit: usize) -> Result<Vec<CostAuditEvent>> {
        if limit == 0 || !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).with_context(|| {
            format!("Failed to read audit storage from {}", self.path.display())
        })?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_number, line) in reader.lines().enumerate() {
            let raw_line = line.with_context(|| {
                format!(
                    "Failed to read line {} from audit storage {}",
                    line_number + 1,
                    self.path.display()
                )
            })?;

            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<CostAuditEvent>(trimmed) {
                Ok(event) => events.push(event),
                Err(error) => tracing::warn!(
                    "Skipping malformed cost audit record at {}:{}: {error}",
                    self.path.display(),
                    line_number + 1
                ),
            }
        }

        if events.len() > limit {
            let split_at = events.len() - limit;
            Ok(events.split_off(split_at))
        } else {
            Ok(events)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{
        CostAuditKind, CostOverrideRequest, CostOverrideScope, CostResetRequest, CostResetScope,
        CostService,
    };
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn enabled_config() -> CostConfig {
        CostConfig {
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn cost_tracker_initialization() {
        let tmp = TempDir::new().unwrap();
        let tracker = CostTracker::new(enabled_config(), tmp.path()).unwrap();
        assert!(!tracker.session_id().is_empty());
    }

    #[test]
    fn budget_check_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: false,
            ..Default::default()
        };

        let tracker = CostTracker::new(config, tmp.path()).unwrap();
        let check = tracker.check_budget(1000.0).unwrap();
        assert!(matches!(check, BudgetCheck::Allowed));
    }

    #[test]
    fn record_usage_and_get_summary() {
        let tmp = TempDir::new().unwrap();
        let tracker = CostTracker::new(enabled_config(), tmp.path()).unwrap();

        let usage = TokenUsage::new("test/model", 1000, 500, 1.0, 2.0);
        tracker.record_usage(usage).unwrap();

        let summary = tracker.get_summary().unwrap();
        assert_eq!(summary.request_count, 1);
        assert!(summary.session_cost_usd > 0.0);
        assert_eq!(summary.by_model.len(), 1);
    }

    #[test]
    fn budget_exceeded_daily_limit() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            daily_limit_usd: 0.01, // Very low limit
            ..Default::default()
        };

        let tracker = CostTracker::new(config, tmp.path()).unwrap();

        // Record a usage that exceeds the limit
        let usage = TokenUsage::new("test/model", 10000, 5000, 1.0, 2.0); // ~0.02 USD
        tracker.record_usage(usage).unwrap();

        let check = tracker.check_budget(0.01).unwrap();
        assert!(matches!(check, BudgetCheck::Exceeded { .. }));
    }

    #[test]
    fn summary_by_model_is_session_scoped() {
        let tmp = TempDir::new().unwrap();
        let storage_path = resolve_storage_path(tmp.path()).unwrap();
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let old_record = CostRecord::new(
            "old-session",
            TokenUsage::new("legacy/model", 500, 500, 1.0, 1.0),
        );
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(storage_path)
            .unwrap();
        writeln!(file, "{}", serde_json::to_string(&old_record).unwrap()).unwrap();
        file.sync_all().unwrap();

        let tracker = CostTracker::new(enabled_config(), tmp.path()).unwrap();
        tracker
            .record_usage(TokenUsage::new("session/model", 1000, 1000, 1.0, 1.0))
            .unwrap();

        let summary = tracker.get_summary().unwrap();
        assert_eq!(summary.by_model.len(), 1);
        assert!(summary.by_model.contains_key("session/model"));
        assert!(!summary.by_model.contains_key("legacy/model"));
    }

    #[test]
    fn malformed_lines_are_ignored_while_loading() {
        let tmp = TempDir::new().unwrap();
        let storage_path = resolve_storage_path(tmp.path()).unwrap();
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let valid_usage = TokenUsage::new("test/model", 1000, 0, 1.0, 1.0);
        let valid_record = CostRecord::new("session-a", valid_usage.clone());

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(storage_path)
            .unwrap();
        writeln!(file, "{}", serde_json::to_string(&valid_record).unwrap()).unwrap();
        writeln!(file, "not-a-json-line").unwrap();
        writeln!(file).unwrap();
        file.sync_all().unwrap();

        let tracker = CostTracker::new(enabled_config(), tmp.path()).unwrap();
        let today_cost = tracker.get_daily_cost(Utc::now().date_naive()).unwrap();
        assert!((today_cost - valid_usage.cost_usd).abs() < f64::EPSILON);
    }

    #[test]
    fn invalid_budget_estimate_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let tracker = CostTracker::new(enabled_config(), tmp.path()).unwrap();

        let err = tracker.check_budget(f64::NAN).unwrap_err();
        assert!(err
            .to_string()
            .contains("Estimated cost must be a finite, non-negative value"));
    }

    #[test]
    fn warning_threshold_uses_projected_cost_math() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            session_limit_usd: 10.0,
            daily_limit_usd: 10.0,
            monthly_limit_usd: 100.0,
            warn_at_percent: 80,
            ..Default::default()
        };
        let tracker = CostTracker::new(config, tmp.path()).unwrap();

        let usage = TokenUsage::new("test/model", 1_000_000, 0, 7.5, 0.0);
        tracker.record_usage(usage).unwrap();

        let check = tracker.check_budget(0.5).unwrap();
        match check {
            BudgetCheck::Warning {
                period,
                current_usd,
                limit_usd,
                projected_usd,
                percent_used,
            } => {
                assert_eq!(period, UsagePeriod::Day);
                assert!((current_usd - 7.5).abs() < 0.0001);
                assert!((projected_usd - 8.0).abs() < 0.0001);
                assert!((limit_usd - 10.0).abs() < 0.0001);
                assert!((percent_used - 80.0).abs() < 0.0001);
            }
            other => panic!("expected warning, got {other:?}"),
        }
    }

    #[test]
    fn session_scope_is_evaluated_with_day_and_month_limits() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            session_limit_usd: 5.0,
            daily_limit_usd: 10.0,
            monthly_limit_usd: 100.0,
            warn_at_percent: 80,
            ..Default::default()
        };
        let tracker = CostTracker::new(config, tmp.path()).unwrap();

        let mut usage = TokenUsage::new("test/model", 1_000, 0, 0.0, 0.0);
        usage.cost_usd = 4.9;
        tracker.record_usage(usage).unwrap();

        match tracker.check_budget(0.2).unwrap() {
            BudgetCheck::Exceeded {
                period, limit_usd, ..
            } => {
                assert_eq!(period, UsagePeriod::Session);
                assert!((limit_usd - 5.0).abs() < 0.0001);
            }
            other => panic!("expected session-scope exceedance, got {other:?}"),
        }

        let scopes = tracker.scope_statuses().unwrap();
        assert!(scopes
            .iter()
            .any(|scope| scope.period == UsagePeriod::Session));
        assert!(scopes.iter().any(|scope| scope.period == UsagePeriod::Day));
        assert!(scopes
            .iter()
            .any(|scope| scope.period == UsagePeriod::Month));
    }

    #[test]
    fn mission_scope_can_govern_request_when_more_restrictive() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            session_limit_usd: 10.0,
            daily_limit_usd: 100.0,
            monthly_limit_usd: 1000.0,
            warn_at_percent: 80,
            ..Default::default()
        };
        let tracker = Arc::new(CostTracker::new(config, tmp.path()).unwrap());
        let service = CostService::new(tracker);

        let evaluation = service
            .evaluate_request(
                0.1,
                Some(crate::cost::MissionBudgetScope {
                    mission_id: "mission-a".to_string(),
                    current_usd: 0.95,
                    limit_usd: 1.0,
                }),
                chrono::Utc::now(),
            )
            .unwrap();

        match evaluation {
            crate::cost::BudgetEvaluation::Blocked {
                check:
                    BudgetCheck::Exceeded {
                        period, limit_usd, ..
                    },
            } => {
                assert_eq!(period, UsagePeriod::Mission);
                assert!((limit_usd - 1.0).abs() < 0.0001);
            }
            other => panic!("expected mission-scope block, got {other:?}"),
        }
    }

    #[test]
    fn history_window_aggregates_daily_buckets() {
        let tmp = TempDir::new().unwrap();
        let tracker = Arc::new(CostTracker::new(enabled_config(), tmp.path()).unwrap());
        let service = CostService::new(tracker.clone());
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 4, 6, 12, 0, 0)
            .single()
            .unwrap();

        let records = [
            ("day-1", now - chrono::Duration::days(2), 1.25),
            ("day-2", now - chrono::Duration::days(1), 2.0),
            (
                "day-2",
                now - chrono::Duration::days(1) + chrono::Duration::hours(1),
                0.5,
            ),
            ("day-3", now, 3.25),
        ];

        for (session_id, timestamp, cost_usd) in records {
            let mut usage = TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
            usage.cost_usd = cost_usd;
            usage.timestamp = timestamp;
            tracker.record_usage_for_session(session_id, usage).unwrap();
        }

        let history = service.history_window(UsagePeriod::Day, 3, now).unwrap();
        assert_eq!(history.points.len(), 3);
        assert_eq!(history.points[0].bucket, "2026-04-04");
        assert!((history.points[0].cost_usd - 1.25).abs() < 0.0001);
        assert_eq!(history.points[1].bucket, "2026-04-05");
        assert!((history.points[1].cost_usd - 2.5).abs() < 0.0001);
        assert_eq!(history.points[2].bucket, "2026-04-06");
        assert!((history.points[2].cost_usd - 3.25).abs() < 0.0001);
        assert!((history.totals.cost_usd - 7.0).abs() < 0.0001);
    }

    #[test]
    fn history_window_rejects_oversized_ranges() {
        let tmp = TempDir::new().unwrap();
        let tracker = Arc::new(CostTracker::new(enabled_config(), tmp.path()).unwrap());
        let service = CostService::new(tracker);
        let now = chrono::Utc::now();

        let day_err = service
            .history_window(UsagePeriod::Day, MAX_HISTORY_WINDOW_DAYS + 1, now)
            .unwrap_err();
        assert!(day_err.to_string().contains("History window is too large"));

        let month_err = service
            .history_window(UsagePeriod::Month, MAX_HISTORY_WINDOW_MONTHS + 1, now)
            .unwrap_err();
        assert!(month_err
            .to_string()
            .contains("History window is too large"));
    }

    #[test]
    fn evaluate_request_reserves_budget_until_released() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            session_limit_usd: 1.0,
            daily_limit_usd: 10.0,
            monthly_limit_usd: 100.0,
            warn_at_percent: 80,
            ..Default::default()
        };
        let tracker = Arc::new(CostTracker::new(config, tmp.path()).unwrap());
        let service = CostService::new(tracker.clone());
        let now = chrono::Utc::now();

        let first = service.evaluate_request(0.75, None, now).unwrap();
        let reservation = match first {
            crate::cost::BudgetEvaluation::Proceed {
                reservation: Some(reservation),
                ..
            } => reservation,
            other => panic!("expected reservation, got {other:?}"),
        };

        let second = service.evaluate_request(0.3, None, now).unwrap();
        assert!(matches!(
            second,
            crate::cost::BudgetEvaluation::Blocked { .. }
        ));

        tracker.release_budget_reservation(&reservation.id);

        let third = service.evaluate_request(0.3, None, now).unwrap();
        assert!(matches!(
            third,
            crate::cost::BudgetEvaluation::Proceed { .. }
        ));
    }

    #[test]
    fn reset_session_removes_only_current_session_records() {
        let tmp = TempDir::new().unwrap();
        let tracker = Arc::new(CostTracker::new(enabled_config(), tmp.path()).unwrap());
        let service = CostService::new(tracker.clone());
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 4, 6, 12, 0, 0)
            .single()
            .unwrap();

        let mut current_usage = TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        current_usage.cost_usd = 1.5;
        current_usage.timestamp = now;
        tracker.record_usage(current_usage).unwrap();

        let mut other_usage = TokenUsage::new("test/model", 2_000, 500, 0.0, 0.0);
        other_usage.cost_usd = 2.0;
        other_usage.timestamp = now;
        tracker
            .record_usage_for_session("other-session", other_usage)
            .unwrap();

        let result = service
            .reset(
                CostResetRequest {
                    scope: CostResetScope::Session,
                    actor: "tester".to_string(),
                    reason: Some("clear current session".to_string()),
                },
                now,
            )
            .unwrap();

        assert_eq!(result.scope, CostResetScope::Session);
        assert_eq!(result.removed_requests, 1);
        assert!((result.removed_cost_usd - 1.5).abs() < 0.0001);

        let summary = tracker.get_summary().unwrap();
        assert_eq!(summary.request_count, 0);
        let day_cost = tracker.get_daily_cost(now.date_naive()).unwrap();
        assert!((day_cost - 2.0).abs() < 0.0001);
    }

    #[test]
    fn next_request_override_expires_before_use() {
        let tmp = TempDir::new().unwrap();
        let config = CostConfig {
            enabled: true,
            session_limit_usd: 1.0,
            daily_limit_usd: 1.0,
            monthly_limit_usd: 100.0,
            warn_at_percent: 80,
            allow_override: true,
            ..Default::default()
        };
        let tracker = Arc::new(CostTracker::new(config, tmp.path()).unwrap());
        let service = CostService::new(tracker.clone());
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 4, 6, 12, 0, 0)
            .single()
            .unwrap();

        let mut usage = TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        usage.timestamp = now;
        tracker.record_usage(usage).unwrap();

        service
            .apply_override(
                CostOverrideRequest {
                    actor: "operator".to_string(),
                    scope: CostOverrideScope::NextRequest,
                    reason: Some("one retry".to_string()),
                    expires_at: Some(now + chrono::Duration::minutes(5)),
                },
                now,
            )
            .unwrap();

        let evaluation = service
            .evaluate_request(0.1, None, now + chrono::Duration::minutes(6))
            .unwrap();
        assert!(matches!(
            evaluation,
            crate::cost::BudgetEvaluation::Blocked { .. }
        ));

        let audit = service.audit_trail(10).unwrap();
        assert!(audit
            .iter()
            .any(|event| event.kind == CostAuditKind::OverrideExpired));
    }
}
