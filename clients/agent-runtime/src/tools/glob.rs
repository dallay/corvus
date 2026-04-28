use super::traits::{Tool, ToolResult};
use crate::search::discovery::{discover_metadata_files_with_stats, DiscoveryRules};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

const MAX_RESULTS: usize = 1_000;

pub struct GlobTool {
    security: Arc<SecurityPolicy>,
}

impl GlobTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[derive(Debug, Clone)]
struct GlobRequest {
    pattern: String,
    path: String,
}

impl GlobRequest {
    fn from_args(args: &Value) -> Result<Self, String> {
        let object = args
            .as_object()
            .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;

        if let Some(unexpected) = object
            .keys()
            .find(|key| !matches!(key.as_str(), "pattern" | "path"))
        {
            return Err(format!("Unknown parameter: {unexpected}"));
        }

        let pattern = object
            .get("pattern")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing required parameter: pattern".to_string())?
            .trim()
            .to_string();
        if pattern.is_empty() {
            return Err("Pattern must not be empty".to_string());
        }

        let path = object
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();

        Ok(Self { pattern, path })
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Claude-style parity file pattern search backed by Corvus workspace discovery internals."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match against workspace-relative files."
                },
                "path": {
                    "type": "string",
                    "description": "Optional workspace-relative directory scope. Defaults to the workspace root."
                }
            },
            "required": ["pattern"]
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: vec!["glob".to_string()],
        }
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let request = match GlobRequest::from_args(&args) {
            Ok(request) => request,
            Err(error) => return Ok(tool_error(error)),
        };

        if self.security.is_rate_limited() {
            return Ok(tool_error(
                "Rate limit exceeded: too many actions in the last hour".to_string(),
            ));
        }

        if !self.security.is_path_allowed(&request.path) {
            return Ok(tool_error(format!(
                "Path not allowed by security policy: {}",
                request.path
            )));
        }

        if !self.security.record_action() {
            return Ok(tool_error(
                "Rate limit exceeded: action budget exhausted".to_string(),
            ));
        }

        let start = Instant::now();
        let discovered = match discover_metadata_files_with_stats(
            &self.security,
            &request.path,
            &request.pattern,
            DiscoveryRules {
                max_files: Some(MAX_RESULTS),
                ..DiscoveryRules::default()
            },
        ) {
            Ok(discovered) => discovered,
            Err(error) => return Ok(tool_error(error.to_string())),
        };

        let filenames = discovered
            .files
            .into_iter()
            .map(|file| file.relative_path)
            .collect::<Vec<_>>();
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let structured = json!({
            "filenames": filenames,
            "durationMs": duration_ms,
            "numFiles": filenames.len(),
            "truncated": discovered.hit_max_files,
        });

        Ok(ToolResult {
            success: true,
            output: filenames.join("\n"),
            error: None,
            structured: Some(structured),
        })
    }
}

fn tool_error(error: String) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(error.clone()),
        structured: Some(json!({
            "error": {
                "message": error,
            }
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_security(workspace: &TempDir) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace.path().to_path_buf(),
            ..SecurityPolicy::default()
        })
    }

    #[test]
    fn glob_name_and_schema() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(&dir));
        assert_eq!(tool.name(), "Glob");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["path"].is_object());
    }

    #[test]
    fn glob_spec_exposes_snake_case_alias() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(&dir));
        let spec = tool.spec();
        assert_eq!(spec.name, "Glob");
        assert_eq!(spec.aliases, vec!["glob"]);
    }

    #[tokio::test]
    async fn glob_returns_workspace_relative_matches() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/lib")).unwrap();
        fs::write(dir.path().join("src/main.ts"), "console.log('main');\n").unwrap();
        sleep(Duration::from_millis(10));
        fs::write(
            dir.path().join("src/lib/util.ts"),
            "export const util = true;\n",
        )
        .unwrap();

        let tool = GlobTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "src/**/*.ts"}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert_eq!(structured["numFiles"], 2);
        assert_eq!(
            structured["filenames"],
            json!(["src/lib/util.ts", "src/main.ts"])
        );
        assert_eq!(structured["truncated"], false);
    }

    #[tokio::test]
    async fn glob_rejects_workspace_escape() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_security(&dir));

        let result = tool
            .execute(json!({"pattern": "**/*.rs", "path": "../.."}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap_or_default().contains("not allowed"));
    }

    #[tokio::test]
    async fn glob_ordering_is_stable_for_unchanged_workspace() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/a.ts"), "a\n").unwrap();
        sleep(Duration::from_millis(10));
        fs::write(dir.path().join("src/b.ts"), "b\n").unwrap();

        let tool = GlobTool::new(test_security(&dir));
        let first = tool
            .execute(json!({"pattern": "src/**/*.ts"}))
            .await
            .unwrap()
            .structured
            .unwrap();
        let second = tool
            .execute(json!({"pattern": "src/**/*.ts"}))
            .await
            .unwrap()
            .structured
            .unwrap();

        assert_eq!(first["filenames"], second["filenames"]);
    }

    #[tokio::test]
    async fn glob_returns_most_recent_matches_first() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/older.ts"), "older\n").unwrap();
        sleep(Duration::from_millis(10));
        fs::write(dir.path().join("src/newer.ts"), "newer\n").unwrap();

        let tool = GlobTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "src/**/*.ts"}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert_eq!(
            result.structured.unwrap()["filenames"],
            json!(["src/newer.ts", "src/older.ts"])
        );
    }
}
