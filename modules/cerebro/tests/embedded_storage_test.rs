use cerebro::{storage_from_config, CerebroConfig, MemoryRecord, Storage, StorageMode};
use cerebro::storage::surreal::SurrealStorage;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn embedded_storage_supports_crud() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let mut config = CerebroConfig::default();
    config.storage_mode = StorageMode::EmbeddedSurreal;
    config.storage_path = Some(path.display().to_string());
    config.surreal.username = Some("root".to_string());
    config.surreal.password = Some(secrecy::SecretString::new(
        "secret".to_string().into_boxed_str(),
    ));

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
    let fetched = storage.get("memory-1").await.expect("get after delete");
    assert!(fetched.is_some());
    assert!(fetched.unwrap().deleted);
    let results = storage
        .search("hello", 10, false, None, None)
        .await
        .expect("search after delete");
    assert!(results.is_empty());
    let count = storage.count().await.expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn embedded_storage_exports_collections() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let mut config = CerebroConfig::default();
    config.surreal.storage_path = Some(path.display().to_string());

    let storage = SurrealStorage::new_embedded(&config)
        .await
        .expect("storage init");
    let record = MemoryRecord::new(
        "memory-1".to_string(),
        "shared".to_string(),
        "topic".to_string(),
        json!({ "content": "hello" }),
    );

    storage.save(record).await.expect("save");

    let export = storage.export_collections().await.expect("export");
    assert_eq!(export.memory.len(), 1);
    assert_eq!(export.memory[0].memory_id, "memory-1");
    assert!(export.session.is_empty());
    assert!(export.prompt.is_empty());
}
