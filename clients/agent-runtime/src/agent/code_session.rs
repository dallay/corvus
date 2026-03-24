use serde::{Deserialize, Serialize};

// ── Code Session Result Primitives ──────────────────────────────

/// Final status of a code session (direct or delegated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CodeSessionStatus {
    /// Session completed; all validation commands passed.
    #[default]
    Success,
    /// Session completed successfully.
    Completed,
    /// Session completed but one or more validation commands failed or produced warnings.
    CompletedWithWarnings,
    /// Session completed but one or more validation commands failed.
    ValidationFailed,
    /// Session was blocked by an unresolved issue and could not finish.
    Blocked,
    /// Session hit the iteration or time budget without completing.
    BudgetExceeded,
    /// Session terminated due to an internal error or missing FINAL RESULT block.
    Failed,
    /// Session terminated due to an internal error.
    Error,
}

impl CodeSessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Completed => "completed",
            Self::CompletedWithWarnings => "completed_with_warnings",
            Self::ValidationFailed => "validation_failed",
            Self::Blocked => "blocked",
            Self::BudgetExceeded => "budget_exceeded",
            Self::Failed => "failed",
            Self::Error => "error",
        }
    }
}

/// A file that was created or modified during a code session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    /// Workspace-relative path to the file.
    pub path: String,
    /// Change kind: "created", "modified", or "deleted".
    pub change: String,
}

/// Summary of a shell command executed during a code session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedCommandSummary {
    /// The command string that was run.
    pub command: String,
    /// Whether the command exited successfully.
    pub success: bool,
    /// Optional risk level tag (e.g. "low", "medium", "high").
    pub risk_level: Option<String>,
}

/// Result of a single validation command run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRunResult {
    /// Unique identifier for this validation step.
    pub id: String,
    /// The command that was run.
    pub command: String,
    /// Whether the command exited successfully.
    pub success: bool,
    /// Whether this validation is required (failure blocks completion).
    pub required: bool,
}

/// Result of a single validation command run (legacy name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutcome {
    /// The command that was run.
    pub command: String,
    /// Whether the command exited successfully.
    pub passed: bool,
    /// Command output (stdout + stderr, truncated if large).
    pub output: String,
}

/// Structured result emitted at the end of a code session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeSessionResult {
    /// Overall session status.
    pub status: CodeSessionStatus,
    /// Human-readable summary of what was done (for display in the parent agent).
    pub summary: String,
    /// Workspace-relative paths of files that were created or modified.
    pub changed_files: Vec<String>,
    /// Shell commands executed during the session.
    pub commands: Vec<ExecutedCommandSummary>,
    /// Validation command outcomes.
    pub validations: Vec<ValidationRunResult>,
    /// Unresolved blockers preventing session completion (empty on success).
    pub blockers: Vec<String>,
    /// Work items deferred to a future session.
    pub pending_work: Vec<String>,
    /// Unique identifier for this session.
    pub session_id: String,
    /// Files that were created, modified, or deleted (legacy field).
    #[serde(default)]
    pub files_changed: Vec<ChangedFile>,
    /// Shell commands executed during the session (legacy field).
    #[serde(default)]
    pub commands_executed: Vec<String>,
    /// Validation command outcomes (legacy field).
    #[serde(default)]
    pub validation_outcomes: Vec<ValidationOutcome>,
}

impl CodeSessionResult {
    pub fn is_success(&self) -> bool {
        matches!(
            self.status,
            CodeSessionStatus::Success
                | CodeSessionStatus::Completed
                | CodeSessionStatus::CompletedWithWarnings
        )
    }

    pub fn from_error(session_id: &str, status: CodeSessionStatus, summary: String) -> Self {
        Self {
            status,
            summary,
            changed_files: vec![],
            commands: vec![],
            validations: vec![],
            blockers: vec![],
            pending_work: vec![],
            session_id: session_id.to_string(),
            files_changed: vec![],
            commands_executed: vec![],
            validation_outcomes: vec![],
        }
    }
    /// Render a plain-text report suitable for returning as a `ToolResult.output`.
    pub fn render(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("status: {:?}", self.status));

        if !self.summary.is_empty() {
            lines.push(format!("summary: {}", self.summary));
        }

