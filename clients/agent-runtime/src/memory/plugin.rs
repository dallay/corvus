use super::markdown::MarkdownMemory;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use std::path::Path;

/// Plugin-backed memory facade.
///
/// Current implementation keeps data-path compatibility by delegating persistence
/// to Markdown memory while plugin loading/security policy is enforced in the
/// memory factory before this adapter is created.
pub struct PluginBackedMemory {
    plugin_id: String,
    fallback: MarkdownMemory,
}

impl PluginBackedMemory {
    pub fn new(plugin_id: String, workspace_dir: &Path) -> Self {
        Self {
            plugin_id,
            fallback: MarkdownMemory::new(workspace_dir),
        }
    }
}

#[async_trait]
impl Memory for PluginBackedMemory {
    fn name(&self) -> &str {
        &self.plugin_id
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.fallback
            .store(key, content, category, session_id)
            .await
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.fallback.recall(query, limit, session_id).await
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        self.fallback.get(key).await
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.fallback.list(category, session_id).await
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.fallback.forget(key).await
    }

    async fn count(&self) -> anyhow::Result<usize> {
        self.fallback.count().await
    }

    async fn health_check(&self) -> bool {
        self.fallback.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn plugin_memory_uses_plugin_name() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.surreal.graphs".to_string(), tmp.path());
        assert_eq!(memory.name(), "memory.surreal.graphs");
    }

    #[tokio::test]
    async fn plugin_memory_delegates_store_and_recall() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.surreal.graphs".to_string(), tmp.path());

        memory
            .store(
                "pref",
                "User likes graph memory",
                MemoryCategory::Core,
                None,
            )
            .await
            .unwrap();

        let recalled = memory.recall("graph", 5, None).await.unwrap();
        assert!(!recalled.is_empty());
    }

    #[tokio::test]
    async fn plugin_memory_delegates_get() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        memory
            .store("key1", "value1", MemoryCategory::Core, None)
            .await
            .unwrap();

        let result = memory.get("key1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().key, "key1");
    }

    #[tokio::test]
    async fn plugin_memory_delegates_list() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        memory
            .store("key1", "value1", MemoryCategory::Core, None)
            .await
            .unwrap();
        memory
            .store("key2", "value2", MemoryCategory::Conversation, None)
            .await
            .unwrap();

        let all = memory.list(None, None).await.unwrap();
        assert!(all.len() >= 2);

        let core_only = memory.list(Some(&MemoryCategory::Core), None).await.unwrap();
        assert!(!core_only.is_empty());
    }

    #[tokio::test]
    async fn plugin_memory_delegates_forget() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        memory
            .store("key1", "value1", MemoryCategory::Core, None)
            .await
            .unwrap();

        let forgotten = memory.forget("key1").await.unwrap();
        assert!(!forgotten); // MarkdownMemory is append-only

        let result = memory.get("key1").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn plugin_memory_delegates_count() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        let initial_count = memory.count().await.unwrap();

        memory
            .store("key1", "value1", MemoryCategory::Core, None)
            .await
            .unwrap();
        memory
            .store("key2", "value2", MemoryCategory::Conversation, None)
            .await
            .unwrap();

        let new_count = memory.count().await.unwrap();
        assert!(new_count >= initial_count + 2);
    }

    #[tokio::test]
    async fn plugin_memory_delegates_health_check() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        let healthy = memory.health_check().await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn plugin_memory_supports_session_filtering() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        memory
            .store("key1", "value1", MemoryCategory::Conversation, Some("session-a"))
            .await
            .unwrap();
        memory
            .store("key2", "value2", MemoryCategory::Conversation, Some("session-b"))
            .await
            .unwrap();

        let session_a = memory.list(None, Some("session-a")).await.unwrap();
        assert!(!session_a.is_empty());
    }

    #[tokio::test]
    async fn plugin_memory_forget_returns_false_for_nonexistent_key() {
        let tmp = TempDir::new().unwrap();
        let memory = PluginBackedMemory::new("memory.test.plugin".to_string(), tmp.path());

        let forgotten = memory.forget("nonexistent-key").await.unwrap();
        assert!(!forgotten);
    }
}