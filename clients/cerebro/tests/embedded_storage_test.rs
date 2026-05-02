use cerebro::storage::surreal::SurrealStorage;
use cerebro::{storage_from_config, CerebroConfig, MemoryRecord, Storage, StorageMode};
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

fn embedded_config(path: &std::path::Path) -> CerebroConfig {
    let mut config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_path: Some(path.display().to_string()),
        ..CerebroConfig::default()
    };
    config.surreal.username = Some("root".to_string());
    config.surreal.password = Some(secrecy::SecretString::new(
        "secret".to_string().into_boxed_str(),
    ));
    config
}

#[tokio::test]
async fn embedded_storage_supports_crud() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let mut config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_path: Some(path.display().to_string()),
        ..CerebroConfig::default()
    };
    config.surreal.username = Some("root".to_string());
    config.surreal.password = Some(secrecy::SecretString::new(
        "secret".to_string().into_boxed_str(),
    ));

    let storage = storage_from_config(&config).await.expect("storage init");
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

    let config = CerebroConfig {
        storage_path: Some(path.display().to_string()),
        ..CerebroConfig::default()
    };

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

#[tokio::test]
async fn embedded_storage_persists_committed_record_across_clean_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let config = embedded_config(&path);

    let record = MemoryRecord::new(
        "restart-memory-1".to_string(),
        "shared".to_string(),
        "restart-topic".to_string(),
        json!({ "content": "persisted across clean restart" }),
    );

    {
        let storage = storage_from_config(&config).await.expect("storage init");
        storage
            .save(record.clone())
            .await
            .expect("save before restart");
        assert!(storage
            .get("restart-memory-1")
            .await
            .expect("get before restart")
            .is_some());
        drop(storage);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;

    let restarted = storage_from_config(&config)
        .await
        .expect("storage reinit after clean restart");
    restarted.ready().await.expect("ready after restart");

    let fetched = restarted
        .get("restart-memory-1")
        .await
        .expect("get after restart")
        .expect("record should persist after restart");
    assert_eq!(fetched.memory_id, record.memory_id);
    assert_eq!(fetched.scope, "shared");
    assert_eq!(fetched.topic_key, "restart-topic");

    let results = restarted
        .search(
            "persisted across clean restart",
            10,
            false,
            Some("shared"),
            Some("restart-topic"),
        )
        .await
        .expect("search after restart");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "restart-memory-1");
}

#[tokio::test]
async fn embedded_storage_recovers_committed_record_after_handle_drop() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("cerebro.db");

    let config = embedded_config(&path);

    {
        let storage = storage_from_config(&config).await.expect("storage init");
        storage
            .save(MemoryRecord::new(
                "crash-memory-1".to_string(),
                "shared".to_string(),
                "crash-topic".to_string(),
                json!({ "content": "committed before simulated crash" }),
            ))
            .await
            .expect("committed save before handle drop");
        drop(storage);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;

    let recovered = storage_from_config(&config)
        .await
        .expect("storage reinit after handle drop");
    recovered.ready().await.expect("ready after handle drop");

    let fetched = recovered
        .get("crash-memory-1")
        .await
        .expect("get after handle drop")
        .expect("committed record should recover after handle drop");
    assert_eq!(fetched.topic_key, "crash-topic");
    assert_eq!(fetched.summary, "committed before simulated crash");
}
