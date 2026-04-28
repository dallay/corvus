use super::traits::{Tool, ToolResult};
use crate::search::content::{search_workspace, SearchLimits, SharedSearchRequest, DEFAULT_LIMITS};
use crate::search::discovery::validate_search_root;
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

const DEFAULT_HEAD_LIMIT: usize = 100;
const MAX_HEAD_LIMIT: usize = 500;
const DEFAULT_COUNT_LIMIT: usize = 1_000;

pub struct GrepTool {
    security: Arc<SecurityPolicy>,
}

impl GrepTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrepOutputMode {
    Content,
    FilesWithMatches,
    Count,
}

impl GrepOutputMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::FilesWithMatches => "files_with_matches",
            Self::Count => "count",
        }
    }
}

#[derive(Debug, Clone)]
struct GrepRequest {
    pattern: String,
    path: String,
    glob: Option<String>,
    output_mode: GrepOutputMode,
    after: usize,
    before: usize,
    context: usize,
    offset: usize,
    head_limit: usize,
    case_insensitive: bool,
    multiline: bool,
}

impl GrepRequest {
    fn from_args(args: &Value) -> Result<Self, String> {
        const ALLOWED_KEYS: &[&str] = &[
            "pattern",
            "path",
            "glob",
            "output_mode",
            "-A",
            "-B",
            "-C",
            "context",
            "offset",
            "head_limit",
            "case_insensitive",
            "multiline",
        ];

        let object = args
            .as_object()
            .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;

        if let Some(unexpected) = object
            .keys()
            .find(|key| !ALLOWED_KEYS.contains(&key.as_str()))
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

        let output_mode = match object
            .get("output_mode")
            .and_then(Value::as_str)
            .unwrap_or("content")
        {
            "content" => GrepOutputMode::Content,
            "files_with_matches" => GrepOutputMode::FilesWithMatches,
            "count" => GrepOutputMode::Count,
            other => return Err(format!("Invalid output_mode: {other}")),
        };

        let after = parse_non_negative(object.get("-A"), "-A", 0)?;
        let before = parse_non_negative(object.get("-B"), "-B", 0)?;
        let context = parse_non_negative(object.get("-C"), "-C", 0)?.max(parse_non_negative(
            object.get("context"),
            "context",
            0,
        )?);
        let has_context = after > 0 || before > 0 || context > 0;
        if output_mode != GrepOutputMode::Content && has_context {
            return Err(
                "content context fields are only valid with output_mode=content".to_string(),
            );
        }

        Ok(Self {
            pattern,
            path: object
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or(".")
                .to_string(),
            glob: object
                .get("glob")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            output_mode,
            after,
            before,
            context,
            offset: parse_non_negative(object.get("offset"), "offset", 0)?,
            head_limit: parse_non_negative(
                object.get("head_limit"),
                "head_limit",
                DEFAULT_HEAD_LIMIT,
            )?
            .min(MAX_HEAD_LIMIT),
            case_insensitive: parse_bool(
                object.get("case_insensitive"),
                "case_insensitive",
                false,
            )?,
            multiline: parse_bool(object.get("multiline"), "multiline", false)?,
        })
    }

    fn context_lines(&self) -> usize {
        self.context.max(self.before).max(self.after)
    }
}

fn parse_non_negative(value: Option<&Value>, name: &str, default: usize) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(default);
    };

    let parsed = value
        .as_i64()
        .ok_or_else(|| format!("Parameter '{name}' must be an integer"))?;
    if parsed < 0 {
        return Err(format!("Parameter '{name}' must not be negative"));
    }

    usize::try_from(parsed).map_err(|_| format!("Parameter '{name}' must not be negative"))
}

fn parse_bool(value: Option<&Value>, name: &str, default: bool) -> Result<bool, String> {
    match value {
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("Parameter '{name}' must be a boolean")),
        None => Ok(default),
    }
}

