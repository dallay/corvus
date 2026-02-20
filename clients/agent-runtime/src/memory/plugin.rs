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
}
