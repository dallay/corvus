use super::traits::{Tool, ToolResult};
use crate::runtime::RuntimeAdapter;
use crate::security::policy::CommandRiskLevel;
use crate::security::{Sandbox, SecurityPolicy};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Maximum output size in bytes (1MB).
const MAX_OUTPUT_BYTES: usize = 1_048_576;
/// Environment variables safe to pass to shell commands.
/// Only functional variables are included — never API keys or secrets.
const SAFE_ENV_VARS: &[&str] = &[
    "PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR",
];

fn should_warn_for_noop_sandbox(
    security: &SecurityPolicy,
    sandbox: &dyn Sandbox,
    command: &str,
) -> bool {
    sandbox.name() == "none"
        && matches!(
            security.command_risk_level(command),
            CommandRiskLevel::Medium | CommandRiskLevel::High
        )
}

fn shell_execution_metadata(
    sandbox_name: &str,
    risk_level: CommandRiskLevel,
    approved: bool,
) -> serde_json::Value {
    json!({
        "sandbox_backend": sandbox_name,
        "risk_level": format!("{:?}", risk_level).to_lowercase(),
        "approved": approved,
    })
}

/// Shell command execution tool with sandboxing
pub struct ShellTool {
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    sandbox: Arc<dyn Sandbox>,
    timeout: Duration,
}

