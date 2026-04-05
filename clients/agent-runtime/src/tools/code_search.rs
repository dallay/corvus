use super::traits::{Tool, ToolResult};
use crate::search::discovery::{
    discover_searchable_files_with_stats, is_binary_file, normalize_relative_path,
    validate_search_root, DiscoveryRules as SearchDiscoveryRules,
};
use crate::search::{CandidateCoverage, CandidateRequest, WorkspaceTrigramIndex};
use crate::security::SecurityPolicy;
use anyhow::Context;
use async_trait::async_trait;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
use std::fs;

const MAX_PATTERN_LENGTH: usize = 1000;
const DEFAULT_MAX_RESULTS: usize = 100;
const MAX_MAX_RESULTS: usize = 500;
const DEFAULT_CONTEXT_LINES: usize = 0;
const MAX_CONTEXT_LINES: usize = 5;
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_FILES_SCANNED: usize = 10_000;
const MAX_OUTPUT_BYTES: usize = 100 * 1024;
const MAX_MATCHES_PER_FILE: usize = 50;
const MAX_LINE_CONTENT_CHARS: usize = 500;
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Search for text or regex patterns across files in the workspace.
pub struct CodeSearchTool {
    security: Arc<SecurityPolicy>,
}

impl CodeSearchTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[derive(Debug, Clone)]
struct SearchParams {
    pattern: String,
    path: String,
    include: Vec<String>,
    exclude: Vec<String>,
    is_regex: bool,
    case_sensitive: bool,
    max_results: usize,
    context_lines: usize,
    whole_word: bool,
}

#[derive(Debug, Clone, Copy)]
struct SearchLimits {
    max_file_size_bytes: u64,
    max_files_scanned: usize,
    max_output_bytes: usize,
    max_matches_per_file: usize,
    timeout: Duration,
    max_line_content_chars: usize,
}

const DEFAULT_LIMITS: SearchLimits = SearchLimits {
    max_file_size_bytes: MAX_FILE_SIZE_BYTES,
    max_files_scanned: MAX_FILES_SCANNED,
    max_output_bytes: MAX_OUTPUT_BYTES,
    max_matches_per_file: MAX_MATCHES_PER_FILE,
    timeout: EXECUTION_TIMEOUT,
    max_line_content_chars: MAX_LINE_CONTENT_CHARS,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SearchMatch {
    file: String,
    line: usize,
    column: usize,
    content: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
    line_end: usize,
    column_end: usize,
    byte_start: usize,
    byte_end: usize,
    preview: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SearchStats {
    files_searched: usize,
    files_matched: usize,
    total_matches: usize,
    truncated: bool,
    duration_ms: u64,
}

#[derive(Debug, Clone)]
struct SearchOutcome {
    matches: Vec<SearchMatch>,
    stats: SearchStats,
    warnings: Vec<String>,
    fatal_error: Option<String>,
}

#[derive(Debug, Clone)]
struct VerificationInput {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct LineRange {
    start: usize,
    end: usize,
}

#[async_trait]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "code_search"
    }

    fn description(&self) -> &str {
        "Search for text or regex patterns across files in the workspace. Returns matching lines with file paths, line numbers, and optional context. Respects .gitignore. Use 'path' and 'include' to narrow scope for faster results."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The search pattern. Treated as a literal string by default; set 'is_regex' to true for regular expression matching. Max 1000 characters."
                },
                "path": {
                    "type": "string",
                    "description": "Subdirectory to scope the search, relative to workspace root. Defaults to workspace root if omitted. Must be a relative path."
                },
                "include": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Glob patterns for files to include (e.g., ['*.rs', '*.toml']). When omitted, all non-ignored files are searched."
                },
                "exclude": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Additional glob patterns to exclude beyond .gitignore rules (e.g., ['*.generated.rs', 'vendor/*'])."
                },
                "is_regex": {
                    "type": "boolean",
                    "description": "When true, 'pattern' is interpreted as a Rust regex (RE2-like semantics). When false (default), 'pattern' is matched as a literal string.",
                    "default": false
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search is case-sensitive. Defaults to true.",
                    "default": true
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of matches to return. Defaults to 100, maximum 500. Results are truncated with a warning when the cap is hit.",
                    "default": 100,
                    "minimum": 1,
                    "maximum": 500
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of lines of context to include before and after each match. Defaults to 0, maximum 5.",
                    "default": 0,
                    "minimum": 0,
                    "maximum": 5
                },
                "whole_word": {
                    "type": "boolean",
                    "description": "When true, only match whole words (the pattern is wrapped in word boundary anchors \\b). Defaults to false.",
                    "default": false
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let params = match SearchParams::from_args(&args) {
            Ok(params) => params,
            Err(error) => return Ok(tool_error(error)),
        };

        let compiled_pattern = match build_pattern_string(&params)
            .and_then(|pattern| Regex::new(&pattern).map_err(|error| error.to_string()))
        {
            Ok(regex) => regex,
            Err(error) => return Ok(tool_error(error)),
        };

        if self.security.is_rate_limited() {
            return Ok(tool_error(
                "Rate limit exceeded: too many actions in the last hour".to_string(),
            ));
        }

        if !self.security.is_path_allowed(&params.path) {
            return Ok(tool_error(format!(
                "Path not allowed by security policy: {}",
                params.path
            )));
        }

        if !self.security.record_action() {
            return Ok(tool_error(
                "Rate limit exceeded: action budget exhausted".to_string(),
            ));
        }

        let security = self.security.clone();
        let params_for_search = params.clone();

        if let Err(error) = validate_search_root(&self.security, &params.path) {
            return Ok(tool_error(error.to_string()));
        }

        let outcome = tokio::task::spawn_blocking(move || {
            search_workspace(
                security,
                params_for_search,
                compiled_pattern,
                DEFAULT_LIMITS,
            )
        })
        .await
        .map_err(|error| anyhow::anyhow!("code_search task failed: {error}"))?;

        Ok(build_result(
            outcome,
            &params,
            DEFAULT_LIMITS.max_output_bytes,
        ))
    }
}

