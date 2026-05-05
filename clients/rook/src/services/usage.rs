use crate::db::{
    usage::{self, StoredUsageEvent, UsageSummary, UsageSummaryQuery},
    SqliteDb,
};
use crate::domain::RookError;
use std::future::Future;

pub trait UsageService: Send + Sync {
    fn record(&self, event: StoredUsageEvent)
        -> impl Future<Output = Result<(), RookError>> + Send;
    fn summary(
        &self,
        query: UsageSummaryQuery,
    ) -> impl Future<Output = Result<UsageSummary, RookError>> + Send;
}

#[derive(Clone)]
pub struct SqliteUsageService {
    db: SqliteDb,
}

impl SqliteUsageService {
    pub fn new(db: SqliteDb) -> Self {
        Self { db }
    }
}

impl UsageService for SqliteUsageService {
    async fn record(&self, event: StoredUsageEvent) -> Result<(), RookError> {
        usage::insert_usage_event(&self.db, event).await
    }

    async fn summary(&self, query: UsageSummaryQuery) -> Result<UsageSummary, RookError> {
        usage::summarize_usage(&self.db, query).await
    }
}