        self.render_files(&mut lines);
        self.render_commands(&mut lines);
        self.render_validations(&mut lines);
        self.render_blockers(&mut lines);

        lines.join("\n")
    }

    fn render_files(&self, lines: &mut Vec<String>) {
        let file_count = self.changed_files.len() + self.files_changed.len();
        if file_count > 0 {
            lines.push(format!("files_changed: {file_count}"));
            for f in &self.files_changed {
                lines.push(format!("  {} {}", f.change, f.path));
            }
            for f in &self.changed_files {
                lines.push(format!("  modified {f}"));
            }
        }
    }

    fn render_commands(&self, lines: &mut Vec<String>) {
        let cmd_count = self.commands.len() + self.commands_executed.len();
        if cmd_count > 0 {
            lines.push(format!("commands_executed: {cmd_count}"));
        }
    }

    fn render_validations(&self, lines: &mut Vec<String>) {
        let all_validations = self.validations.len() + self.validation_outcomes.len();
        if all_validations > 0 {
            let passed_new = self.validations.iter().filter(|v| v.success).count();
            let passed_legacy = self.validation_outcomes.iter().filter(|v| v.passed).count();
            let passed = passed_new + passed_legacy;
            lines.push(format!("validation: {}/{} passed", passed, all_validations));
            for v in &self.validations {
                let mark = if v.success { "✓" } else { "✗" };
                lines.push(format!("  {} {}", mark, v.command));
            }
            for v in &self.validation_outcomes {
                let mark = if v.passed { "✓" } else { "✗" };
                lines.push(format!("  {} {}", mark, v.command));
            }
        }
    }

    fn render_blockers(&self, lines: &mut Vec<String>) {
        if !self.blockers.is_empty() {
            lines.push(format!("blockers: {}", self.blockers.len()));
            for b in &self.blockers {
                lines.push(format!("  - {b}"));
            }
        }
    }

    /// Convert to a `serde_json::Value` for use as `ToolResult.structured`.
    pub fn to_structured(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Parse a structured `CodeSessionResult` from agent output text.
    /// Looks for a `FINAL RESULT` block and extracts fields.
    /// Returns a `Failed` result with a blocker note if no block is found.
    pub fn parse_from_output(output: &str, session_id: &str) -> Self {
        let Some(block_start) = output.find("FINAL RESULT") else {
            return Self {
                status: CodeSessionStatus::Failed,
                summary: "Session completed without emitting a FINAL RESULT block.".to_string(),
                changed_files: vec![],
                commands: vec![],
                validations: vec![],
                blockers: vec!["no FINAL RESULT block found in agent output".to_string()],
                pending_work: vec![],
                session_id: session_id.to_string(),
                files_changed: vec![],
                commands_executed: vec![],
                validation_outcomes: vec![],
            };
        };

        let block = &output[block_start..];
        let status = parse_field(block, "status")
            .map(|s| parse_status_label(&s))
            .unwrap_or(CodeSessionStatus::Failed);

        let summary = parse_field(block, "summary")
            .unwrap_or_default()
            .trim()
            .to_string();

        let changed_files = parse_list_field(block, "changed_files");
        let commands_raw = parse_list_field(block, "commands_run");
        let commands = commands_raw
            .into_iter()
            .map(|cmd| {
                let (command, success) = parse_command_entry(&cmd);
                ExecutedCommandSummary {
                    command,
                    success,
                    risk_level: None,
                }
            })
            .collect();

        let validations_raw = parse_list_field(block, "validations");
        let validations = validations_raw
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let (success, cmd) = if let Some(rest) = v.strip_prefix("pass:") {
                    (true, rest.to_string())
                } else if let Some(rest) = v.strip_prefix("fail:") {
                    (false, rest.to_string())
                } else {
                    (true, v)
                };
                ValidationRunResult {
                    id: format!("v{i}"),
                    command: cmd,
                    success,
                    required: true,
                }
            })
            .collect();

        let blockers = parse_list_field(block, "blockers");
        let pending_work = parse_list_field(block, "pending_work");

        Self {
            status,
            summary,
            changed_files,
            commands,
            validations,
            blockers,
            pending_work,
            session_id: session_id.to_string(),
            files_changed: vec![],
            commands_executed: vec![],
            validation_outcomes: vec![],
        }
    }
}

