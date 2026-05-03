use crate::db::SqliteDb;
use crate::domain::RookError;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct StoredUsageEvent {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub request_id: Option<String>,
    pub logical_model: String,
    pub vendor: String,
    pub account_id: Option<String>,
    pub account_label: String,
    pub stream: bool,
    pub outcome: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub currency: Option<String>,
    pub provider_request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageSummaryQuery {
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageAggregate {
    pub requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub streaming_requests: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub known_token_requests: u64,
    pub estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageGroupAggregate {
    pub key: String,
    pub aggregate: UsageAggregate,
}

type UsageGroupRow = (String, i64, i64, i64, i64, i64, i64, i64, i64, Option<f64>);

#[derive(Debug, Clone, PartialEq)]
pub struct UsageSummary {
    pub totals: UsageAggregate,
    pub by_model: Vec<UsageGroupAggregate>,
    pub by_vendor: Vec<UsageGroupAggregate>,
    pub by_outcome: Vec<UsageGroupAggregate>,
}

pub async fn insert_usage_event(db: &SqliteDb, event: StoredUsageEvent) -> Result<(), RookError> {
    sqlx::query(
        "INSERT INTO usage_events (
            id, occurred_at, request_id, logical_model, vendor, account_id, account_label,
            stream, outcome, status_code, latency_ms, prompt_tokens, completion_tokens,
            total_tokens, cost_usd, currency, provider_request_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.id)
    .bind(event.occurred_at.to_rfc3339())
    .bind(event.request_id)
    .bind(event.logical_model)
    .bind(event.vendor)
    .bind(event.account_id)
    .bind(event.account_label)
    .bind(if event.stream { 1_i64 } else { 0_i64 })
    .bind(event.outcome)
    .bind(i64::from(event.status_code))
    .bind(i64::try_from(event.latency_ms).unwrap_or(i64::MAX))
    .bind(
        event
            .prompt_tokens
            .and_then(|value| i64::try_from(value).ok()),
    )
    .bind(
        event
            .completion_tokens
            .and_then(|value| i64::try_from(value).ok()),
    )
    .bind(
        event
            .total_tokens
            .and_then(|value| i64::try_from(value).ok()),
    )
    .bind(event.cost_usd)
    .bind(event.currency)
    .bind(event.provider_request_id)
    .execute(db.pool())
    .await
    .map_err(|e| RookError::Registry(format!("failed to insert usage event: {e}")))?;

    Ok(())
}

pub async fn summarize_usage(
    db: &SqliteDb,
    query: UsageSummaryQuery,
) -> Result<UsageSummary, RookError> {
    let limit = query.limit.clamp(1, 100);
    let since = query.since.to_rfc3339();
    let until = query.until.to_rfc3339();

    Ok(UsageSummary {
        totals: aggregate_totals(db, &since, &until).await?,
        by_model: aggregate_group(db, "logical_model", &since, &until, limit).await?,
        by_vendor: aggregate_group(db, "vendor", &since, &until, limit).await?,
        by_outcome: aggregate_group(db, "outcome", &since, &until, limit).await?,
    })
}

async fn aggregate_totals(
    db: &SqliteDb,
    since: &str,
    until: &str,
) -> Result<UsageAggregate, RookError> {
    let row: (i64, i64, i64, i64, i64, i64, i64, i64, Option<f64>) = sqlx::query_as(
        "SELECT
            COUNT(*) AS requests,
            COALESCE(SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END), 0) AS successful_requests,
            COALESCE(SUM(CASE WHEN outcome != 'success' THEN 1 ELSE 0 END), 0) AS failed_requests,
            COALESCE(SUM(CASE WHEN stream = 1 THEN 1 ELSE 0 END), 0) AS streaming_requests,
            COALESCE(SUM(prompt_tokens), 0) AS prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) AS completion_tokens,
            COALESCE(SUM(total_tokens), 0) AS total_tokens,
            COALESCE(SUM(CASE WHEN total_tokens IS NOT NULL THEN 1 ELSE 0 END), 0) AS known_token_requests,
            SUM(cost_usd) AS estimated_cost_usd
        FROM usage_events
        WHERE occurred_at >= ? AND occurred_at < ?",
    )
    .bind(since)
    .bind(until)
    .fetch_one(db.pool())
    .await
    .map_err(|e| RookError::Registry(format!("failed to summarize usage totals: {e}")))?;

    Ok(aggregate_from_row(row))
}

async fn aggregate_group(
    db: &SqliteDb,
    column: &str,
    since: &str,
    until: &str,
    limit: usize,
) -> Result<Vec<UsageGroupAggregate>, RookError> {
    let sql = format!(
        "SELECT
            {column} AS key,
            COUNT(*) AS requests,
            COALESCE(SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END), 0) AS successful_requests,
            COALESCE(SUM(CASE WHEN outcome != 'success' THEN 1 ELSE 0 END), 0) AS failed_requests,
            COALESCE(SUM(CASE WHEN stream = 1 THEN 1 ELSE 0 END), 0) AS streaming_requests,
            COALESCE(SUM(prompt_tokens), 0) AS prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) AS completion_tokens,
            COALESCE(SUM(total_tokens), 0) AS total_tokens,
            COALESCE(SUM(CASE WHEN total_tokens IS NOT NULL THEN 1 ELSE 0 END), 0) AS known_token_requests,
            SUM(cost_usd) AS estimated_cost_usd
        FROM usage_events
        WHERE occurred_at >= ? AND occurred_at < ?
        GROUP BY {column}
        ORDER BY requests DESC, key ASC
        LIMIT ?"
    );

    let rows: Vec<UsageGroupRow> = sqlx::query_as(&sql)
        .bind(since)
        .bind(until)
        .bind(i64::try_from(limit).unwrap_or(100))
        .fetch_all(db.pool())
        .await
        .map_err(|e| RookError::Registry(format!("failed to summarize usage by {column}: {e}")))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                key,
                requests,
                successful,
                failed,
                streaming,
                prompt,
                completion,
                total,
                known,
                cost,
            )| {
                UsageGroupAggregate {
                    key,
                    aggregate: aggregate_from_row((
                        requests, successful, failed, streaming, prompt, completion, total, known,
                        cost,
                    )),
                }
            },
        )
        .collect())
}