impl SearchParams {
    fn from_args(args: &Value) -> Result<Self, String> {
        const ALLOWED_KEYS: &[&str] = &[
            "pattern",
            "path",
            "include",
            "exclude",
            "is_regex",
            "case_sensitive",
            "max_results",
            "context_lines",
            "whole_word",
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
            .to_string();

        if pattern.is_empty() {
            return Err("Pattern must not be empty".to_string());
        }

        if pattern.chars().count() > MAX_PATTERN_LENGTH {
            return Err(format!(
                "Pattern exceeds maximum length of {MAX_PATTERN_LENGTH} characters"
            ));
        }

        let max_results = parse_usize_with_bounds(
            object.get("max_results"),
            "max_results",
            DEFAULT_MAX_RESULTS,
            1,
            MAX_MAX_RESULTS,
        )?;
        let context_lines = parse_usize_with_bounds(
            object.get("context_lines"),
            "context_lines",
            DEFAULT_CONTEXT_LINES,
            0,
            MAX_CONTEXT_LINES,
        )?;

        Ok(Self {
            pattern,
            path: object
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or(".")
                .to_string(),
            include: parse_string_array(object.get("include"), "include")?,
            exclude: parse_string_array(object.get("exclude"), "exclude")?,
            is_regex: parse_bool(object.get("is_regex"), "is_regex", false)?,
            case_sensitive: parse_bool(object.get("case_sensitive"), "case_sensitive", true)?,
            max_results,
            context_lines,
            whole_word: parse_bool(object.get("whole_word"), "whole_word", false)?,
        })
    }
}

fn parse_string_array(value: Option<&Value>, name: &str) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| format!("Parameter '{name}' must be an array of strings"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("Parameter '{name}' must be an array of strings"))
        })
        .collect()
}

fn parse_bool(value: Option<&Value>, name: &str, default: bool) -> Result<bool, String> {
    match value {
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("Parameter '{name}' must be a boolean")),
        None => Ok(default),
    }
}

fn parse_usize_with_bounds(
    value: Option<&Value>,
    name: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = usize::try_from(
        value
            .as_u64()
            .ok_or_else(|| format!("Parameter '{name}' must be an integer"))?,
    )
    .map_err(|_| format!("Parameter '{name}' must be between {min} and {max}"))?;
    if parsed < min || parsed > max {
        return Err(format!(
            "Parameter '{name}' must be between {min} and {max}"
        ));
    }
    Ok(parsed)
}

fn build_pattern_string(params: &SearchParams) -> Result<String, String> {
    if params.pattern.is_empty() {
        return Err("Pattern must not be empty".to_string());
    }
    if params.pattern.chars().count() > MAX_PATTERN_LENGTH {
        return Err(format!(
            "Pattern exceeds maximum length of {MAX_PATTERN_LENGTH} characters"
        ));
    }

    let mut pattern = if params.is_regex {
        params.pattern.clone()
    } else {
        regex::escape(&params.pattern)
    };

    if !params.case_sensitive {
        pattern = format!("(?i){pattern}");
    }

    if params.whole_word {
        pattern = format!(r"\b{pattern}\b");
    }

    Ok(pattern)
}

fn search_workspace(
    security: Arc<SecurityPolicy>,
    params: SearchParams,
    regex: Regex,
    limits: SearchLimits,
) -> SearchOutcome {
    let start = Instant::now();
    let mut warnings = Vec::new();
    let plan = WorkspaceTrigramIndex::for_workspace(&security.workspace_dir)
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: params.path.clone(),
                include: params.include.clone(),
                exclude: params.exclude.clone(),
                raw_pattern: params.pattern.clone(),
                is_regex: params.is_regex,
                case_sensitive: params.case_sensitive,
                whole_word: params.whole_word,
            },
            limits.max_file_size_bytes,
        )
        .unwrap_or_else(|_| crate::search::CandidatePlan {
            coverage: CandidateCoverage::Unavailable,
            ordered_paths: Vec::new(),
            reason: "index_planning_failed".to_string(),
        });

    let inputs = match build_verification_inputs(&security, &params, &plan, limits, &mut warnings) {
        Ok(inputs) => inputs,
        Err(error) => {
            return SearchOutcome {
                matches: Vec::new(),
                stats: SearchStats {
                    files_searched: 0,
                    files_matched: 0,
                    total_matches: 0,
                    truncated: false,
                    duration_ms: 0,
                },
                warnings: vec![error.to_string()],
                fatal_error: Some(error.to_string()),
            };
        }
    };

    verify_inputs(inputs, &params, &regex, limits, start, warnings)
}