fn build_regex(request: &GrepRequest) -> Result<Regex, String> {
    let mut pattern = request.pattern.clone();
    if request.case_insensitive {
        pattern = format!("(?i){pattern}");
    }
    if request.multiline {
        pattern = format!("(?m){pattern}");
    }

    Regex::new(&pattern).map_err(|error| error.to_string())
}

fn unique_filenames(matches: &[crate::search::content::SearchMatch]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut files = Vec::new();
    for matched in matches {
        if seen.insert(matched.file.clone()) {
            files.push(matched.file.clone());
        }
    }
    files
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

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Claude-style parity content search backed by Corvus native code_search internals."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string" },
                "glob": { "type": "string" },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"]
                },
                "-A": { "type": "integer", "minimum": 0 },
                "-B": { "type": "integer", "minimum": 0 },
                "-C": { "type": "integer", "minimum": 0 },
                "context": { "type": "integer", "minimum": 0 },
                "offset": { "type": "integer", "minimum": 0 },
                "head_limit": { "type": "integer", "minimum": 0 },
                "case_insensitive": { "type": "boolean" },
                "multiline": { "type": "boolean" }
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
            aliases: vec!["grep".to_string()],
        }
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let request = match GrepRequest::from_args(&args) {
            Ok(request) => request,
            Err(error) => return Ok(tool_error(error)),
        };
        let regex = match build_regex(&request) {
            Ok(regex) => regex,
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

        let (search_path, exact_file) = match normalize_scope(&self.security, &request.path) {
            Ok(scope) => scope,
            Err(error) => return Ok(tool_error(error.to_string())),
        };

        if let Err(error) = validate_search_root(&self.security, &search_path) {
            return Ok(tool_error(error.to_string()));
        }

        let mut include = request.glob.iter().cloned().collect::<Vec<_>>();
        if let Some(file_name) = exact_file {
            include.push(file_name);
        }

        let max_results = match request.output_mode {
            GrepOutputMode::Count => DEFAULT_COUNT_LIMIT,
            GrepOutputMode::FilesWithMatches => {
                request.head_limit.saturating_add(request.offset).max(1)
            }
            GrepOutputMode::Content => request.head_limit.saturating_add(request.offset).max(1),
        };

        let outcome = search_workspace(
            self.security.clone(),
            SharedSearchRequest {
                pattern: request.pattern.clone(),
                path: search_path,
                include,
                exclude: Vec::new(),
                is_regex: true,
                case_sensitive: !request.case_insensitive,
                max_results,
                context_lines: request.context_lines(),
                whole_word: false,
            },
            regex,
            SearchLimits {
                max_output_bytes: DEFAULT_LIMITS.max_output_bytes,
                ..DEFAULT_LIMITS
            },
        );

        if let Some(error) = outcome.fatal_error.clone() {
            return Ok(tool_error(error));
        }

        let filenames = unique_filenames(&outcome.matches);
        match request.output_mode {
            GrepOutputMode::FilesWithMatches => {
                let selected = filenames
                    .into_iter()
                    .skip(request.offset)
                    .take(request.head_limit)
                    .collect::<Vec<_>>();
                Ok(ToolResult {
                    success: true,
                    output: selected.join("\n"),
                    error: None,
                    structured: Some(json!({
                        "mode": request.output_mode.as_str(),
                        "numFiles": selected.len(),
                        "filenames": selected,
                        "appliedLimit": request.head_limit,
                        "appliedOffset": request.offset,
                    })),
                })
            }
            GrepOutputMode::Count => Ok(ToolResult {
                success: true,
                output: outcome.stats.total_matches.to_string(),
                error: None,
                structured: Some(json!({
                    "mode": request.output_mode.as_str(),
                    "numFiles": filenames.len(),
                    "filenames": filenames,
                    "numMatches": outcome.stats.total_matches,
                    "appliedLimit": request.head_limit,
                    "appliedOffset": request.offset,
                })),
            }),
            GrepOutputMode::Content => {
                let selected = outcome
                    .matches
                    .into_iter()
                    .skip(request.offset)
                    .take(request.head_limit)
                    .collect::<Vec<_>>();
                let content_lines = selected
                    .iter()
                    .map(|matched| {
                        format!(
                            "{}:{}:{}: {}",
                            matched.file, matched.line, matched.column, matched.content
                        )
                    })
                    .collect::<Vec<_>>();
                let selected_files = unique_filenames(&selected);
                Ok(ToolResult {
                    success: true,
                    output: content_lines.join("\n"),
                    error: None,
                    structured: Some(json!({
                        "mode": request.output_mode.as_str(),
                        "numFiles": selected_files.len(),
                        "filenames": selected_files,
                        "content": content_lines.join("\n"),
                        "numLines": content_lines.len(),
                        "appliedLimit": request.head_limit,
                        "appliedOffset": request.offset,
                    })),
                })
            }
        }
    }
}