impl ShellTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        sandbox: Arc<dyn Sandbox>,
    ) -> Self {
        Self {
            security,
            runtime,
            sandbox,
            timeout: Duration::from_secs(60),
        }
    }

    #[cfg(test)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace directory"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "approved": {
                    "type": "boolean",
                    "description": "Set true to explicitly approve medium/high-risk commands in supervised mode",
                    "default": false
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;
        let approved = args
            .get("approved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
                structured: None,
            });
        }

        let risk_level = match self.security.validate_command_execution(command, approved) {
            Ok(risk) => risk,
            Err(reason) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(reason),
                    structured: None,
                });
            }
        };

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
                structured: None,
            });
        }

        // Execute with timeout to prevent hanging commands.
        // Build the command via the runtime adapter, then extract program+args
        // so we can apply OS-level sandbox wrapping on a std::process::Command.
        let tokio_cmd = match self
            .runtime
            .build_shell_command(command, &self.security.workspace_dir)
        {
            Ok(cmd) => cmd,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to build runtime command: {e}")),
                    structured: None,
                });
            }
        };

        // Build std::process::Command for env sanitization + sandbox wrapping.
        // Clear the environment to prevent leaking API keys and other secrets
        // (CWE-200), then re-add only safe, functional variables.
        let program = tokio_cmd.as_std().get_program().to_os_string();
        let args: Vec<_> = tokio_cmd
            .as_std()
            .get_args()
            .map(|a| a.to_os_string())
            .collect();
        let mut std_cmd = std::process::Command::new(&program);
        std_cmd.args(&args);
        std_cmd.current_dir(&self.security.workspace_dir);
        std_cmd.env_clear();

        for var in SAFE_ENV_VARS {
            if let Ok(val) = std::env::var(var) {
                std_cmd.env(var, val);
            }
        }

        // OS-level sandbox wrapping (defense in depth)
        if let Err(e) = self.sandbox.wrap_command(&mut std_cmd) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Sandbox wrapping failed: {e}")),
                structured: Some(shell_execution_metadata(
                    self.sandbox.name(),
                    risk_level,
                    approved,
                )),
            });
        }

        if should_warn_for_noop_sandbox(self.security.as_ref(), self.sandbox.as_ref(), command) {
            tracing::warn!(
                command = %command,
                "OS-level sandbox not active; running with application-layer policy only"
            );
        }

        // Convert to tokio for async execution
        let mut cmd = tokio::process::Command::from(std_cmd);
        let result = tokio::time::timeout(self.timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Truncate output to prevent OOM
                if stdout.len() > MAX_OUTPUT_BYTES {
                    stdout.truncate(stdout.floor_char_boundary(MAX_OUTPUT_BYTES));
                    stdout.push_str("\n... [output truncated at 1MB]");
                }
                if stderr.len() > MAX_OUTPUT_BYTES {
                    stderr.truncate(stderr.floor_char_boundary(MAX_OUTPUT_BYTES));
                    stderr.push_str("\n... [stderr truncated at 1MB]");
                }

                Ok(ToolResult {
                    success: output.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                    structured: Some(shell_execution_metadata(
                        self.sandbox.name(),
                        risk_level,
                        approved,
                    )),
                })
            }
            Ok(Err(e)) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to execute command: {e}")),
                structured: Some(shell_execution_metadata(
                    self.sandbox.name(),
                    risk_level,
                    approved,
                )),
            }),
            Err(_) => {
                // If it times out, tokio's child process is dropped and killed.
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!(
                        "Command timed out after {}ms and was killed.",
                        self.timeout.as_millis()
                    )),
                    structured: Some(shell_execution_metadata(
                        self.sandbox.name(),
                        risk_level,
                        approved,
                    )),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{NativeRuntime, RuntimeAdapter};
    use crate::security::{AutonomyLevel, NoopSandbox, Sandbox, SecurityPolicy};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn test_security(autonomy: AutonomyLevel) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        })
    }

    fn test_runtime() -> Arc<dyn RuntimeAdapter> {
        Arc::new(NativeRuntime::new())
    }

    fn test_sandbox() -> Arc<dyn Sandbox> {
        Arc::new(NoopSandbox)
    }

    struct MockSandbox {
        wrap_called: AtomicBool,
        fail: bool,
    }

    struct EnvInspectSandbox {
        saw_path: AtomicBool,
        saw_secret: AtomicBool,
    }

    impl MockSandbox {
        fn success() -> Arc<Self> {
            Arc::new(Self {
                wrap_called: AtomicBool::new(false),
                fail: false,
            })
        }

        fn failure() -> Arc<Self> {
            Arc::new(Self {
                wrap_called: AtomicBool::new(false),
                fail: true,
            })
        }
    }

    impl Sandbox for MockSandbox {
        fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
            self.wrap_called.store(true, Ordering::SeqCst);
            if self.fail {
                Err(std::io::Error::other("mock sandbox failure"))
            } else {
                Ok(())
            }
        }

        fn is_available(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            if self.fail {
                "mock-fail"
            } else {
                "mock"
            }
        }

        fn description(&self) -> &str {
            "Mock sandbox"
        }
    }

    impl Sandbox for EnvInspectSandbox {
        fn wrap_command(&self, cmd: &mut std::process::Command) -> std::io::Result<()> {
            for (key, value) in cmd.get_envs() {
                if key == "PATH" && value.is_some() {
                    self.saw_path.store(true, Ordering::SeqCst);
                }
                if key == "CORVUS_TEST_SECRET" && value.is_some() {
                    self.saw_secret.store(true, Ordering::SeqCst);
                }
            }
            Ok(())
        }

        fn is_available(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "inspect"
        }

        fn description(&self) -> &str {
            "Env inspect sandbox"
        }
    }

    #[test]
    fn shell_tool_name() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        assert_eq!(tool.name(), "shell");
    }

    #[test]
    fn shell_tool_description() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn shell_tool_schema_has_command() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["command"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("command")));
        assert!(schema["properties"]["approved"].is_object());
    }

    #[tokio::test]
    async fn shell_executes_allowed_command() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.trim().contains("hello"));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn shell_blocks_disallowed_command() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool.execute(json!({"command": "rm -rf /"})).await.unwrap();
        assert!(!result.success);
        let error = result.error.as_deref().unwrap_or("");
        assert!(error.contains("not allowed") || error.contains("high-risk"));
    }

    #[tokio::test]
    async fn shell_blocks_readonly() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::ReadOnly),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool.execute(json!({"command": "ls"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn shell_missing_command_param() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("command"));
    }

    #[tokio::test]
    async fn shell_wrong_type_param() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool.execute(json!({"command": 123})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shell_captures_exit_code() {
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            test_sandbox(),
        );
        let result = tool
            .execute(json!({"command": "ls /nonexistent_dir_xyz"}))
            .await
            .unwrap();
        assert!(!result.success);
    }

    fn test_security_with_env_cmd() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: std::env::temp_dir(),
            allowed_commands: vec!["env".into(), "echo".into()],
            ..SecurityPolicy::default()
        })
    }

    /// RAII guard that restores an environment variable to its original state on drop,
    /// ensuring cleanup even if the test panics.
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => std::env::set_var(self.key, val),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn shell_does_not_leak_api_key() {
        let _g1 = EnvGuard::set("API_KEY", "sk-test-secret-12345");
        let _g2 = EnvGuard::set("CORVUS_API_KEY", "sk-test-secret-67890");

        let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime(), test_sandbox());
        let result = tool.execute(json!({"command": "env"})).await.unwrap();
        assert!(result.success);
        assert!(
            !result.output.contains("sk-test-secret-12345"),
            "API_KEY leaked to shell command output"
        );
        assert!(
            !result.output.contains("sk-test-secret-67890"),
            "CORVUS_API_KEY leaked to shell command output"
        );
    }

    #[tokio::test]
    async fn shell_preserves_path_and_home() {
        let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime(), test_sandbox());

        let result = tool
            .execute(json!({"command": "echo $HOME"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            !result.output.trim().is_empty(),
            "HOME should be available in shell"
        );

        let result = tool
            .execute(json!({"command": "echo $PATH"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            !result.output.trim().is_empty(),
            "PATH should be available in shell"
        );
    }

    #[tokio::test]
    async fn shell_requires_approval_for_medium_risk_command() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_commands: vec!["touch".into()],
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });

        let tool = ShellTool::new(security.clone(), test_runtime(), test_sandbox());
        let denied = tool
            .execute(json!({"command": "touch corvus_shell_approval_test"}))
            .await
            .unwrap();
        assert!(!denied.success);
        assert!(denied
            .error
            .as_deref()
            .unwrap_or("")
            .contains("explicit approval"));

        let allowed = tool
            .execute(json!({
                "command": "touch corvus_shell_approval_test",
                "approved": true
            }))
            .await
            .unwrap();
        assert!(allowed.success);

        let _ = std::fs::remove_file(std::env::temp_dir().join("corvus_shell_approval_test"));
    }

    // ── §5.2 Shell timeout enforcement tests ─────────────────

    #[test]
    fn shell_output_limit_is_1mb() {
        assert_eq!(
            MAX_OUTPUT_BYTES, 1_048_576,
            "max output must be 1 MB to prevent OOM"
        );
    }

    // ── §5.3 Non-UTF8 binary output tests ────────────────────

    #[test]
    fn shell_safe_env_vars_excludes_secrets() {
        for var in SAFE_ENV_VARS {
            let lower = var.to_lowercase();
            assert!(
                !lower.contains("key") && !lower.contains("secret") && !lower.contains("token"),
                "SAFE_ENV_VARS must not include sensitive variable: {var}"
            );
        }
    }

    #[test]
    fn shell_safe_env_vars_includes_essentials() {
        assert!(
            SAFE_ENV_VARS.contains(&"PATH"),
            "PATH must be in safe env vars"
        );
        assert!(
            SAFE_ENV_VARS.contains(&"HOME"),
            "HOME must be in safe env vars"
        );
        assert!(
            SAFE_ENV_VARS.contains(&"TERM"),
            "TERM must be in safe env vars"
        );
    }

    #[tokio::test]
    async fn shell_blocks_rate_limited() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            max_actions_per_hour: 0,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });
        let tool = ShellTool::new(security, test_runtime(), test_sandbox());
        let result = tool.execute(json!({"command": "echo test"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("Rate limit"));
    }

    #[tokio::test]
    async fn shell_handles_timeout_gracefully() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: std::env::temp_dir(),
            allowed_commands: vec!["sleep".into()],
            ..SecurityPolicy::default()
        });

        let tool = ShellTool::new(security, test_runtime(), test_sandbox())
            .with_timeout(Duration::from_secs(1));

        // Sleep for 3 seconds to trigger the 1-second timeout
        let result = tool.execute(json!({"command": "sleep 3"})).await.unwrap();

        assert!(!result.success);
        let err = result.error.expect("Expected an error from timeout");
        assert!(err.contains("timed out after 1000ms"));
    }

    #[tokio::test]
    async fn shell_calls_wrap_command_on_injected_sandbox() {
        let sandbox = MockSandbox::success();
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            sandbox.clone(),
        );

        let result = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(sandbox.wrap_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn shell_returns_error_when_sandbox_wrap_fails() {
        let sandbox = MockSandbox::failure();
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            sandbox.clone(),
        );

        let result = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(sandbox.wrap_called.load(Ordering::SeqCst));
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Sandbox wrapping failed"));
    }

    #[tokio::test]
    async fn shell_wraps_after_env_sanitization() {
        let previous = std::env::var("CORVUS_TEST_SECRET").ok();
        unsafe {
            std::env::set_var("CORVUS_TEST_SECRET", "super-secret");
        }

        let sandbox = Arc::new(EnvInspectSandbox {
            saw_path: AtomicBool::new(false),
            saw_secret: AtomicBool::new(false),
        });
        let tool = ShellTool::new(
            test_security(AutonomyLevel::Supervised),
            test_runtime(),
            sandbox.clone(),
        );

        let result = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(sandbox.saw_path.load(Ordering::SeqCst));
        assert!(!sandbox.saw_secret.load(Ordering::SeqCst));

        match previous {
            Some(value) => unsafe { std::env::set_var("CORVUS_TEST_SECRET", value) },
            None => unsafe { std::env::remove_var("CORVUS_TEST_SECRET") },
        }
    }

    #[test]
    fn noop_sandbox_warning_helper_triggers_for_mutating_commands() {
        let security = SecurityPolicy {
            allowed_commands: vec!["touch".into()],
            ..SecurityPolicy::default()
        };

        assert!(should_warn_for_noop_sandbox(
            &security,
            &NoopSandbox,
            "touch file.txt"
        ));
    }

    #[test]
    fn noop_sandbox_warning_helper_skips_read_only_commands() {
        let security = SecurityPolicy::default();

        assert!(!should_warn_for_noop_sandbox(&security, &NoopSandbox, "ls"));
        assert!(!should_warn_for_noop_sandbox(
            &security,
            &NoopSandbox,
            "git status"
        ));
    }

    #[test]
    fn noop_sandbox_warning_helper_skips_real_sandbox() {
        let security = SecurityPolicy {
            allowed_commands: vec!["touch".into()],
            ..SecurityPolicy::default()
        };
        let sandbox = MockSandbox::success();

        assert!(!should_warn_for_noop_sandbox(
            &security,
            sandbox.as_ref(),
            "touch file.txt"
        ));
    }
}