fn build_verification_inputs(
    security: &SecurityPolicy,
    params: &SearchParams,
    plan: &crate::search::CandidatePlan,
    limits: SearchLimits,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<VerificationInput>> {
    match plan.coverage {
        CandidateCoverage::Complete => {
            match read_index_candidate_inputs(security, &plan.ordered_paths, limits) {
                Ok(inputs) => {
                    if inputs.len() >= limits.max_files_scanned
                        && plan.ordered_paths.len() > inputs.len()
                    {
                        push_warning(
                            warnings,
                            format!(
                                "Search stopped after {} files. Narrow your search with 'path' or 'include' filters.",
                                limits.max_files_scanned
                            ),
                        );
                    }
                    Ok(inputs)
                }
                Err(_) => discover_fallback_inputs(security, params, limits, warnings),
            }
        }
        CandidateCoverage::Partial | CandidateCoverage::Unavailable => {
            discover_fallback_inputs(security, params, limits, warnings)
        }
    }
}

fn discover_fallback_inputs(
    security: &SecurityPolicy,
    params: &SearchParams,
    limits: SearchLimits,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<VerificationInput>> {
    let discovered = discover_searchable_files_with_stats(
        security,
        &params.path,
        &params.include,
        &params.exclude,
        SearchDiscoveryRules {
            max_file_size_bytes: limits.max_file_size_bytes,
            max_files: Some(limits.max_files_scanned),
            ..SearchDiscoveryRules::default()
        },
    )?;

    if discovered.hit_max_files {
        push_warning(
            warnings,
            format!(
                "Search stopped after {} files. Narrow your search with 'path' or 'include' filters.",
                limits.max_files_scanned
            ),
        );
    }

    Ok(discovered
        .files
        .into_iter()
        .map(|entry| VerificationInput {
            relative_path: entry.file.relative_path,
            bytes: entry.bytes,
        })
        .collect())
}

fn read_index_candidate_inputs(
    security: &SecurityPolicy,
    ordered_paths: &[String],
    limits: SearchLimits,
) -> anyhow::Result<Vec<VerificationInput>> {
    let workspace_root = security
        .workspace_dir
        .canonicalize()
        .unwrap_or_else(|_| security.workspace_dir.clone());
    let mut inputs = Vec::new();

    for relative_path in ordered_paths.iter().take(limits.max_files_scanned) {
        let resolved_path = resolve_relative_file(security, &workspace_root, relative_path)?;
        let mut file = OpenOptions::new()
            .read(true)
            .open(&resolved_path)
            .with_context(|| format!("failed to open candidate file '{}'", relative_path))?;
        let metadata = file
            .metadata()
            .with_context(|| format!("failed to read candidate metadata '{}'", relative_path))?;
        if !metadata.is_file() || metadata.len() > limits.max_file_size_bytes {
            continue;
        }

        let mut bytes = Vec::with_capacity(metadata.len().try_into().unwrap_or(0));
        file.read_to_end(&mut bytes)
            .with_context(|| format!("failed to read candidate file '{}'", relative_path))?;
        if is_binary_file(&bytes) {
            continue;
        }

        inputs.push(VerificationInput {
            relative_path: relative_path.clone(),
            bytes,
        });
    }

    Ok(inputs)
}

fn resolve_relative_file(
    security: &SecurityPolicy,
    workspace_root: &Path,
    relative_path: &str,
) -> anyhow::Result<PathBuf> {
    if !security.is_path_allowed(relative_path) {
        anyhow::bail!("candidate path not allowed by security policy: {relative_path}");
    }

    let joined = security.workspace_dir.join(relative_path);
    let resolved = joined
        .canonicalize()
        .with_context(|| format!("failed to resolve candidate path '{}'", relative_path))?;
    anyhow::ensure!(
        security.is_resolved_path_allowed(&resolved),
        "candidate path escapes workspace: {}",
        resolved.display()
    );
    let normalized = normalize_relative_path(workspace_root, &resolved)?;
    anyhow::ensure!(
        normalized == relative_path,
        "candidate path changed during resolution: expected '{}' but got '{}'",
        relative_path,
        normalized
    );
    Ok(resolved)
}

fn verify_inputs(
    inputs: Vec<VerificationInput>,
    params: &SearchParams,
    regex: &Regex,
    limits: SearchLimits,
    start: Instant,
    mut warnings: Vec<String>,
) -> SearchOutcome {
    let mut files_searched = 0usize;
    let mut files_matched = 0usize;
    let mut total_matches = 0usize;
    let mut matches = Vec::new();
    let mut truncated = warnings
        .iter()
        .any(|warning| warning.starts_with("Search stopped after"));

    'files: for input in inputs {
        if start.elapsed() >= limits.timeout {
            truncated = true;
            push_warning(
                &mut warnings,
                format!(
                    "Search timed out after {}ms. Partial results returned.",
                    limits.timeout.as_millis()
                ),
            );
            break;
        }

        files_searched += 1;
        let file_matches = verify_file_matches(
            &input.relative_path,
            &input.bytes,
            regex,
            params.context_lines,
            limits.max_line_content_chars,
            limits.max_matches_per_file,
        );

        if !file_matches.is_empty() {
            files_matched += 1;
        }

        for matched in file_matches {
            if matches.len() >= params.max_results {
                truncated = true;
                push_warning(
                    &mut warnings,
                    format!(
                        "Results truncated at {} matches. Narrow your search with 'path' or 'include' filters.",
                        params.max_results
                    ),
                );
                break 'files;
            }

            total_matches += 1;
            matches.push(matched);
        }
    }

    SearchOutcome {
        matches,
        stats: SearchStats {
            files_searched,
            files_matched,
            total_matches,
            truncated,
            duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
        warnings,
        fatal_error: None,
    }
}

fn verify_file_matches(
    relative_path: &str,
    bytes: &[u8],
    regex: &Regex,
    context_lines: usize,
    max_line_content_chars: usize,
    max_matches_per_file: usize,
) -> Vec<SearchMatch> {
    let contents = String::from_utf8_lossy(bytes);
    let lines = line_ranges(&contents);
    let mut matches = Vec::new();

    for found in regex.find_iter(&contents) {
        if matches.len() >= max_matches_per_file {
            break;
        }

        let start_line_index = locate_line(&lines, found.start());
        let end_anchor = found
            .end()
            .saturating_sub(1)
            .min(contents.len().saturating_sub(1));
        let end_line_index = locate_line(&lines, end_anchor);
        let line = slice_line(&contents, lines[start_line_index]);
        let (context_before, context_after) =
            collect_context(&contents, &lines, start_line_index, context_lines);

        matches.push(SearchMatch {
            file: relative_path.to_string(),
            line: start_line_index + 1,
            column: found.start().saturating_sub(lines[start_line_index].start) + 1,
            content: truncate_chars(line, max_line_content_chars),
            context_before,
            context_after,
            line_end: end_line_index + 1,
            column_end: found.end().saturating_sub(lines[end_line_index].start) + 1,
            byte_start: found.start(),
            byte_end: found.end(),
            preview: truncate_chars(
                &contents[found.start()..found.end()],
                max_line_content_chars,
            ),
        });
    }

    matches.sort_by(|left, right| {
        left.byte_start
            .cmp(&right.byte_start)
            .then(left.byte_end.cmp(&right.byte_end))
    });
    matches
}

fn collect_context(
    contents: &str,
    lines: &[LineRange],
    line_index: usize,
    context_lines: usize,
) -> (Vec<String>, Vec<String>) {
    if context_lines == 0 {
        return (Vec::new(), Vec::new());
    }

    let before_start = line_index.saturating_sub(context_lines);
    let before = lines[before_start..line_index]
        .iter()
        .map(|line| slice_line(contents, *line).to_string())
        .collect();
    let after_end = (line_index + 1 + context_lines).min(lines.len());
    let after = lines[line_index + 1..after_end]
        .iter()
        .map(|line| slice_line(contents, *line).to_string())
        .collect();

    (before, after)
}

fn line_ranges(contents: &str) -> Vec<LineRange> {
    if contents.is_empty() {
        return vec![LineRange { start: 0, end: 0 }];
    }

    let bytes = contents.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0usize;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let mut end = index;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            ranges.push(LineRange { start, end });
            start = index + 1;
        }
    }

    if start < bytes.len() {
        ranges.push(LineRange {
            start,
            end: bytes.len(),
        });
    }

    if ranges.is_empty() {
        ranges.push(LineRange { start: 0, end: 0 });
    }

    ranges
}

