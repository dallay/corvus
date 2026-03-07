use corvus::conductor::workspace::{sanitize_workspace_leaf, WorkspaceManager};
use corvus::conductor::TaskId;

#[test]
fn concurrent_tasks_get_unique_workspaces() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manager = WorkspaceManager::new(temp.path().join("workspaces"));

    let t1 = manager
        .create_workspace(&TaskId::new("task_alpha").expect("task id"), None)
        .expect("workspace 1");
    let t2 = manager
        .create_workspace(&TaskId::new("task_beta").expect("task id"), None)
        .expect("workspace 2");

    assert_ne!(t1, t2);
    assert!(t1.exists());
    assert!(t2.exists());
}

#[test]
fn workspace_paths_are_sanitized_and_prevent_traversal() {
    let sanitized = sanitize_workspace_leaf("../prod-secrets").expect("sanitize");
    assert!(!sanitized.contains(".."));
    assert!(!sanitized.contains('/'));
}

#[test]
fn task_workspace_cannot_access_other_task_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manager = WorkspaceManager::new(temp.path().join("workspaces"));

    let t1 = manager
        .create_workspace(&TaskId::new("task_1").expect("task id"), None)
        .expect("workspace 1");
    let t2 = manager
        .create_workspace(&TaskId::new("task_2").expect("task id"), None)
        .expect("workspace 2");

    let t1_file = t1.join("artifact.txt");
    let t2_file = t2.join("artifact.txt");

    assert!(manager.is_within_workspace(&t1, &t1_file));
    assert!(!manager.is_within_workspace(&t1, &t2_file));
}
