use cerebro::{CerebroConfig, StorageMode};
use secrecy::SecretString;
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

mod helpers;

/// Integration test validating the complete backup and restore cycle.
///
/// This test follows the documented cold backup procedure:
/// 1. Setup: Start Cerebro with embedded SurrealDB, write test data
/// 2. Backup: Gracefully shutdown, copy storage directory
/// 3. Restore: Clear original storage, copy backup back, restart
/// 4. Verify: Check readiness, validate data integrity
#[tokio::test]
async fn backup_restore_preserves_data() {
    // ========== SETUP PHASE ==========
    // Create temporary directories for storage and backup
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let storage_path = temp_dir.path().join("cerebro.db");
    let backup_path = temp_dir.path().join("backup");

    // Configure Cerebro with embedded SurrealDB
    let config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_path: Some(storage_path.display().to_string()),
        auth_token: Some(SecretString::new("test-token".to_string().into_boxed_str())),
        surreal: cerebro::SurrealConfig {
            username: Some("root".to_string()),
            password: Some(SecretString::new(
                "test-password".to_string().into_boxed_str(),
            )),
            namespace: "cerebro".to_string(),
            database: "cerebro".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    // Start Cerebro server
    let (service, shutdown_tx, base_url) = helpers::start_cerebro_server(config.clone())
        .await
        .expect("failed to start cerebro server");

    // Wait for service to become ready
    helpers::wait_for_ready(&base_url, 10)
        .await
        .expect("service did not become ready");

    // Write test data via MCP
    let memory_ids = helpers::create_test_memories(&base_url, "test-token", 10)
        .await
        .expect("failed to create test memories");

    // Verify data exists via mem_stats
    let client = reqwest::Client::new();
    let stats_request = json!({
        "jsonrpc": "2.0",
        "id": "stats-1",
        "method": "tools/call",
        "params": {
            "name": "mem_stats",
            "arguments": {
                "input": {}
            }
        }
    });

    let stats_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Authorization", "Bearer test-token")
        .json(&stats_request)
        .send()
        .await
        .expect("mem_stats request failed");

    let stats_body: serde_json::Value = stats_resp.json().await.expect("failed to parse stats");
    let original_count = stats_body["result"]["output"]["memory_count"]
        .as_u64()
        .expect("failed to parse memory count") as usize;

    assert_eq!(
        original_count, 10,
        "expected 10 memories before backup, got {}",
        original_count
    );

    // Verify specific test data via mem_search
    let search_request = json!({
        "jsonrpc": "2.0",
        "id": "search-1",
        "method": "tools/call",
        "params": {
            "name": "mem_search",
            "arguments": {
                "input": {
                    "query": "test content",
                    "limit": 5
                }
            }
        }
    });

    let search_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Authorization", "Bearer test-token")
        .json(&search_request)
        .send()
        .await
        .expect("mem_search request failed");

    let search_body: serde_json::Value = search_resp.json().await.expect("failed to parse search");
    assert!(
        search_body["result"].is_object(),
        "mem_search should return results"
    );

    // ========== BACKUP PHASE ==========
    // Gracefully shutdown Cerebro
    let _ = shutdown_tx.send(true);
    
    // Drop service to ensure all handles are released
    drop(service);
    
    // Wait for RocksDB to release locks and flush buffers
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify storage directory exists
    assert!(
        storage_path.exists(),
        "storage directory should exist at {:?}",
        storage_path
    );

    // Copy storage directory to backup location
    let copy_options = fs_extra::dir::CopyOptions::new().content_only(true);
    fs_extra::dir::copy(&storage_path, &backup_path, &copy_options)
        .expect("failed to copy storage to backup");

    // Verify backup directory exists and contains data
    assert!(
        backup_path.exists(),
        "backup directory should exist at {:?}",
        backup_path
    );
    let backup_files = std::fs::read_dir(&backup_path)
        .expect("failed to read backup dir")
        .count();
    assert!(
        backup_files > 0,
        "backup directory should contain RocksDB files, found {} files",
        backup_files
    );

    // ========== RESTORE PHASE ==========
    // Clear original storage directory (simulating data loss)
    std::fs::remove_dir_all(&storage_path).expect("failed to remove original storage");

    // Copy backup back to storage location
    // backup_path contains the copied contents, we need to copy them back to storage_path
    std::fs::create_dir_all(&storage_path).expect("failed to create storage dir");
    let restore_options = fs_extra::dir::CopyOptions::new().content_only(true);
    fs_extra::dir::copy(&backup_path, &storage_path, &restore_options)
        .expect("failed to restore from backup");

    // Verify restored storage exists
    assert!(
        storage_path.exists(),
        "restored storage should exist at {:?}",
        storage_path
    );

    // Restart Cerebro with restored data
    let (_service, _shutdown_tx, base_url) = helpers::start_cerebro_server(config)
        .await
        .expect("failed to restart cerebro after restore");

    // ========== VERIFICATION PHASE ==========
    // Wait for service to become ready post-restore
    helpers::wait_for_ready(&base_url, 10)
        .await
        .expect("service did not become ready after restore");

    // Verify mem_stats shows original count
    let stats_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Authorization", "Bearer test-token")
        .json(&stats_request)
        .send()
        .await
        .expect("mem_stats request failed after restore");

    let stats_body: serde_json::Value = stats_resp.json().await.expect("failed to parse stats");
    let restored_count = stats_body["result"]["output"]["memory_count"]
        .as_u64()
        .expect("failed to parse restored memory count") as usize;

    assert_eq!(
        restored_count, original_count,
        "restored memory count should match original: expected {}, got {}",
        original_count, restored_count
    );

    // Verify mem_search returns original test data
    let search_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Authorization", "Bearer test-token")
        .json(&search_request)
        .send()
        .await
        .expect("mem_search request failed after restore");

    let search_body: serde_json::Value = search_resp.json().await.expect("failed to parse search");
    assert!(
        search_body["result"].is_object(),
        "mem_search should return results after restore"
    );

    // Verify specific memory IDs are present
    for memory_id in &memory_ids[0..3] {
        let get_request = json!({
            "jsonrpc": "2.0",
            "id": format!("get-{}", memory_id),
            "method": "tools/call",
            "params": {
                "name": "mem_get_observation",
                "arguments": {
                    "input": {
                        "memory_id": memory_id
                    }
                }
            }
        });

        let get_resp = client
            .post(format!("{}/mcp", base_url))
            .header("Authorization", "Bearer test-token")
            .json(&get_request)
            .send()
            .await
            .expect("mem_get_observation request failed after restore");

        let get_body: serde_json::Value = get_resp.json().await.expect("failed to parse get");
        assert!(
            get_body["result"]["output"].is_object(),
            "memory {} should exist after restore",
            memory_id
        );
    }

    // Test passes - cleanup happens automatically via TempDir Drop
}
