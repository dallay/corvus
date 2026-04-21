use super::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;

/// Explicit no-op memory backend.
///
/// This backend is used when `memory.backend = "none"` to disable persistence
/// while keeping the runtime wiring stable.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoneMemory;

impl NoneMemory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Memory for NoneMemory {
    fn name(&self) -> &str {
        "none"
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
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        is_task_unsupported_error, TaskCreateInput, TaskListQuery, TaskPatch, TaskPriority,
        TaskStatus,
    };

    #[tokio::test]
    async fn none_memory_is_noop() {
        let memory = NoneMemory::new();

        memory
            .store("k", "v", MemoryCategory::Core, None)
            .await
            .unwrap();

        assert!(memory.get("k").await.unwrap().is_none());
        assert!(memory.recall("k", 10, None).await.unwrap().is_empty());
        assert!(memory.list(None, None).await.unwrap().is_empty());
        assert!(!memory.forget("k").await.unwrap());
        assert_eq!(memory.count().await.unwrap(), 0);
        assert!(memory.health_check().await);
    }

    #[tokio::test]
    async fn none_memory_rejects_slash_session_operations() {
        let memory = NoneMemory::new();

        let error = memory
            .create_session_snapshot(
                "session-1",
                super::super::traits::SessionSnapshotKind::Compact,
                serde_json::json!({"preview": "hello"}),
                true,
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("backend=none"));
    }

    #[tokio::test]
    async fn none_memory_rejects_persistent_task_operations() {
        let memory = NoneMemory::new();

        let create_error = memory
            .create_task(TaskCreateInput {
                id: "11111111-1111-4111-8111-111111111111".into(),
                title: "Review parity slice".into(),
                description: String::new(),
                status: TaskStatus::Pending,
                priority: TaskPriority::Medium,
                session_id: None,
                created_at: "2026-04-18T00:00:00Z".into(),
                updated_at: "2026-04-18T00:00:00Z".into(),
            })
            .await
            .unwrap_err();
        assert!(is_task_unsupported_error(&create_error));

        let list_error = memory
            .list_tasks(TaskListQuery {
                session_id: None,
                status: None,
                priority: None,
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap_err();
        assert!(list_error.to_string().contains("backend=none"));

        let update_error = memory
            .update_task(TaskPatch {
                id: "11111111-1111-4111-8111-111111111111".into(),
                title: None,
                description: Some("updated".into()),
                status: None,
                priority: None,
            })
            .await
            .unwrap_err();
        assert!(is_task_unsupported_error(&update_error));
    }
}
