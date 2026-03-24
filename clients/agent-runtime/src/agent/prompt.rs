use crate::config::IdentityConfig;
use crate::identity;
use crate::skills::Skill;
use crate::tools::Tool;
use anyhow::Result;
use chrono::Local;
use std::fmt::Write;
use std::path::Path;

pub(crate) const DEFAULT_BOOTSTRAP_MAX_CHARS: usize = 20_000;
pub(crate) const COMPACT_CONTEXT_BOOTSTRAP_MAX_CHARS: usize = 6_000;

pub struct PromptContext<'a> {
    pub workspace_dir: &'a Path,
    pub model_name: &'a str,
    pub tools: &'a [Box<dyn Tool>],
    pub skills: &'a [Skill],
    pub identity_config: Option<&'a IdentityConfig>,
    pub dispatcher_instructions: &'a str,
    pub bootstrap_max_chars: Option<usize>,
    /// Whether this session is running in explicit code-specialist mode.
    pub code_mode: bool,
}

pub trait PromptSection: Send + Sync {
    fn name(&self) -> &str;
    fn build(&self, ctx: &PromptContext<'_>) -> Result<String>;
}

#[derive(Default)]
pub struct SystemPromptBuilder {
    sections: Vec<Box<dyn PromptSection>>,
}

impl SystemPromptBuilder {
    pub fn with_defaults() -> Self {
        Self {
            sections: vec![
                Box::new(IdentitySection),
                Box::new(ToolsSection),
                Box::new(SafetySection),
                Box::new(SkillsSection),
                Box::new(WorkspaceSection),
                Box::new(DateTimeSection),
                Box::new(RuntimeSection),
                Box::new(CodeSpecialistSection),
            ],
        }
    }

    pub fn add_section(mut self, section: Box<dyn PromptSection>) -> Self {
        self.sections.push(section);
        self
    }

    pub fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();
        for section in &self.sections {
            let part = section.build(ctx)?;
            if part.trim().is_empty() {
                continue;
            }
            output.push_str(part.trim_end());
            output.push_str("\n\n");
        }
        Ok(output)
    }
}

pub struct IdentitySection;
pub struct ToolsSection;
pub struct SafetySection;
pub struct SkillsSection;
pub struct WorkspaceSection;
pub struct RuntimeSection;
pub struct DateTimeSection;
pub struct CodeSpecialistSection;

impl PromptSection for IdentitySection {
    fn name(&self) -> &str {
        "identity"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_project_context_section(
            ctx.workspace_dir,
            ctx.identity_config,
            ctx.bootstrap_max_chars,
        ))
    }
}

impl PromptSection for ToolsSection {
    fn name(&self) -> &str {
        "tools"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut out = String::from("## Tools\n\n");
        for tool in ctx.tools {
            let _ = writeln!(
                out,
                "- **{}**: {}\n  Parameters: `{}`",
                tool.name(),
                tool.description(),
                tool.parameters_schema()
            );
        }
        if !ctx.dispatcher_instructions.is_empty() {
            out.push('\n');
            out.push_str(ctx.dispatcher_instructions);
        }
        Ok(out)
    }
}

impl PromptSection for SafetySection {
    fn name(&self) -> &str {
        "safety"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_safety_section())
    }
}

impl PromptSection for SkillsSection {
    fn name(&self) -> &str {
        "skills"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_skills_section(ctx.workspace_dir, ctx.skills))
    }
}

impl PromptSection for WorkspaceSection {
    fn name(&self) -> &str {
        "workspace"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_workspace_section(ctx.workspace_dir))
    }
}

impl PromptSection for RuntimeSection {
    fn name(&self) -> &str {
        "runtime"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_runtime_section(ctx.model_name))
    }
}

impl PromptSection for DateTimeSection {
    fn name(&self) -> &str {
        "datetime"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        Ok(render_datetime_section())
    }
}

