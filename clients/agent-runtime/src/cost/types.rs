use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token usage information from a single API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Model identifier (e.g., "anthropic/claude-sonnet-4-20250514")
    pub model: String,
    /// Input/prompt tokens
    pub input_tokens: u64,
    /// Output/completion tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
    /// Calculated cost in USD
    pub cost_usd: f64,
    /// Timestamp of the request
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TokenUsage {
    fn sanitize_price(value: f64) -> f64 {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            0.0
        }
    }

    /// Create a new token usage record.
    pub fn new(
        model: impl Into<String>,
        input_tokens: u64,
        output_tokens: u64,
        input_price_per_million: f64,
        output_price_per_million: f64,
    ) -> Self {
        let model = model.into();
        let input_price_per_million = Self::sanitize_price(input_price_per_million);
        let output_price_per_million = Self::sanitize_price(output_price_per_million);
        let total_tokens = input_tokens.saturating_add(output_tokens);

        // Calculate cost: (tokens / 1M) * price_per_million
        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price_per_million;
        let cost_usd = input_cost + output_cost;

        Self {
            model,
            input_tokens,
            output_tokens,
            total_tokens,
            cost_usd,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get the total cost.
    pub fn cost(&self) -> f64 {
        self.cost_usd
    }
}

/// Time period for cost aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsagePeriod {
    Session,
    Day,
    Month,
    Mission,
}

/// A single cost record for persistent storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    /// Unique identifier
    pub id: String,
    /// Token usage details
    pub usage: TokenUsage,
    /// Session identifier (for grouping)
    pub session_id: String,
}

impl CostRecord {
    /// Create a new cost record.
    pub fn new(session_id: impl Into<String>, usage: TokenUsage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            usage,
            session_id: session_id.into(),
        }
    }
}

/// Budget enforcement result.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetCheck {
    /// Within budget, request can proceed
    Allowed,
    /// Warning threshold exceeded but request can proceed
    Warning {
        current_usd: f64,
        projected_usd: f64,
        limit_usd: f64,
        percent_used: f64,
        period: UsagePeriod,
    },
    /// Budget exceeded, request blocked
    Exceeded {
        current_usd: f64,
        projected_usd: f64,
        limit_usd: f64,
        percent_used: f64,
        period: UsagePeriod,
    },
}

impl BudgetCheck {
    pub fn state(&self) -> BudgetState {
        match self {
            Self::Allowed => BudgetState::Allowed,
            Self::Warning { .. } => BudgetState::Warning,
            Self::Exceeded { .. } => BudgetState::Exceeded,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetState {
    Allowed,
    Warning,
    Exceeded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetScopeStatus {
    pub period: UsagePeriod,
    pub state: BudgetState,
    pub current_usd: f64,
    pub limit_usd: f64,
    pub percent_used: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionBudgetScope {
    pub mission_id: String,
    pub current_usd: f64,
    pub limit_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostBudgetReservation {
    pub id: String,
    pub estimated_cost_usd: f64,
    pub mission_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostTrackerSnapshot {
    pub session_id: String,
    pub usage: CostSummary,
    pub scope_statuses: Vec<BudgetScopeStatus>,
    pub active_override: Option<CostOverrideRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BudgetEvaluation {
    Proceed {
        check: BudgetCheck,
        override_applied: Option<CostOverrideRecord>,
        reservation: Option<CostBudgetReservation>,
    },
    Blocked {
        check: BudgetCheck,
    },
}

/// Cost summary for reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostSummary {
    /// Total cost for the session
    pub session_cost_usd: f64,
    /// Total cost for the day
    pub daily_cost_usd: f64,
    /// Total cost for the month
    pub monthly_cost_usd: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Number of requests
    pub request_count: usize,
    /// Breakdown by model
    pub by_model: HashMap<String, ModelStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostGovernanceSummary {
    pub session_id: String,
    pub usage: CostSummary,
    pub budget_state: BudgetState,
    pub active_period: Option<UsagePeriod>,
    pub scope_statuses: Vec<BudgetScopeStatus>,
    pub active_override: Option<CostOverrideRecord>,
}

/// Statistics for a specific model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelStats {
    /// Model name
    pub model: String,
    /// Total cost for this model
    pub cost_usd: f64,
    /// Total tokens for this model
    pub total_tokens: u64,
    /// Number of requests for this model
    pub request_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostHistoryPoint {
    pub bucket: String,
    pub cost_usd: f64,
    pub tokens: u64,
    pub requests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostHistoryTotals {
    pub cost_usd: f64,
    pub tokens: u64,
    pub requests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostHistory {
    pub period: UsagePeriod,
    pub points: Vec<CostHistoryPoint>,
    pub totals: CostHistoryTotals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostOverrideScope {
    NextRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostOverrideRequest {
    pub actor: String,
    pub scope: CostOverrideScope,
    pub reason: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostOverrideRecord {
    pub id: String,
    pub actor: String,
    pub scope: CostOverrideScope,
    pub reason: Option<String>,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub session_id: Option<String>,
    pub remaining_uses: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostResetScope {
    Session,
    Day,
    Month,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostResetRequest {
    pub scope: CostResetScope,
    pub actor: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostResetResult {
    pub scope: CostResetScope,
    pub removed_cost_usd: f64,
    pub removed_requests: usize,
    pub effective_at: chrono::DateTime<chrono::Utc>,
    pub audit_event: CostAuditEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostAuditKind {
    OverrideGranted,
    OverrideConsumed,
    OverrideExpired,
    ResetApplied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostAuditEvent {
    pub id: String,
    pub kind: CostAuditKind,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub actor: Option<String>,
    pub reason: Option<String>,
    pub period: Option<UsagePeriod>,
    pub override_scope: Option<CostOverrideScope>,
    pub reset_scope: Option<CostResetScope>,
    pub override_id: Option<String>,
    pub session_id: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub removed_cost_usd: Option<f64>,
    pub removed_requests: Option<usize>,
}

impl Default for CostSummary {
    fn default() -> Self {
        Self {
            session_cost_usd: 0.0,
            daily_cost_usd: 0.0,
            monthly_cost_usd: 0.0,
            total_tokens: 0,
            request_count: 0,
            by_model: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_calculation() {
        let usage = TokenUsage::new("test/model", 1000, 500, 3.0, 15.0);

        // Expected: (1000/1M)*3 + (500/1M)*15 = 0.003 + 0.0075 = 0.0105
        assert!((usage.cost_usd - 0.0105).abs() < 0.0001);
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
        assert_eq!(usage.total_tokens, 1500);
    }

    #[test]
    fn token_usage_zero_tokens() {
        let usage = TokenUsage::new("test/model", 0, 0, 3.0, 15.0);
        assert!(usage.cost_usd.abs() < f64::EPSILON);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn token_usage_negative_or_non_finite_prices_are_clamped() {
        let usage = TokenUsage::new("test/model", 1000, 1000, -3.0, f64::NAN);
        assert!(usage.cost_usd.abs() < f64::EPSILON);
        assert_eq!(usage.total_tokens, 2000);
    }

    #[test]
    fn cost_record_creation() {
        let usage = TokenUsage::new("test/model", 100, 50, 1.0, 2.0);
        let record = CostRecord::new("session-123", usage);

        assert_eq!(record.session_id, "session-123");
        assert!(!record.id.is_empty());
        assert_eq!(record.usage.model, "test/model");
    }
}
