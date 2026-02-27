use corvus::runtime::RuntimeAdapter;
use corvus::tools::shell::ShellTool;
use corvus::tools::traits::Tool;
use corvus::security::{SecurityPolicy, AutonomyLevel};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use serde_json::json;

struct MockFailureRuntime;

impl RuntimeAdapter for MockFailureRuntime {
    fn name(&self) -> &str { "mock-failure" }
    fn has_shell_access(&self) -> bool { true }
    fn has_filesystem_access(&self) -> bool { true }
    fn storage_path(&self) -> PathBuf { PathBuf::from("/tmp") }
    fn supports_long_running(&self) -> bool { false }

    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        anyhow::bail!("Simulated build failure")
    }
}

#[tokio::test]
async fn test_shell_tool_handles_runtime_build_failure() {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    });
    let runtime = Arc::new(MockFailureRuntime);
    let tool = ShellTool::new(security, runtime);

    let result = tool.execute(json!({"command": "echo test"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Simulated build failure"));
}