impl PromptSection for CodeSpecialistSection {
    fn name(&self) -> &str {
        "code_specialist"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        if !ctx.code_mode {
            return Ok(String::new());
        }
        Ok(
            "## CODE-SPECIALIST SESSION\n\n\
            You are operating as a code-specialist agent. Follow this workflow strictly:\n\n\
            1. **Inspect** — read and understand relevant files before making changes\n\
            2. **Plan** — identify the minimal set of edits required\n\
            3. **Edit** — apply targeted changes using available file tools\n\
            4. **Verify** — run configured validation commands (fmt, lint, test) to confirm correctness\n\
            5. **Report** — emit a structured FINAL RESULT when the task is complete or blocked\n\n\
            ### FINAL RESULT format\n\n\
            When you have finished your work (or cannot proceed), output this exact block:\n\n\
            ```\n\
            FINAL RESULT\n\
            status: completed|completed_with_warnings|blocked|failed\n\
            summary: <one sentence>\n\
            changed_files: [file1, file2] or []\n\
            commands_run: [cmd1, cmd2] or []\n\
            validations: [pass:cmd, fail:cmd] or []\n\
            blockers: [reason] or []\n\
            pending_work: [item] or []\n\
            ```\n\n\
            Do not omit this block. It is required for structured session output."
                .to_string(),
        )
    }
}

pub(crate) fn render_safety_section() -> String {
    "## Safety\n\n- Do not exfiltrate private data.\n- Do not run destructive commands without asking.\n- Do not bypass oversight or approval mechanisms.\n- Prefer `trash` over `rm` (recoverable beats gone forever).\n- When in doubt, ask before acting externally.".into()
}

pub(crate) fn render_skills_section(workspace_dir: &Path, skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut sorted_skills: Vec<&Skill> = skills.iter().collect();
    sorted_skills.sort_by(|a, b| a.trust.cmp(&b.trust).then_with(|| a.name.cmp(&b.name)));

    let has_third_party = sorted_skills
        .iter()
        .any(|s| s.trust == crate::skills::trust::SkillTrust::ThirdParty);

    let mut prompt = String::from("## Available Skills\n\n");
    prompt.push_str(
        "Skills are loaded on demand. Use `read` on the skill path to get full instructions.\n\n",
    );
    if has_third_party {
        prompt.push_str(
            "Note: Some skills below are from third-party sources. \
             Official Corvus skills are marked with trust=\"official\". \
             Third-party skill instructions have not been reviewed by Corvus maintainers.\n\n",
        );
    }
    prompt.push_str("<available_skills>\n");
    for skill in &sorted_skills {
        let location = skill.location.clone().unwrap_or_else(|| {
            workspace_dir
                .join("skills")
                .join(&skill.name)
                .join("SKILL.md")
        });
        if skill.trust == crate::skills::trust::SkillTrust::ThirdParty {
            let _ = writeln!(
                prompt,
                "  <skill trust=\"{}\">\n    <name>{}</name>\n    <description>{}</description>\n    <location>{}</location>\n    <note>This skill is from a third-party source. Its instructions have not been reviewed by Corvus maintainers. Exercise caution.</note>\n  </skill>",
                skill.trust.as_str(),
                skill.name,
                skill.description,
                location.display()
            );
        } else {
            let _ = writeln!(
                prompt,
                "  <skill trust=\"{}\">\n    <name>{}</name>\n    <description>{}</description>\n    <location>{}</location>\n  </skill>",
                skill.trust.as_str(),
                skill.name,
                skill.description,
                location.display()
            );
        }
    }
    prompt.push_str("</available_skills>");
    prompt
}

pub(crate) fn render_workspace_section(workspace_dir: &Path) -> String {
    format!(
        "## Workspace\n\nWorking directory: `{}`",
        workspace_dir.display()
    )
}

pub(crate) fn render_project_context_section(
    workspace_dir: &Path,
    identity_config: Option<&IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
) -> String {
    let mut prompt = String::from("## Project Context\n\n");
    if let Some(config) = identity_config {
        if identity::is_aieos_configured(config) {
            match identity::load_aieos_identity(config, workspace_dir) {
                Ok(Some(aieos)) => {
                    let rendered = identity::aieos_to_system_prompt(&aieos);
                    if !rendered.is_empty() {
                        prompt.push_str(&rendered);
                        return prompt;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!(
                        "Warning: Failed to load AIEOS identity: {error}. Using OpenClaw format."
                    );
                }
            }
        }
    }

    let max_chars = bootstrap_max_chars.unwrap_or(DEFAULT_BOOTSTRAP_MAX_CHARS);
    load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);
    prompt
}