fn parse_status_label(s: &str) -> CodeSessionStatus {
    match s.trim().to_ascii_lowercase().as_str() {
        "success" => CodeSessionStatus::Success,
        "completed" => CodeSessionStatus::Completed,
        "completed_with_warnings" => CodeSessionStatus::CompletedWithWarnings,
        "validation_failed" => CodeSessionStatus::ValidationFailed,
        "blocked" => CodeSessionStatus::Blocked,
        "budget_exceeded" => CodeSessionStatus::BudgetExceeded,
        "error" => CodeSessionStatus::Error,
        _ => CodeSessionStatus::Failed,
    }
}

fn parse_command_entry(raw: &str) -> (String, bool) {
    let trimmed = raw.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let is_failure = lowered.starts_with("fail:")
        || lowered.starts_with("failed:")
        || lowered.starts_with("error:")
        || lowered.contains("[fail]")
        || lowered.contains("status: failed");
    let success = !is_failure;

    let cleaned = trimmed
        .strip_prefix("fail:")
        .or_else(|| trimmed.strip_prefix("failed:"))
        .or_else(|| trimmed.strip_prefix("error:"))
        .or_else(|| trimmed.strip_prefix("success:"))
        .or_else(|| trimmed.strip_prefix("succeeded:"))
        .unwrap_or(trimmed)
        .trim()
        .to_string();

    (cleaned, success)
}