fn normalize_scope(
    security: &SecurityPolicy,
    raw_path: &str,
) -> anyhow::Result<(String, Option<String>)> {
    if raw_path == "." {
        return Ok((".".to_string(), None));
    }

    let full_path = security.workspace_dir.join(raw_path);
    let resolved = full_path.canonicalize()?;
    if !security.is_resolved_path_allowed(&resolved) {
        anyhow::bail!("Resolved path escapes workspace: {}", resolved.display());
    }

    let metadata = std::fs::metadata(&resolved)?;
    if metadata.is_dir() {
        return Ok((raw_path.to_string(), None));
    }

    let parent = Path::new(raw_path)
        .parent()
        .and_then(|path| path.to_str())
        .filter(|path| !path.is_empty())
        .unwrap_or(".")
        .to_string();
    let file_name = Path::new(raw_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Search path is not valid UTF-8"))?
        .to_string();
    Ok((parent, Some(file_name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use std::fs;
    use tempfile::TempDir;

    fn test_security(workspace: &TempDir) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace.path().to_path_buf(),
            ..SecurityPolicy::default()
        })
    }

    #[test]
    fn grep_name_and_schema() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(&dir));
        assert_eq!(tool.name(), "Grep");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["output_mode"].is_object());
    }

    #[test]
    fn grep_spec_exposes_snake_case_alias() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(&dir));
        let spec = tool.spec();
        assert_eq!(spec.name, "Grep");
        assert_eq!(spec.aliases, vec!["grep"]);
    }

    #[tokio::test]
    async fn grep_returns_files_with_matches_in_deterministic_contract() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/app.ts"),
            "const SearchClient = true;\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("src/lib.ts"),
            "export const SearchClient = true;\n",
        )
        .unwrap();

        let tool = GrepTool::new(test_security(&dir));
        let result = tool
            .execute(json!({
                "pattern": "SearchClient",
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert_eq!(structured["mode"], "files_with_matches");
        assert_eq!(structured["filenames"], json!(["src/app.ts", "src/lib.ts"]));
    }

    #[tokio::test]
    async fn grep_rejects_invalid_output_mode_combinations() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(&dir));
        let result = tool
            .execute(json!({
                "pattern": "needle",
                "output_mode": "count",
                "-A": 2
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("content context fields are only valid"));
    }

    #[tokio::test]
    async fn grep_cannot_search_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "token", "path": "/etc"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap_or_default().contains("not allowed"));
    }

    #[tokio::test]
    async fn grep_preserves_zero_match_count_success() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("readme.md"), "hello\n").unwrap();
        let tool = GrepTool::new(test_security(&dir));
        let result = tool
            .execute(json!({
                "pattern": "pattern_that_does_not_exist_536",
                "output_mode": "count"
            }))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert_eq!(structured["numMatches"], 0);
        assert_eq!(structured["numFiles"], 0);
        assert_eq!(structured["mode"], "count");
    }
}
