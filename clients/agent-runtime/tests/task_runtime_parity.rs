use corvus::memory::{Memory, SqliteMemory, TaskPriority, TaskStatus};
use corvus::security::{SecurityPolicy, ToolOperation};
use corvus::tasks::{TaskCreateRequest, TaskListRequest, TaskService, TaskUpdateRequest};
use corvus::tools::{
    TaskCreateTool, TaskGetTool, TaskListTool, TaskStopTool, TaskUpdateTool, Tool,
};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::default())
}

#[tokio::test]
async fn task_service_applies_defaults_and_enforces_lifecycle_rules() {
    let tmp = TempDir::new().unwrap();
    let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(tmp.path()).unwrap());
    memory
        .upsert_session("session-123", Some("scope-a"))
        .await
        .unwrap();

    let service = TaskService::new(Arc::clone(&memory));
    let denied = service
        .create_task(TaskCreateRequest {
            title: "Denied scoped task".into(),
            description: None,
            priority: None,
            session_id: Some("session-123".into()),
            caller_scope_key: None,
        })
        .await
        .unwrap_err();
    assert!(denied.to_string().contains("permission denied"));

    let created = service
        .create_task(TaskCreateRequest {
            title: "Review parity slice".into(),
            description: None,
            priority: None,
            session_id: Some("session-123".into()),
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();

    assert_eq!(created.status, TaskStatus::Pending);
    assert_eq!(created.priority, TaskPriority::Medium);
    assert_eq!(created.session_id.as_deref(), Some("session-123"));

    let in_progress = service
        .update_task(TaskUpdateRequest {
            id: created.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::InProgress),
            session_id: None,
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();
    assert_eq!(in_progress.status, TaskStatus::InProgress);

    let completed = service
        .update_task(TaskUpdateRequest {
            id: created.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::Completed),
            session_id: None,
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();
    assert_eq!(completed.status, TaskStatus::Completed);

    let stop_error = service
        .stop_task(&created.id, Some("scope-a"))
        .await
        .unwrap_err();
    assert!(stop_error.to_string().contains("completed"));

    let reopen_error = service
        .update_task(TaskUpdateRequest {
            id: completed.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::Pending),
            session_id: None,
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap_err();
    assert!(reopen_error.to_string().contains("terminal"));

    let session_mutation_error = service
        .update_task(TaskUpdateRequest {
            id: created.id,
            title: None,
            description: None,
            priority: None,
            status: None,
            session_id: Some("other-session".into()),
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap_err();
    assert!(session_mutation_error.to_string().contains("session_id"));
}

#[tokio::test]
async fn task_service_allows_stop_for_in_progress_task() {
    let tmp = TempDir::new().unwrap();
    let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(tmp.path()).unwrap());
    let service = TaskService::new(Arc::clone(&memory));

    let created = service
        .create_task(TaskCreateRequest {
            title: "Stop me".into(),
            description: None,
            priority: None,
            session_id: None,
            caller_scope_key: None,
        })
        .await
        .unwrap();
    service
        .update_task(TaskUpdateRequest {
            id: created.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::InProgress),
            session_id: None,
            caller_scope_key: None,
        })
        .await
        .unwrap();

    let stopped = service.stop_task(&created.id, None).await.unwrap();
    assert_eq!(stopped.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn task_tools_validate_inputs_and_return_structured_payloads() {
    let tmp = TempDir::new().unwrap();
    let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(tmp.path()).unwrap());
    let service = Arc::new(TaskService::new(Arc::clone(&memory)));

    let create_tool = TaskCreateTool::new(test_security(), Arc::clone(&service));
    let get_tool = TaskGetTool::new(test_security(), Arc::clone(&service));
    let list_tool = TaskListTool::new(test_security(), Arc::clone(&service));
    let update_tool = TaskUpdateTool::new(test_security(), Arc::clone(&service));
    let stop_tool = TaskStopTool::new(test_security(), Arc::clone(&service));

    assert!(update_tool.parameters_schema()["properties"]
        .get("session_id")
        .is_none());

    let invalid_create = create_tool.execute(json!({"title": "   "})).await.unwrap();
    assert!(!invalid_create.success);
    assert!(invalid_create.error.unwrap_or_default().contains("title"));

    let invalid_priority_create = create_tool
        .execute(json!({"title": "Bad priority", "priority": "urgent"}))
        .await
        .unwrap();
    assert!(!invalid_priority_create.success);
    assert!(invalid_priority_create
        .error
        .unwrap_or_default()
        .contains("priority must be one of: low, medium, high"));

    let unsupported_feature_create = create_tool
        .execute(json!({"title": "Scoped task", "subtasks": []}))
        .await
        .unwrap();
    assert!(!unsupported_feature_create.success);
    assert!(unsupported_feature_create
        .error
        .unwrap_or_default()
        .contains("Unknown parameter: subtasks"));

    let unsupported_dependency_create = create_tool
        .execute(json!({"title": "Scoped task", "dependencies": ["task-1"]}))
        .await
        .unwrap();
    assert!(!unsupported_dependency_create.success);
    assert!(unsupported_dependency_create
        .error
        .unwrap_or_default()
        .contains("Unknown parameter: dependencies"));

    let denied_scoped_create = create_tool
        .execute(json!({"title": "Scoped task", "session_id": "session-123"}))
        .await
        .unwrap();
    assert!(!denied_scoped_create.success);
    assert!(denied_scoped_create
        .error
        .unwrap_or_default()
        .contains("permission denied"));

    let created = create_tool
        .execute(json!({"title": "Review parity slice", "priority": "high"}))
        .await
        .unwrap();
    assert!(created.success);
    let created_task = created.structured.unwrap()["task"].clone();
    let created_id = created_task["id"].as_str().unwrap().to_string();
    assert_eq!(created_task["priority"], "high");

    let fetched = get_tool.execute(json!({"id": created_id})).await.unwrap();
    assert!(fetched.success);
    assert_eq!(
        fetched.structured.unwrap()["task"]["id"],
        created_task["id"]
    );

    let invalid_get = get_tool.execute(json!({"id": "not-a-uuid"})).await.unwrap();
    assert!(!invalid_get.success);

    let missing_get = get_tool
        .execute(json!({"id": "11111111-1111-4111-8111-111111111111"}))
        .await
        .unwrap();
    assert!(!missing_get.success);

    let invalid_list = list_tool
        .execute(json!({"status": "paused", "limit": 0, "offset": -1}))
        .await
        .unwrap();
    assert!(!invalid_list.success);

    let invalid_update = update_tool
        .execute(json!({"id": created_id, "session_id": "other-session"}))
        .await
        .unwrap();
    assert!(!invalid_update.success);
    assert!(invalid_update
        .error
        .unwrap_or_default()
        .contains("session_id"));

    let cancel_via_update = update_tool
        .execute(json!({"id": created_task["id"], "status": "cancelled"}))
        .await
        .unwrap();
    assert!(!cancel_via_update.success);
    assert!(cancel_via_update
        .error
        .unwrap_or_default()
        .contains("TaskStop"));

    let stopped = stop_tool
        .execute(json!({"id": created_task["id"]}))
        .await
        .unwrap();
    assert!(stopped.success);

    let repeated_stop = stop_tool
        .execute(json!({"id": created_task["id"]}))
        .await
        .unwrap();
    assert!(!repeated_stop.success);
    assert!(repeated_stop
        .error
        .unwrap_or_default()
        .contains("cancelled"));
}

#[tokio::test]
async fn task_list_tool_supports_filtering_and_pagination_basics() {
    let tmp = TempDir::new().unwrap();
    let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(tmp.path()).unwrap());
    memory
        .upsert_session("session-123", Some("scope-a"))
        .await
        .unwrap();
    let service = Arc::new(TaskService::new(Arc::clone(&memory)));

    let first = service
        .create_task(TaskCreateRequest {
            title: "Newest in progress".into(),
            description: None,
            priority: Some(TaskPriority::High),
            session_id: Some("session-123".into()),
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();
    service
        .update_task(TaskUpdateRequest {
            id: first.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::InProgress),
            session_id: None,
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();

    let second = service
        .create_task(TaskCreateRequest {
            title: "Older in progress".into(),
            description: None,
            priority: Some(TaskPriority::High),
            session_id: Some("session-123".into()),
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();
    service
        .update_task(TaskUpdateRequest {
            id: second.id.clone(),
            title: None,
            description: None,
            priority: None,
            status: Some(TaskStatus::InProgress),
            session_id: None,
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();

    service
        .create_task(TaskCreateRequest {
            title: "Pending other session".into(),
            description: None,
            priority: Some(TaskPriority::Low),
            session_id: None,
            caller_scope_key: None,
        })
        .await
        .unwrap();

    let list_tool = TaskListTool::new(test_security(), Arc::clone(&service));
    let denied_result = list_tool
        .execute(json!({
            "status": "in_progress",
            "session_id": "session-123",
            "limit": 1,
            "offset": 0
        }))
        .await
        .unwrap();
    assert!(!denied_result.success);
    assert!(denied_result
        .error
        .unwrap_or_default()
        .contains("permission denied"));

    let page = service
        .list_tasks(TaskListRequest {
            status: Some(TaskStatus::InProgress),
            priority: None,
            session_id: Some("session-123".into()),
            limit: Some(1),
            offset: Some(1),
            caller_scope_key: Some("scope-a".into()),
        })
        .await
        .unwrap();
    assert_eq!(page.tasks.len(), 1);
    assert_eq!(page.applied_limit, 1);
    assert_eq!(page.applied_offset, 1);
    assert!(!page.has_more);
}

#[test]
fn task_tools_should_use_security_operation_tiers() {
    assert!(test_security()
        .enforce_tool_operation(ToolOperation::Read, "TaskList")
        .is_ok());
}
