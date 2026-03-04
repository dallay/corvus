use super::traits::{Tool, ToolResult};
use crate::config::Config;
use crate::cron::{self, CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct CronAddTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl CronAddTool {
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    fn error_result(error: &str) -> ToolResult {
        ToolResult {
            success: false,
            output: String::new(),
            error: Some(error.to_string()),
        }
    }

    fn parse_schedule(args: &serde_json::Value) -> Result<Schedule, String> {
        let v = args.get("schedule").ok_or("Missing 'schedule' parameter")?;
        serde_json::from_value(v.clone()).map_err(|e| format!("Invalid schedule: {e}"))
    }

    fn parse_job_type(args: &serde_json::Value) -> Result<JobType, String> {
        match args.get("job_type").and_then(serde_json::Value::as_str) {
            Some("agent") => Ok(JobType::Agent),
            Some("shell") => Ok(JobType::Shell),
            Some(other) => Err(format!("Invalid job_type: {other}")),
            None => {
                if args.get("prompt").is_some() {
                    Ok(JobType::Agent)
                } else {
                    Ok(JobType::Shell)
                }
            }
        }
    }

    fn parse_delete_after_run(args: &serde_json::Value, schedule: &Schedule) -> bool {
        let default = matches!(schedule, Schedule::At { .. });
        args.get("delete_after_run")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    }

    fn parse_session_target(args: &serde_json::Value) -> anyhow::Result<SessionTarget> {
        match args.get("session_target") {
            Some(v) => Ok(serde_json::from_value(v.clone())?),
            None => Ok(SessionTarget::Isolated),
        }
        .map_err(|e: serde_json::Error| anyhow::anyhow!("Invalid session_target: {e}"))
    }

    fn parse_delivery(args: &serde_json::Value) -> anyhow::Result<Option<DeliveryConfig>> {
        match args.get("delivery") {
            Some(v) => Ok(Some(serde_json::from_value(v.clone())?)),
            None => Ok(None),
        }
        .map_err(|e: serde_json::Error| anyhow::anyhow!("Invalid delivery config: {e}"))
    }

    fn handle_job_result(result: anyhow::Result<CronJob>) -> ToolResult {
        match result {
            Ok(job) => ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&json!({
                    "id": job.id,
                    "name": job.name,
                    "job_type": job.job_type,
                    "schedule": job.schedule,
                    "next_run": job.next_run,
                    "enabled": job.enabled
                }))
                .unwrap_or_default(),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }
}

#[async_trait]
impl Tool for CronAddTool {
    fn name(&self) -> &str {
        "cron_add"
    }

    fn description(&self) -> &str {
        "Create a scheduled cron job (shell or agent) with cron/at/every schedules"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "schedule": {
                    "type": "object",
                    "description": "Schedule object: {kind:'cron',expr,tz?} | {kind:'at',at} | {kind:'every',every_ms}"
                },
                "job_type": { "type": "string", "enum": ["shell", "agent"] },
                "command": { "type": "string" },
                "prompt": { "type": "string" },
                "session_target": { "type": "string", "enum": ["isolated", "main"] },
                "model": { "type": "string" },
                "delivery": { "type": "object" },
                "delete_after_run": { "type": "boolean" }
            },
            "required": ["schedule"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if !self.config.cron.enabled {
            return Ok(Self::error_result(
                "cron is disabled by config (cron.enabled=false)",
            ));
        }

        let schedule = match Self::parse_schedule(&args) {
            Ok(s) => s,
            Err(e) => return Ok(Self::error_result(&e)),
        };

        let name = args
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        let job_type = match Self::parse_job_type(&args) {
            Ok(jt) => jt,
            Err(e) => return Ok(Self::error_result(&e)),
        };

        let delete_after_run = Self::parse_delete_after_run(&args, &schedule);

        match job_type {
            JobType::Shell => {
                let command = match args.get("command").and_then(serde_json::Value::as_str) {
                    Some(cmd) if !cmd.trim().is_empty() => cmd,
                    _ => {
                        return Ok(Self::error_result("Missing 'command' for shell job"));
                    }
                };

                if !self.security.is_command_allowed(command) {
                    return Ok(Self::error_result(&format!(
                        "Command blocked by security policy: {command}"
                    )));
                }

                let result =
                    cron::add_shell_job(&self.config, name, schedule, command, delete_after_run);
                Ok(Self::handle_job_result(result))
            }
            JobType::Agent => {
                let prompt = match args.get("prompt").and_then(serde_json::Value::as_str) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => {
                        return Ok(Self::error_result("Missing 'prompt' for agent job"));
                    }
                };

                let session_target = match Self::parse_session_target(&args) {
                    Ok(st) => st,
                    Err(e) => return Ok(Self::error_result(&e.to_string())),
                };
                let model = args
                    .get("model")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let delivery = match Self::parse_delivery(&args) {
                    Ok(d) => d,
                    Err(e) => return Ok(Self::error_result(&e.to_string())),
                };

                let result = cron::add_agent_job(
                    &self.config,
                    name,
                    schedule,
                    prompt,
                    session_target,
                    model,
                    delivery,
                    delete_after_run,
                );

                Ok(Self::handle_job_result(result))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::security::AutonomyLevel;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Arc<Config> {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        Arc::new(config)
    }

    fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::from_config(
            &cfg.autonomy,
            &cfg.workspace_dir,
        ))
    }

    #[tokio::test]
    async fn adds_shell_job() {
        let tmp = TempDir::new().unwrap();
        let cfg = test_config(&tmp);
        let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));
        let result = tool
            .execute(json!({
                "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
                "job_type": "shell",
                "command": "echo ok"
            }))
            .await
            .unwrap();

        assert!(result.success, "{:?}", result.error);
        assert!(result.output.contains("next_run"));
    }

    #[tokio::test]
    async fn blocks_disallowed_shell_command() {
        let tmp = TempDir::new().unwrap();
        let mut config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        config.autonomy.allowed_commands = vec!["echo".into()];
        config.autonomy.level = AutonomyLevel::Supervised;
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        let cfg = Arc::new(config);
        let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

        let result = tool
            .execute(json!({
                "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
                "job_type": "shell",
                "command": "curl https://example.com"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("blocked by security policy"));
    }

    #[tokio::test]
    async fn rejects_invalid_schedule() {
        let tmp = TempDir::new().unwrap();
        let cfg = test_config(&tmp);
        let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

        let result = tool
            .execute(json!({
                "schedule": { "kind": "every", "every_ms": 0 },
                "job_type": "shell",
                "command": "echo nope"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("every_ms must be > 0"));
    }

    #[tokio::test]
    async fn agent_job_requires_prompt() {
        let tmp = TempDir::new().unwrap();
        let cfg = test_config(&tmp);
        let tool = CronAddTool::new(cfg.clone(), test_security(&cfg));

        let result = tool
            .execute(json!({
                "schedule": { "kind": "cron", "expr": "*/5 * * * *" },
                "job_type": "agent"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("Missing 'prompt'"));
    }
}
