use cerebro::{storage_from_config, CerebroConfig, MemoryRecord, StorageMode};
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn embedded_storage_supports_crud() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let mut config = CerebroConfig::default();
    config.storage_mode = StorageMode::EmbeddedSurreal;
    config.storage_path = Some(path.display().to_string());

    let storage = storage_from_config(&config)
        .await
        .expect("storage init");
    let record = MemoryRecord::new(
        "memory-1".to_string(),
        "shared".to_string(),
        "topic".to_string(),
        json!({ "content": "hello" }),
    );

    storage.save(record.clone()).await.expect("save");
    let fetched = storage.get("memory-1").await.expect("get");
    assert!(fetched.is_some());

    let results = storage
        .search("hello", 10, false, None, None)
        .await
        .expect("search");
    assert_eq!(results.len(), 1);

    let deleted = storage.delete("memory-1", false).await.expect("delete");
    assert!(deleted);
    let count = storage.count().await.expect("count");
    assert_eq!(count, 1);
}