/// Parse a single `key: value` line from the FINAL RESULT block.
fn parse_field(block: &str, key: &str) -> Option<String> {
    for line in block.lines() {
        if let Some(rest) = line.trim().strip_prefix(&format!("{key}:")) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Parse a `key: [item1, item2]` list field from the FINAL RESULT block.
fn parse_list_field(block: &str, key: &str) -> Vec<String> {
    let Some(raw) = parse_field(block, key) else {
        return vec![];
    };
    let trimmed = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.is_empty() {
        return vec![];
    }
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success_result() -> CodeSessionResult {
        CodeSessionResult {
            status: CodeSessionStatus::Success,
            summary: "Implemented feature X".into(),
            changed_files: vec![],
            commands: vec![],
            validations: vec![],
            blockers: vec![],
            pending_work: vec![],
            session_id: "test-session".into(),
            files_changed: vec![
                ChangedFile {
                    path: "src/main.rs".into(),
                    change: "modified".into(),
                },
                ChangedFile {
                    path: "src/lib.rs".into(),
                    change: "created".into(),
                },
            ],
            commands_executed: vec!["cargo build".into()],
            validation_outcomes: vec![ValidationOutcome {
                command: "cargo test".into(),
                passed: true,
                output: "test result: ok. 5 passed".into(),
            }],
        }
    }

    fn blocked_result() -> CodeSessionResult {
        CodeSessionResult {
            status: CodeSessionStatus::Blocked,
            summary: "Could not resolve type error".into(),
            changed_files: vec![],
            commands: vec![],
            validations: vec![],
            blockers: vec!["type mismatch in src/main.rs:42".into()],
            pending_work: vec![],
            session_id: "test-session".into(),
            files_changed: vec![],
            commands_executed: vec![],
            validation_outcomes: vec![ValidationOutcome {
                command: "cargo test".into(),
                passed: false,
                output: "error[E0308]: mismatched types".into(),
            }],
        }
    }

    // ── 1.3 Contract tests ────────────────────────────────────────

    #[test]
    fn code_session_status_defaults_to_success() {
        assert_eq!(CodeSessionStatus::default(), CodeSessionStatus::Success);
    }

    #[test]
    fn code_session_result_render_contains_status() {
        let result = success_result();
        let rendered = result.render();
        assert!(
            rendered.contains("Success"),
            "render must include status; got: {rendered}"
        );
    }

    #[test]
    fn code_session_result_render_contains_file_count() {
        let result = success_result();
        let rendered = result.render();
        assert!(
            rendered.contains("files_changed: 2"),
            "render must include file count; got: {rendered}"
        );
    }

    #[test]
    fn code_session_result_render_contains_validation_summary() {
        let result = success_result();
        let rendered = result.render();
        assert!(
            rendered.contains("1/1 passed"),
            "render must show validation pass ratio; got: {rendered}"
        );
    }

    #[test]
    fn code_session_result_render_contains_blockers_when_present() {
        let result = blocked_result();
        let rendered = result.render();
        assert!(
            rendered.contains("blockers: 1"),
            "render must list blockers; got: {rendered}"
        );
        assert!(
            rendered.contains("type mismatch"),
            "render must include blocker text; got: {rendered}"
        );
    }

    #[test]
    fn code_session_result_to_structured_is_valid_json_object() {
        let result = success_result();
        let structured = result.to_structured();
        assert!(structured.is_object());
        assert_eq!(structured["status"], "success");
        assert_eq!(structured["files_changed"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn code_session_result_serialization_roundtrip() {
        let original = success_result();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: CodeSessionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, CodeSessionStatus::Success);
        assert_eq!(parsed.files_changed.len(), 2);
        assert_eq!(parsed.summary, "Implemented feature X");
    }

    #[test]
    fn blocked_session_render_shows_failed_validation() {
        let result = blocked_result();
        let rendered = result.render();
        assert!(
            rendered.contains("Blocked"),
            "status must be Blocked; got: {rendered}"
        );
        assert!(
            rendered.contains("✗"),
            "failed validation must show ✗; got: {rendered}"
        );
    }

    #[test]
    fn parse_final_result_block_extracts_status_and_files() {
        let output = r#"
I've fixed the issue.

FINAL RESULT
status: completed
summary: Fixed failing test in lib.rs
changed_files: [src/lib.rs, src/main.rs]
commands_run: [cargo fmt, cargo test]
validations: [pass:cargo test]
blockers: []
pending_work: []
"#;
        let result = CodeSessionResult::parse_from_output(output, "sess-test");
        assert_eq!(result.status, CodeSessionStatus::Completed);
        assert_eq!(result.session_id, "sess-test");
        assert!(result.changed_files.contains(&"src/lib.rs".to_string()));
        assert!(!result.commands.is_empty());
    }

    #[test]
    fn parse_status_label_success_returns_success_not_completed() {
        assert_eq!(parse_status_label("success"), CodeSessionStatus::Success);
        assert_ne!(parse_status_label("success"), CodeSessionStatus::Completed);
    }

    #[test]
    fn parse_status_label_all_variants() {
        assert_eq!(
            parse_status_label("completed"),
            CodeSessionStatus::Completed
        );
        assert_eq!(
            parse_status_label("completed_with_warnings"),
            CodeSessionStatus::CompletedWithWarnings
        );
        assert_eq!(
            parse_status_label("validation_failed"),
            CodeSessionStatus::ValidationFailed
        );
        assert_eq!(parse_status_label("blocked"), CodeSessionStatus::Blocked);
        assert_eq!(
            parse_status_label("budget_exceeded"),
            CodeSessionStatus::BudgetExceeded
        );
        assert_eq!(parse_status_label("error"), CodeSessionStatus::Error);
        assert_eq!(
            parse_status_label("unknown_thing"),
            CodeSessionStatus::Failed
        );
    }

    #[test]
    fn parse_status_label_case_insensitive() {
        assert_eq!(parse_status_label("SUCCESS"), CodeSessionStatus::Success);
        assert_eq!(
            parse_status_label("  Success  "),
            CodeSessionStatus::Success
        );
    }

    #[test]
    fn parse_command_entry_success_prefix() {
        let (cmd, success) = parse_command_entry("success: cargo test");
        assert_eq!(cmd, "cargo test");
        assert!(success);
    }

    #[test]
    fn parse_command_entry_fail_prefix() {
        let (cmd, success) = parse_command_entry("fail: cargo clippy");
        assert_eq!(cmd, "cargo clippy");
        assert!(!success);
    }

    #[test]
    fn parse_command_entry_plain() {
        let (cmd, success) = parse_command_entry("cargo fmt");
        assert_eq!(cmd, "cargo fmt");
        assert!(success);
    }

    #[test]
    fn parse_final_result_returns_failed_when_no_block_found() {
        let output = "I couldn't complete the task. Something went wrong.";
        let result = CodeSessionResult::parse_from_output(output, "sess-missing");
        assert_eq!(result.status, CodeSessionStatus::Failed);
        assert!(result
            .blockers
            .iter()
            .any(|b| b.contains("no FINAL RESULT")));
    }
}
