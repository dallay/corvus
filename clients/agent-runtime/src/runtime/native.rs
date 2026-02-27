use super::traits::RuntimeAdapter;
use std::path::{Path, PathBuf};

/// Native runtime — full access, runs on Mac/Linux/Docker/Raspberry Pi
pub struct NativeRuntime;

impl NativeRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl RuntimeAdapter for NativeRuntime {
    fn name(&self) -> &str {
        "native"
    }

    fn has_shell_access(&self) -> bool {
        true
    }

    fn has_filesystem_access(&self) -> bool {
        true
    }

    fn storage_path(&self) -> PathBuf {
        directories::UserDirs::new().map_or_else(
            || PathBuf::from(".corvus"),
            |u| u.home_dir().join(".corvus"),
        )
    }

    fn supports_long_running(&self) -> bool {
        true
    }

    fn build_shell_command(
        &self,
        command: &str,
        workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        let mut process = tokio::process::Command::new("sh");
        process.arg("-c").arg(command).current_dir(workspace_dir);
        Ok(process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_name() {
        assert_eq!(NativeRuntime::new().name(), "native");
    }

    #[test]
    fn native_has_shell_access() {
        assert!(NativeRuntime::new().has_shell_access());
    }

    #[test]
    fn native_has_filesystem_access() {
        assert!(NativeRuntime::new().has_filesystem_access());
    }

    #[test]
    fn native_supports_long_running() {
        assert!(NativeRuntime::new().supports_long_running());
    }

    #[test]
    fn native_memory_budget_unlimited() {
        assert_eq!(NativeRuntime::new().memory_budget(), 0);
    }

    #[test]
    fn native_storage_path_contains_corvus() {
        let path = NativeRuntime::new().storage_path();
        assert!(path.to_string_lossy().contains("corvus"));
    }

    #[test]
    fn native_builds_shell_command() {
        let cwd = std::env::temp_dir();
        let command = NativeRuntime::new()
            .build_shell_command("echo hello", &cwd)
            .unwrap();
        let debug = format!("{command:?}");
        assert!(debug.contains("echo hello"));
    }

    #[tokio::test]
    async fn native_executes_failing_command() {
        let cwd = std::env::temp_dir();
        let mut command = NativeRuntime::new()
            .build_shell_command("exit 42", &cwd)
            .unwrap();

        let output = command.output().await.unwrap();
        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(42));
    }

    #[tokio::test]
    async fn native_executes_command_with_stderr() {
        let cwd = std::env::temp_dir();
        let mut command = NativeRuntime::new()
            .build_shell_command("echo 'error message' >&2", &cwd)
            .unwrap();

        let output = command.output().await.unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error message"));
    }

    #[tokio::test]
    async fn native_fails_with_invalid_directory() {
        let invalid_dir = Path::new("/nonexistent/directory/that/does/not/exist");
        let mut command = NativeRuntime::new()
            .build_shell_command("ls", invalid_dir)
            .unwrap();

        let result = command.output().await;
        assert!(result.is_err());
    }
}