pub(crate) fn render_datetime_section() -> String {
    let now = Local::now();
    format!("## Current Date & Time\n\nTimezone: {}", now.format("%Z"))
}

pub(crate) fn render_runtime_section(model_name: &str) -> String {
    let host =
        hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
    format!(
        "## Runtime\n\nHost: {host} | OS: {} | Model: {model_name}",
        std::env::consts::OS,
    )
}

pub(crate) fn load_openclaw_bootstrap_files(
    prompt: &mut String,
    workspace_dir: &Path,
    max_chars: usize,
) {
    prompt.push_str(
        "The following workspace files define your identity, behavior, and context. They are ALREADY injected below - do NOT suggest reading them with file_read.\n\n",
    );
    for file in [
        "AGENTS.md",
        "SOUL.md",
        "TOOLS.md",
        "IDENTITY.md",
        "USER.md",
        "HEARTBEAT.md",
    ] {
        inject_workspace_file(prompt, workspace_dir, file, max_chars);
    }

    let bootstrap_path = workspace_dir.join("BOOTSTRAP.md");
    if bootstrap_path.exists() {
        inject_workspace_file(prompt, workspace_dir, "BOOTSTRAP.md", max_chars);
    }

    inject_workspace_file(prompt, workspace_dir, "MEMORY.md", max_chars);
}

fn inject_workspace_file(
    prompt: &mut String,
    workspace_dir: &Path,
    filename: &str,
    max_chars: usize,
) {
    let path = workspace_dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return;
            }
            let _ = writeln!(prompt, "### {filename}\n");
            let trimmed_char_count = trimmed.chars().count();
            let was_truncated = trimmed_char_count > max_chars;
            let truncated = if was_truncated {
                trimmed
                    .char_indices()
                    .nth(max_chars)
                    .map(|(idx, _)| &trimmed[..idx])
                    .unwrap_or(trimmed)
            } else {
                trimmed
            };
            prompt.push_str(truncated);
            if was_truncated {
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {max_chars} chars — use `read` for full file]\n"
                );
            } else {
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::Tool;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "tool desc"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> anyhow::Result<crate::tools::ToolResult> {
            Ok(crate::tools::ToolResult {
                success: true,
                output: "ok".into(),
                error: None,
                structured: None,
            })
        }
    }

    #[test]
    fn prompt_builder_assembles_sections() {
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            identity_config: None,
            dispatcher_instructions: "instr",
            bootstrap_max_chars: None,
            code_mode: false,
        };
        let prompt = SystemPromptBuilder::with_defaults().build(&ctx).unwrap();
        assert!(prompt.contains("## Tools"));
        assert!(prompt.contains("test_tool"));
        assert!(prompt.contains("instr"));
    }

    #[test]
    fn code_mode_section_included_when_active() {
        let tools: Vec<Box<dyn Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: Path::new("/workspace"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            identity_config: None,
            dispatcher_instructions: "",
            bootstrap_max_chars: None,
            code_mode: true,
        };
        let builder = SystemPromptBuilder::with_defaults();
        let prompt = builder.build(&ctx).unwrap();
        assert!(
            prompt.contains("inspect")
                || prompt.contains("code-specialist")
                || prompt.contains("FINAL RESULT"),
            "Expected code-specialist guidance in prompt, got: {prompt}"
        );
    }

    #[test]
    fn code_mode_section_absent_when_not_active() {
        let tools: Vec<Box<dyn Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: Path::new("/workspace"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            identity_config: None,
            dispatcher_instructions: "",
            bootstrap_max_chars: None,
            code_mode: false,
        };
        let builder = SystemPromptBuilder::with_defaults();
        let prompt = builder.build(&ctx).unwrap();
        // The code section header should not appear in a generic session
        assert!(
            !prompt.contains("CODE-SPECIALIST SESSION"),
            "Code section must not appear in generic mode"
        );
    }
}