fn locate_line(lines: &[LineRange], byte_offset: usize) -> usize {
    lines
        .partition_point(|line| line.start <= byte_offset)
        .saturating_sub(1)
}

fn slice_line(contents: &str, line: LineRange) -> &str {
    &contents[line.start..line.end]
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
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

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

fn build_result(
    outcome: SearchOutcome,
    params: &SearchParams,
    max_output_bytes: usize,
) -> ToolResult {
    if let Some(error) = outcome.fatal_error {
        return ToolResult {
            success: false,
            output: outcome.warnings.join("\n"),
            error: Some(error.clone()),
            structured: Some(json!({
                "error": {
                    "message": error,
                    "warnings": outcome.warnings,
                },
                "stats": outcome.stats,
            })),
        };
    }

    if let Some(glob_warning) = outcome.warnings.iter().find(|warning| {
        warning.starts_with("Invalid include glob")
            || warning.starts_with("Invalid exclude glob")
            || warning.starts_with("Invalid search glob override")
    }) {
        return tool_error(glob_warning.clone());
    }

    let mut stats = outcome.stats.clone();
    let mut warnings = outcome.warnings.clone();
    let initial_output = format_output(&outcome.matches, &warnings, &stats, params.context_lines);

    if initial_output.len() > max_output_bytes {
        stats.truncated = true;
        push_warning(
            &mut warnings,
            format!(
                "Output truncated at {} bytes. Narrow your search with 'path' or 'include' filters.",
                max_output_bytes
            ),
        );
    }

    let structured_matches =
        cap_structured_matches(outcome.matches, &mut stats, &mut warnings, max_output_bytes);

    let mut output = format_output(&structured_matches, &warnings, &stats, params.context_lines);
    if output.len() > max_output_bytes {
        output = hard_cap_output(output, max_output_bytes);
    }

    let structured = json!({
        "matches": structured_matches,
        "stats": stats,
    });

    ToolResult {
        success: true,
        output,
        error: None,
        structured: Some(structured),
    }
}

fn cap_structured_matches(
    mut matches: Vec<SearchMatch>,
    stats: &mut SearchStats,
    warnings: &mut Vec<String>,
    max_output_bytes: usize,
) -> Vec<SearchMatch> {
    loop {
        let structured = json!({
            "matches": &matches,
            "stats": stats,
        });

        let serialized_len = serde_json::to_vec(&structured)
            .map(|bytes| bytes.len())
            .unwrap_or(0);

        if serialized_len <= max_output_bytes {
            return matches;
        }

        stats.truncated = true;
        push_warning(
            warnings,
            format!(
                "Structured results truncated at {} bytes. Narrow your search with 'path' or 'include' filters.",
                max_output_bytes
            ),
        );

        if matches.is_empty() {
            return matches;
        }

        matches.pop();
    }
}

fn format_output(
    matches: &[SearchMatch],
    warnings: &[String],
    stats: &SearchStats,
    context_lines: usize,
) -> String {
    let mut lines = Vec::new();

    for (index, matched) in matches.iter().enumerate() {
        if context_lines > 0 {
            let start_line = matched.line.saturating_sub(matched.context_before.len());
            for (offset, context) in matched.context_before.iter().enumerate() {
                lines.push(format!(
                    "{}-{}- {}",
                    matched.file,
                    start_line + offset,
                    context
                ));
            }
        }

        lines.push(format!(
            "{}:{}:{}: {}",
            matched.file, matched.line, matched.column, matched.content
        ));

        if context_lines > 0 {
            for (offset, context) in matched.context_after.iter().enumerate() {
                lines.push(format!(
                    "{}-{}- {}",
                    matched.file,
                    matched.line + offset + 1,
                    context
                ));
            }
            if index + 1 < matches.len() {
                lines.push("--".to_string());
            }
        }
    }

    lines.extend(warnings.iter().cloned());
    lines.push(format!(
        "Found {} matches in {} files ({} files searched, {}ms)",
        stats.total_matches, stats.files_matched, stats.files_searched, stats.duration_ms
    ));

    lines.join("\n")
}

fn hard_cap_output(output: String, max_output_bytes: usize) -> String {
    if output.len() <= max_output_bytes {
        return output;
    }

    let summary = output.lines().last().unwrap_or_default().to_string();
    let marker = "[output truncated]";
    let reserved = summary.len() + marker.len() + 2;
    let mut cut = max_output_bytes.saturating_sub(reserved).min(output.len());
    while cut > 0 && !output.is_char_boundary(cut) {
        cut -= 1;
    }
    let prefix = output[..cut].to_string();
    format!("{prefix}\n{marker}\n{summary}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::index::WorkspaceTrigramIndex;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use rusqlite::params;
    use serde_json::json;
    use tempfile::TempDir;

    fn test_security(workspace: &TempDir) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace.path().to_path_buf(),
            ..SecurityPolicy::default()
        })
    }

    fn test_security_with(
        workspace: &TempDir,
        autonomy: AutonomyLevel,
        max_actions_per_hour: u32,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            workspace_dir: workspace.path().to_path_buf(),
            max_actions_per_hour,
            ..SecurityPolicy::default()
        })
    }

    #[test]
    fn code_search_name() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        assert_eq!(tool.name(), "code_search");
    }

    #[test]
    fn code_search_schema_has_expected_fields() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let schema = tool.parameters_schema();
        for field in [
            "pattern",
            "path",
            "include",
            "exclude",
            "is_regex",
            "case_sensitive",
            "max_results",
            "context_lines",
            "whole_word",
        ] {
            assert!(
                schema["properties"][field].is_object(),
                "missing field: {field}"
            );
        }
        assert_eq!(schema["required"], json!(["pattern"]));
    }

    #[tokio::test]
    async fn code_search_empty_pattern_returns_error() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "fn main() {}")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": ""})).await.unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Pattern must not be empty"));
    }

    #[tokio::test]
    async fn code_search_pattern_too_long_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "x".repeat(1001)}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("maximum length of 1000"));
    }

    #[tokio::test]
    async fn code_search_invalid_regex_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "[invalid(", "is_regex": true}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("regex parse error"));
    }

    #[tokio::test]
    async fn code_search_nonexistent_path_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "foo", "path": "missing"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Search path not found"));
    }

    #[tokio::test]
    async fn code_search_blocks_absolute_paths() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "foo", "path": "/etc"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not allowed"));
    }

    #[tokio::test]
    async fn code_search_blocks_path_traversal() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "foo", "path": "../etc"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not allowed"));
    }

    #[tokio::test]
    async fn code_search_path_must_be_directory() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "fn main() {}")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "main", "path": "main.rs"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not a directory"));
    }

    #[test]
    fn code_search_escapes_literal_patterns() {
        let params = SearchParams {
            pattern: "vec[0]".to_string(),
            path: ".".to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            is_regex: false,
            case_sensitive: true,
            max_results: DEFAULT_MAX_RESULTS,
            context_lines: DEFAULT_CONTEXT_LINES,
            whole_word: false,
        };

        assert_eq!(build_pattern_string(&params).unwrap(), "vec\\[0\\]");
    }

    #[test]
    fn code_search_prepends_case_insensitive_flag() {
        let params = SearchParams {
            pattern: "need".to_string(),
            path: ".".to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            is_regex: false,
            case_sensitive: false,
            max_results: DEFAULT_MAX_RESULTS,
            context_lines: DEFAULT_CONTEXT_LINES,
            whole_word: false,
        };

        assert_eq!(build_pattern_string(&params).unwrap(), "(?i)need");
    }

    #[test]
    fn code_search_wraps_whole_word_pattern() {
        let params = SearchParams {
            pattern: "foo".to_string(),
            path: ".".to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            is_regex: true,
            case_sensitive: true,
            max_results: DEFAULT_MAX_RESULTS,
            context_lines: DEFAULT_CONTEXT_LINES,
            whole_word: true,
        };

        assert_eq!(build_pattern_string(&params).unwrap(), "\\bfoo\\b");
    }

    #[test]
    fn code_search_preserves_combined_pattern_order() {
        let params = SearchParams {
            pattern: "foo".to_string(),
            path: ".".to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            is_regex: false,
            case_sensitive: false,
            max_results: DEFAULT_MAX_RESULTS,
            context_lines: DEFAULT_CONTEXT_LINES,
            whole_word: true,
        };

        assert_eq!(build_pattern_string(&params).unwrap(), "\\b(?i)foo\\b");
    }

    #[tokio::test]
    async fn code_search_literal_search_finds_matches() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "fn main"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.expect("structured payload");
        assert_eq!(structured["matches"].as_array().unwrap().len(), 1);
        assert_eq!(structured["matches"][0]["file"], "src/main.rs");
    }

    #[tokio::test]
    async fn code_search_regex_search_finds_matches() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("vars.rs"), "let x = 42;\nlet y = 99;\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "let \\w+ = \\d+", "is_regex": true}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert_eq!(
            result.structured.unwrap()["matches"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn code_search_case_insensitive_matches_mixed_case() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("notes.txt"),
            "NEED: fix this\nNeed: refactor\n",
        )
        .await
        .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "need", "case_sensitive": false}))
            .await
            .unwrap();

        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(matches, 2);
    }

    #[tokio::test]
    async fn code_search_case_sensitive_is_default() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("notes.txt"), "Error\nerror\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "Error"})).await.unwrap();

        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(matches, 1);
    }

    #[tokio::test]
    async fn code_search_whole_word_skips_substrings() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("notes.txt"),
            "log(\"message\")\nlogger.info(\"message\")\n",
        )
        .await
        .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "log", "whole_word": true}))
            .await
            .unwrap();

        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(matches, 1);
    }

    #[tokio::test]
    async fn code_search_whole_word_regex_mode_skips_substrings() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("notes.txt"),
            "fn test()\nfn test_helper()\n",
        )
        .await
        .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "test", "is_regex": true, "whole_word": true}))
            .await
            .unwrap();

        let structured = result.structured.unwrap();
        let matches = &structured["matches"];
        assert_eq!(matches.as_array().unwrap().len(), 1);
        assert_eq!(matches[0]["line"], 1);
    }

    #[tokio::test]
    async fn code_search_regex_supports_unicode_character_classes() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("unicode.txt"), "café = true\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "\\w+ = true", "is_regex": true}))
            .await
            .unwrap();

        let structured = result.structured.unwrap();
        let matches = &structured["matches"];
        assert_eq!(matches.as_array().unwrap().len(), 1);
        assert_eq!(matches[0]["content"], "café = true");
    }

    #[tokio::test]
    async fn code_search_rejects_unsupported_lookahead_regex() {
        let dir = TempDir::new().unwrap();
        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "foo(?=bar)", "is_regex": true}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("regex parse error"));
    }

    #[tokio::test]
    async fn code_search_include_filters_files() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "hello\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("main.py"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "hello", "include": ["*.rs"]}))
            .await
            .unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["main.rs"]);
    }

    #[tokio::test]
    async fn code_search_exclude_filters_files() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/app.rs"), "struct App;\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/app.generated.rs"), "struct AppGen;\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "struct", "exclude": ["*.generated.rs"]}))
            .await
            .unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["src/app.rs"]);
    }

    #[tokio::test]
    async fn code_search_respects_gitignore() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join(".gitignore"), "target/\n")
            .await
            .unwrap();
        tokio::fs::create_dir_all(dir.path().join("target/debug"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("target/debug/output.rs"), "hello\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("readme.md"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["readme.md"]);
    }

    #[tokio::test]
    async fn code_search_skips_hidden_directories_by_default() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(dir.path().join(".hidden"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join(".hidden/secret.rs"), "hello\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("visible.rs"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["visible.rs"]);
    }

    #[tokio::test]
    async fn code_search_skips_binary_files() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("image.bin"), [0, 1, 2, 3, 0, 4])
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("readme.md"), "pattern\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "pattern"})).await.unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["readme.md"]);
    }

    #[tokio::test]
    async fn code_search_skips_large_files() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("small.txt"), "needle\n")
            .await
            .unwrap();
        let huge = vec![b'x'; 10 * 1024 * 1024 + 1];
        tokio::fs::write(dir.path().join("huge.txt"), huge)
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["small.txt"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn code_search_skips_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let workspace = root.path().join("workspace");
        let outside = root.path().join("outside");
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();
        tokio::fs::write(outside.join("secret.rs"), "secret\n")
            .await
            .unwrap();
        symlink(outside.join("secret.rs"), workspace.join("link.rs")).unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let tool = CodeSearchTool::new(security);
        let result = tool.execute(json!({"pattern": "secret"})).await.unwrap();

        assert!(result.success);
        assert!(result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn code_search_scopes_by_subdirectory() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(dir.path().join("tests"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/main.rs"), "fn target() {}\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("tests/main.rs"), "fn target() {}\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "target", "path": "src"}))
            .await
            .unwrap();

        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["src/main.rs"]);
    }

    #[tokio::test]
    async fn code_search_output_and_structured_format() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "fn main"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert!(result.output.contains("src/main.rs:1:1: fn main() {}"));
        let structured = result.structured.unwrap();
        assert!(structured["matches"].is_array());
        assert!(structured["stats"].is_object());
        assert!(structured["stats"]["duration_ms"].is_number());
    }

    #[tokio::test]
    async fn code_search_context_lines_and_group_separator() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("notes.txt"), "A\nB\nC\nD\nE\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "C", "context_lines": 1}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.unwrap();
        assert_eq!(structured["matches"][0]["context_before"], json!(["B"]));
        assert_eq!(structured["matches"][0]["context_after"], json!(["D"]));
        assert!(result.output.contains("notes.txt-2- B"));
        assert!(result.output.contains("notes.txt:3:1: C"));
    }

    #[tokio::test]
    async fn code_search_truncates_long_content() {
        let dir = TempDir::new().unwrap();
        let long_line = format!("{}needle", "a".repeat(700));
        tokio::fs::write(dir.path().join("long.txt"), format!("{long_line}\n"))
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        let content = result.structured.unwrap()["matches"][0]["content"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(content.chars().count(), 500);
    }

    #[tokio::test]
    async fn code_search_zero_matches_returns_success() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("readme.md"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "xyzzy_nonexistent_pattern_42"}))
            .await
            .unwrap();

        assert!(result.success);
        let structured = result.structured.unwrap();
        assert!(structured["matches"].as_array().unwrap().is_empty());
        assert_eq!(structured["stats"]["total_matches"], 0);
        assert_eq!(structured["stats"]["files_matched"], 0);
        assert_eq!(structured["stats"]["truncated"], false);
        assert!(result.output.contains("Found 0 matches in 0 files"));
    }

    #[tokio::test]
    async fn code_search_max_results_truncates() {
        let dir = TempDir::new().unwrap();
        let mut contents = String::new();
        for _ in 0..10 {
            contents.push_str("match_me\n");
        }
        tokio::fs::write(dir.path().join("many.txt"), contents)
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool
            .execute(json!({"pattern": "match_me", "max_results": 5}))
            .await
            .unwrap();

        let structured = result.structured.unwrap();
        assert_eq!(structured["matches"].as_array().unwrap().len(), 5);
        assert_eq!(structured["stats"]["truncated"], true);
        assert!(result.output.contains("Results truncated at 5 matches"));
    }

    #[tokio::test]
    async fn code_search_caps_matches_per_file_at_fifty() {
        let dir = TempDir::new().unwrap();
        let mut contents = String::new();
        for _ in 0..100 {
            contents.push_str("cap_me\n");
        }
        tokio::fs::write(dir.path().join("many.txt"), contents)
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "cap_me"})).await.unwrap();

        assert_eq!(
            result.structured.unwrap()["matches"]
                .as_array()
                .unwrap()
                .len(),
            50
        );
    }

    #[tokio::test]
    async fn code_search_rate_limited_invocation_is_rejected() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("readme.md"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security_with(&dir, AutonomyLevel::Supervised, 0));
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Rate limit exceeded"));
    }

    #[tokio::test]
    async fn code_search_counts_one_action_per_invocation() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("one.txt"), "hello\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("two.txt"), "hello\n")
            .await
            .unwrap();

        let security = test_security_with(&dir, AutonomyLevel::Supervised, 5);
        let before = security.tracker.count();
        let tool = CodeSearchTool::new(security.clone());
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert_eq!(security.tracker.count(), before + 1);
    }

    #[tokio::test]
    async fn code_search_allows_readonly_mode() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("readonly.txt"), "hello\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security_with(&dir, AutonomyLevel::ReadOnly, 5));
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn code_search_skips_unreadable_files() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let readable = dir.path().join("readable.txt");
        let unreadable = dir.path().join("unreadable.txt");
        tokio::fs::write(&readable, "hello\n").await.unwrap();
        tokio::fs::write(&unreadable, "hello\n").await.unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "hello"})).await.unwrap();

        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let files: Vec<String> = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(files, vec!["readable.txt"]);
    }

    #[test]
    fn code_search_file_scan_cap_sets_truncation() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("one.txt"), "hello\n").unwrap();
        fs::write(dir.path().join("two.txt"), "hello\n").unwrap();
        fs::write(dir.path().join("three.txt"), "hello\n").unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: dir.path().to_path_buf(),
            ..SecurityPolicy::default()
        });
        let params = SearchParams::from_args(&json!({"pattern": "hello"})).unwrap();
        let regex = Regex::new(&build_pattern_string(&params).unwrap()).unwrap();
        let outcome = search_workspace(
            security,
            params,
            regex,
            SearchLimits {
                max_files_scanned: 2,
                ..DEFAULT_LIMITS
            },
        );

        assert!(outcome.stats.truncated);
        assert!(outcome
            .warnings
            .iter()
            .any(|warning| warning.contains("Search stopped after 2 files")));
    }

    #[test]
    fn code_search_timeout_sets_truncation_warning() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("one.txt"), "hello\n").unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: dir.path().to_path_buf(),
            ..SecurityPolicy::default()
        });
        let params = SearchParams::from_args(&json!({"pattern": "hello"})).unwrap();
        let regex = Regex::new(&build_pattern_string(&params).unwrap()).unwrap();
        let outcome = search_workspace(
            security,
            params,
            regex,
            SearchLimits {
                timeout: Duration::ZERO,
                ..DEFAULT_LIMITS
            },
        );

        assert!(outcome.stats.truncated);
        assert!(outcome
            .warnings
            .iter()
            .any(|warning| warning.contains("Search timed out")));
    }
    #[tokio::test]
    async fn code_search_eliminates_index_false_positives_with_live_verification() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("alpha.rs"),
            "const VALUE: &str = \"needle\";\n",
        )
        .await
        .unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();
        tokio::fs::write(
            dir.path().join("alpha.rs"),
            "const VALUE: &str = \"other\";\n",
        )
        .await
        .unwrap();

        let tool = CodeSearchTool::new(security);
        let result = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert!(result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn code_search_falls_back_when_indexed_entry_is_hash_stale() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("alpha.rs");
        tokio::fs::write(&file_path, "abcOLDxyz\n").await.unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();

        tokio::fs::write(&file_path, "abcNEWxyz\n").await.unwrap();
        let modified_unix_ms = i64::try_from(
            std::fs::metadata(&file_path)
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let conn =
            rusqlite::Connection::open(dir.path().join("state/code-search/index.db")).unwrap();
        conn.execute(
            "UPDATE files SET size_bytes = ?1, modified_unix_ms = ?2 WHERE relative_path = 'alpha.rs'",
            params![10_i64, modified_unix_ms],
        )
        .unwrap();
        drop(conn);

        let tool = CodeSearchTool::new(security);
        let result = tool.execute(json!({"pattern": "NEW"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["file"], "alpha.rs");
    }

    #[tokio::test]
    async fn code_search_falls_back_when_index_is_unavailable() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "fn needle() {}\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(test_security(&dir));
        let result = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["file"], "main.rs");
    }

    #[tokio::test]
    async fn code_search_falls_back_for_unsupported_regex_query_shape() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "let value = 123;\n")
            .await
            .unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();

        let tool = CodeSearchTool::new(security);
        let result = tool
            .execute(json!({"pattern": "let \\w+ = \\d+", "is_regex": true}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let matches = result.structured.unwrap()["matches"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["file"], "main.rs");
        assert_eq!(matches[0]["byte_start"], json!(0));
    }

    #[tokio::test]
    async fn code_search_applies_max_results_to_verified_matches_after_filtering_candidates() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("a.rs"), "const VALUE: &str = \"needle\";\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("b.rs"), "const VALUE: &str = \"needle\";\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("c.rs"), "const VALUE: &str = \"needle\";\n")
            .await
            .unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();
        tokio::fs::write(dir.path().join("a.rs"), "const VALUE: &str = \"other\";\n")
            .await
            .unwrap();

        let tool = CodeSearchTool::new(security);
        let result = tool
            .execute(json!({"pattern": "needle", "max_results": 1}))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        let structured = result.structured.unwrap();
        let matches = structured["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["file"], "b.rs");
        assert_eq!(structured["stats"]["truncated"], json!(true));
    }

    #[tokio::test]
    async fn code_search_returns_deterministic_verified_order_across_repeated_runs() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("z.rs"), "needle second\n")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("a.rs"), "needle first\nneedle again\n")
            .await
            .unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();
        let tool = CodeSearchTool::new(security);

        let first = tool.execute(json!({"pattern": "needle"})).await.unwrap();
        let second = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        let first_structured = first.structured.unwrap();
        let second_structured = second.structured.unwrap();
        assert_eq!(first_structured["matches"], second_structured["matches"]);
        let files: Vec<_> = first_structured["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["file"].as_str().unwrap())
            .collect();
        assert_eq!(files, vec!["a.rs", "a.rs", "z.rs"]);
    }

    #[tokio::test]
    async fn code_search_structured_matches_include_verified_offsets_and_preview() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("main.rs"), "alpha needle beta\n")
            .await
            .unwrap();

        let security = test_security(&dir);
        WorkspaceTrigramIndex::for_workspace(dir.path())
            .build(security.clone())
            .unwrap();

        let tool = CodeSearchTool::new(security);
        let result = tool.execute(json!({"pattern": "needle"})).await.unwrap();

        let structured = result.structured.unwrap();
        let first = &structured["matches"][0];
        assert_eq!(first["file"], "main.rs");
        assert_eq!(first["line"], json!(1));
        assert_eq!(first["column"], json!(7));
        assert_eq!(first["byte_start"], json!(6));
        assert_eq!(first["byte_end"], json!(12));
        assert_eq!(first["line_end"], json!(1));
        assert_eq!(first["column_end"], json!(13));
        assert_eq!(first["preview"], json!("needle"));
        assert_eq!(first["content"], json!("alpha needle beta"));
        assert!(first["context_before"].is_array());
        assert!(first["context_after"].is_array());
    }
}