fn aggregate_from_row(
    row: (i64, i64, i64, i64, i64, i64, i64, i64, Option<f64>),
) -> UsageAggregate {
    UsageAggregate {
        requests: row.0.max(0) as u64,
        successful_requests: row.1.max(0) as u64,
        failed_requests: row.2.max(0) as u64,
        streaming_requests: row.3.max(0) as u64,
        prompt_tokens: row.4.max(0) as u64,
        completion_tokens: row.5.max(0) as u64,
        total_tokens: row.6.max(0) as u64,
        known_token_requests: row.7.max(0) as u64,
        estimated_cost_usd: row.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn event(
        id: &str,
        model: &str,
        vendor: &str,
        outcome: &str,
        total_tokens: Option<u64>,
    ) -> StoredUsageEvent {
        StoredUsageEvent {
            id: id.to_string(),
            occurred_at: Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap(),
            request_id: Some(format!("req-{id}")),
            logical_model: model.to_string(),
            vendor: vendor.to_string(),
            account_id: Some("acct-1".to_string()),
            account_label: "primary".to_string(),
            stream: false,
            outcome: outcome.to_string(),
            status_code: if outcome == "success" { 200 } else { 502 },
            latency_ms: 42,
            prompt_tokens: total_tokens.map(|_| 10),
            completion_tokens: total_tokens.map(|value| value.saturating_sub(10)),
            total_tokens,
            cost_usd: None,
            currency: None,
            provider_request_id: None,
        }
    }

    #[tokio::test]
    async fn insert_and_summarize_usage_events() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        insert_usage_event(&db, event("one", "gpt-4o", "openai", "success", Some(30)))
            .await
            .unwrap();
        let mut failed = event("two", "gpt-4o", "openai", "upstream_error", None);
        failed.stream = true;
        insert_usage_event(&db, failed).await.unwrap();

        let summary = summarize_usage(
            &db,
            UsageSummaryQuery {
                since: Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
                until: Utc.with_ymd_and_hms(2026, 5, 4, 0, 0, 0).unwrap(),
                limit: 10,
            },
        )
        .await
        .unwrap();

        assert_eq!(summary.totals.requests, 2);
        assert_eq!(summary.totals.successful_requests, 1);
        assert_eq!(summary.totals.failed_requests, 1);
        assert_eq!(summary.totals.streaming_requests, 1);
        assert_eq!(summary.totals.prompt_tokens, 10);
        assert_eq!(summary.totals.completion_tokens, 20);
        assert_eq!(summary.totals.total_tokens, 30);
        assert_eq!(summary.totals.known_token_requests, 1);
        assert_eq!(summary.by_model[0].key, "gpt-4o");
        assert_eq!(summary.by_model[0].aggregate.requests, 2);
        assert_eq!(summary.by_vendor[0].key, "openai");
        assert_eq!(summary.by_outcome.len(), 2);
    }

    #[tokio::test]
    async fn summarize_usage_respects_time_window_and_limit() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let mut old = event("old", "old-model", "openai", "success", Some(50));
        old.occurred_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        insert_usage_event(&db, old).await.unwrap();
        insert_usage_event(
            &db,
            event("new-a", "model-a", "openai", "success", Some(30)),
        )
        .await
        .unwrap();
        insert_usage_event(
            &db,
            event("new-b", "model-b", "anthropic", "success", Some(40)),
        )
        .await
        .unwrap();

        let summary = summarize_usage(
            &db,
            UsageSummaryQuery {
                since: Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
                until: Utc.with_ymd_and_hms(2026, 5, 3, 23, 59, 59).unwrap() + Duration::seconds(1),
                limit: 1,
            },
        )
        .await
        .unwrap();

        assert_eq!(summary.totals.requests, 2);
        assert_eq!(summary.totals.total_tokens, 70);
        assert_eq!(summary.by_model.len(), 1);
        assert_ne!(summary.by_model[0].key, "old-model");
    }
}
