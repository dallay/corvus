pub mod catalog;
pub mod frontmatter;
pub mod lockfile;
pub mod sandbox;
pub mod scanner;
pub mod trust;
pub mod validation;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A skill is a user-defined or community-built capability.
/// Skills live in `~/.corvus/workspace/skills/<name>/SKILL.md`
/// and can include tool definitions, prompts, and automation scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    #[serde(default)]
    pub prompts: Vec<String>,
    #[serde(skip)]
    pub location: Option<PathBuf>,
    #[serde(skip)]
    pub trust: trust::SkillTrust,
    #[serde(skip)]
    pub origin: trust::SkillOrigin,
    #[serde(skip)]
    pub allowed_tools: Vec<String>,
}

/// A tool defined by a skill (shell command, HTTP call, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    /// "shell", "http", "script"
    pub kind: String,
    /// The command/URL/script to execute
    pub command: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
    /// Whether this tool is sandboxed (filesystem restricted to skill dir).
    /// Derived from skill trust tier: true for ThirdParty, false for Official/Local.
    #[serde(skip)]
    pub sandboxed: bool,
}

/// Load all skills from the workspace skills directory.
/// Uses default config (integrity verification enabled).
pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    load_skills_with_config(workspace_dir, &crate::config::SkillsConfig::default())
}

/// Load all skills with explicit skills configuration.
pub fn load_skills_with_config(
    workspace_dir: &Path,
    config: &crate::config::SkillsConfig,
) -> Vec<Skill> {
    // Read lockfile for trust/origin enrichment
    let lockfile = lockfile::read_lockfile(workspace_dir);
    let skills_path = skills_dir(workspace_dir);

    let mut skills = load_workspace_skills(workspace_dir);
    // Enrich workspace skills with lockfile data
    for skill in &mut skills {
        if let Some(entry) = lockfile.skills.get(&skill.name) {
            skill.origin = lockfile::lock_entry_to_origin(entry);
            skill.trust = trust::SkillTrust::from(&skill.origin.source);
            if let Some(ref tools) = entry.allowed_tools {
                skill.allowed_tools = tools.clone();
            }
        }
        // Skills without lockfile entry keep default Local trust

        // Integrity verification
        if config.verify_integrity {
            let skill_md_path = skill
                .location
                .clone()
                .unwrap_or_else(|| skills_path.join(&skill.name).join("SKILL.md"));

            let result = lockfile::verify_integrity(
                &skill_md_path,
                skill.origin.content_hash.as_deref(),
                true,
            );

            if let lockfile::IntegrityResult::Mismatch { expected, actual } = result {
                match skill.trust {
                    trust::SkillTrust::ThirdParty => {
                        tracing::warn!(
                            "integrity mismatch for third-party skill '{}': \
                             expected {expected}, got {actual}. \
                             Tools disabled — instruction-only mode.",
                            skill.name,
                        );
                        skill.allowed_tools.clear();
                    }
                    trust::SkillTrust::Official => {
                        tracing::warn!(
                            "integrity mismatch for official skill '{}': \
                             expected {expected}, got {actual}. \
                             Content may have been updated locally.",
                            skill.name,
                        );
                    }
                    trust::SkillTrust::Local => {
                        tracing::warn!(
                            "integrity mismatch for local skill '{}': \
                             expected {expected}, got {actual}.",
                            skill.name,
                        );
                    }
                }
            }
        }

        // Scanner check (ThirdParty only)
        if skill.trust == trust::SkillTrust::ThirdParty {
            if let Some(threshold) = config.scan_threshold {
                let all_content: String = skill.prompts.join("\n");
                if !all_content.is_empty() {
                    let scan = scanner::scan_skill_content(&all_content);
                    if scan.exceeds_threshold(threshold) {
                        tracing::warn!(
                            "skill '{}' scored {} in injection scan \
                             (threshold: {}). \
                             Tools disabled — instruction-only mode.",
                            skill.name,
                            scan.score,
                            threshold,
                        );
                        skill.allowed_tools.clear();
                    }
                }
            }
        }

        // Set sandboxed flag on all tools based on trust tier
        for tool in &mut skill.tools {
            tool.sandboxed = skill.trust == trust::SkillTrust::ThirdParty;
        }
    }

    skills
}

fn load_workspace_skills(workspace_dir: &Path) -> Vec<Skill> {
    let skills_dir = workspace_dir.join("skills");
    load_skills_from_directory(&skills_dir)
}

fn load_skills_from_directory(skills_dir: &Path) -> Vec<Skill> {
    if !skills_dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();

    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return skills;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Validate directory name per Agent Skills standard (warn only for backward compat)
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if let Err(err) = validation::validate_skill_name(&dir_name) {
            tracing::warn!("skill '{}' has invalid name: {err}", dir_name);
        }

        let md_path = path.join("SKILL.md");

        if md_path.exists() {
            if let Ok(skill) = load_skill_md(&md_path, &path) {
                skills.push(skill);
            }
        } else if path.join("SKILL.toml").exists() {
            tracing::warn!(
                "Skill directory '{}' contains only SKILL.toml which is no longer supported. \
                 Create a SKILL.md file with YAML frontmatter instead. Skipping.",
                path.display(),
            );
        }
    }

    skills
}

/// Load a skill from a SKILL.md file
fn load_skill_md(path: &Path, dir: &Path) -> Result<Skill> {
    let content = std::fs::read_to_string(path)?;
    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let fm = frontmatter::parse_frontmatter(&content);

    Ok(Skill {
        name: fm.name.unwrap_or(dir_name),
        description: fm
            .description
            .unwrap_or_else(|| extract_description(&content)),
        version: fm.version.unwrap_or_else(|| "0.1.0".to_string()),
        author: fm.author,
        tags: fm.tags,
        tools: Vec::new(),
        prompts: vec![content],
        location: Some(path.to_path_buf()),
        trust: trust::SkillTrust::Local,
        origin: trust::SkillOrigin::default(),
        allowed_tools: fm.allowed_tools,
    })
}

fn extract_description(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.starts_with('#') && !line.trim().is_empty())
        .unwrap_or("No description")
        .trim()
        .to_string()
}

/// Filter skill tools based on trust tier and allowed-tools list.
///
/// Official and Local skills expose all tools unconditionally.
/// ThirdParty skills only expose tools declared in `allowed_tools`;
/// if `allowed_tools` is empty, no tools are exposed (instruction-only).
fn filter_tools_by_trust(skill: &Skill) -> Vec<&SkillTool> {
    match skill.trust {
        trust::SkillTrust::Official | trust::SkillTrust::Local => skill.tools.iter().collect(),
        trust::SkillTrust::ThirdParty => {
            if skill.allowed_tools.is_empty() {
                Vec::new()
            } else {
                skill
                    .tools
                    .iter()
                    .filter(|tool| skill.allowed_tools.contains(&tool.name))
                    .collect()
            }
        }
    }
}

/// Build a system prompt addition from all loaded skills
pub fn skills_to_prompt(skills: &[Skill]) -> String {
    use std::fmt::Write;

    if skills.is_empty() {
        return String::new();
    }

    let mut prompt = String::from("\n## Active Skills\n\n");

    for skill in skills {
        let _ = writeln!(prompt, "### {} (v{})", skill.name, skill.version);
        let _ = writeln!(prompt, "{}", skill.description);

        let visible_tools = filter_tools_by_trust(skill);
        if !visible_tools.is_empty() {
            prompt.push_str("Tools:\n");
            for tool in visible_tools {
                let _ = writeln!(
                    prompt,
                    "- **{}**: {} ({})",
                    tool.name, tool.description, tool.kind
                );
            }
        }

        for p in &skill.prompts {
            prompt.push_str(p);
            prompt.push('\n');
        }

        prompt.push('\n');
    }

    prompt
}

/// Check if a tool invocation is allowed under sandbox policy.
/// Returns Ok(()) or Err with violation description.
pub fn check_sandbox(tool: &SkillTool, skill: &Skill, args: &[&str]) -> Result<()> {
    if !tool.sandboxed {
        return Ok(()); // Not sandboxed — allow everything
    }

    let skill_dir = skill
        .location
        .as_ref()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("sandboxed tool has no skill directory"))?;

    sandbox::validate_tool_paths(args, skill_dir)
        .map_err(|violation| anyhow::anyhow!("sandbox violation: {violation}"))
}

/// Get the skills directory path
pub fn skills_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("skills")
}

/// Initialize the skills directory with a README
pub fn init_skills_dir(workspace_dir: &Path) -> Result<()> {
    let dir = skills_dir(workspace_dir);
    std::fs::create_dir_all(&dir)?;

    let readme = dir.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            "# Corvus Skills\n\n\
             Each subdirectory is a skill. Create a `SKILL.md` file inside.\n\n\
             ## SKILL.md format\n\n\
             Write a markdown file with optional YAML frontmatter and instructions for the agent.\n\n\
             ```markdown\n\
             ---\n\
             name: my-skill\n\
             description: What this skill does\n\
             version: 0.1.0\n\
             author: your-name\n\
             tags:\n\
               - productivity\n\
               - automation\n\
             ---\n\n\
             # My Skill\n\n\
             Instructions for the agent go here.\n\
             ```\n\n\
             ## Installing community skills\n\n\
             ```bash\n\
             corvus skills install <github-url>\n\
             corvus skills list\n\
             ```\n",
        )?;
    }

    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Handle the `skills` CLI command
#[allow(clippy::too_many_lines)]
pub fn handle_command(
    command: crate::SkillCommands,
    workspace_dir: &Path,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    match command {
        crate::SkillCommands::List { catalog } => {
            if catalog {
                handle_list_catalog(workspace_dir, config)
            } else {
                handle_list_command(workspace_dir)
            }
        }
        crate::SkillCommands::Install { source, trust } => {
            handle_install_command(workspace_dir, &source, trust, config)
        }
        crate::SkillCommands::Remove { name } => handle_remove_command(workspace_dir, &name),
        crate::SkillCommands::Search { query } => {
            handle_search_command(workspace_dir, &query, config)
        }
        crate::SkillCommands::Update { name } => {
            handle_update_command(workspace_dir, name.as_deref(), config)
        }
        crate::SkillCommands::Discover { query } => {
            handle_discover_command(workspace_dir, query.as_deref())
        }
        crate::SkillCommands::Lock { cmd } => match cmd {
            crate::LockCommands::Repair => handle_lock_repair_command(workspace_dir),
        },
    }
}

fn handle_list_command(workspace_dir: &Path) -> Result<()> {
    let skills = load_skills(workspace_dir);
    if skills.is_empty() {
        print_empty_skills_message();
    } else {
        print_installed_skills(&skills);
    }

    println!();
    Ok(())
}

fn handle_list_catalog(workspace_dir: &Path, config: &crate::config::SkillsConfig) -> Result<()> {
    let index = catalog::resolve_index(workspace_dir, config)?;

    if index.skills.is_empty() {
        println!("The official catalog has no skills yet.");
        println!(
            "Check back later or contribute at {}",
            catalog::OFFICIAL_REPO
        );
        return Ok(());
    }

    // Cross-reference with installed skills
    let lockfile = lockfile::read_lockfile(workspace_dir);
    let installed: std::collections::HashSet<&str> =
        lockfile.skills.keys().map(|s| s.as_str()).collect();

    println!(
        "  {} Official Skills Catalog ({} skills):\n",
        console::style("\u{1f4e6}").bold(),
        index.skills.len(),
    );

    for entry in index.skills.values() {
        let status = if installed.contains(entry.name.as_str()) {
            console::style("[installed]").green().to_string()
        } else {
            String::new()
        };
        println!(
            "  {:<20} {:<8} {} {}",
            console::style(&entry.name).cyan().bold(),
            entry.version.as_deref().unwrap_or("-"),
            entry.description,
            status,
        );
    }

    println!("\nInstall with: corvus skills install <name>");
    Ok(())
}

fn print_empty_skills_message() {
    println!("No skills installed.");
    println!();
    println!("  Create one: mkdir -p ~/.corvus/workspace/skills/my-skill");
    println!("              echo '# My Skill' > ~/.corvus/workspace/skills/my-skill/SKILL.md");
    println!();
    println!("  Or install: corvus skills install <github-url>");
}

fn print_installed_skills(skills: &[Skill]) {
    println!("Installed skills ({}):", skills.len());
    println!();

    for skill in skills {
        println!(
            "  {} {} — {}",
            console::style(&skill.name).white().bold(),
            console::style(format!("v{}", skill.version)).dim(),
            skill.description
        );

        if !skill.tools.is_empty() {
            println!("    Tools: {}", format_tool_names(skill));
        }

        if !skill.tags.is_empty() {
            println!("    Tags:  {}", skill.tags.join(", "));
        }
    }
}

fn format_tool_names(skill: &Skill) -> String {
    skill
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn handle_lock_repair_command(workspace_dir: &Path) -> Result<()> {
    println!(
        "  {} Repairing skills lockfile...",
        console::style("🔧").bold(),
    );

    let summary = lockfile::repair_lockfile(workspace_dir)?;

    println!(
        "  {} Lockfile repaired: {} added, {} removed, {} updated, {} unchanged.",
        console::style("✓").green().bold(),
        summary.added,
        summary.removed,
        summary.updated,
        summary.unchanged,
    );
    Ok(())
}

fn handle_search_command(
    workspace_dir: &Path,
    query: &str,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    let index = catalog::resolve_index(workspace_dir, config)?;
    let results = catalog::search(&index, query);

    if results.is_empty() {
        println!("No skills found matching '{query}'.");
        println!("Try a different search term or browse with 'corvus skills list --catalog'.");
        return Ok(());
    }

    println!(
        "  {} Found {} skill(s) matching '{}':\n",
        console::style("🔍").bold(),
        results.len(),
        query,
    );

    for entry in &results {
        println!(
            "  {:<20} {:<8} {}",
            console::style(&entry.name).green().bold(),
            entry.version.as_deref().unwrap_or("-"),
            entry.description,
        );
        if !entry.tags.is_empty() {
            println!("  {:<20} tags: {}", "", entry.tags.join(", "));
        }
    }

    println!("\nInstall with: corvus skills install <name>");
    Ok(())
}

/// A discovered skill from an external source (display-only).
struct DiscoveredSkill {
    name: String,
    description: String,
    url: String,
    stars: u64,
}

fn handle_discover_command(_workspace_dir: &Path, query: Option<&str>) -> Result<()> {
    println!(
        "  {} Discovering skills from external sources...\n",
        console::style("🔍").bold(),
    );

    // Search GitHub for skill-related repositories using the blocking reqwest
    // client (already a dependency). This avoids coupling to the skillforge
    // module which lives in the binary crate.
    let results = discover_from_github()?;

    if results.is_empty() {
        println!("  No skills discovered. Try different search terms.");
        return Ok(());
    }

    // Filter by query if provided
    let filtered: Vec<_> = if let Some(q) = query {
        let q_lower = q.to_lowercase();
        results
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&q_lower)
                    || r.description.to_lowercase().contains(&q_lower)
            })
            .collect()
    } else {
        results.iter().collect()
    };

    if filtered.is_empty() {
        println!("  No skills found matching '{}'.", query.unwrap_or(""));
        return Ok(());
    }

    println!(
        "  Found {} skill(s) from external sources:\n",
        filtered.len(),
    );

    for result in &filtered {
        println!(
            "  {:<25} {} ⭐ {}",
            console::style(&result.name).yellow().bold(),
            result.description.chars().take(50).collect::<String>(),
            result.stars,
        );
        println!("  {:<25} {}", "", console::style(&result.url).dim(),);
    }

    println!(
        "\n  {} These are third-party skills (not reviewed by Corvus).",
        console::style("⚠").yellow().bold(),
    );
    println!("  Install with: corvus skills install <url> --trust");
    Ok(())
}

/// Query GitHub's search API for skill-related repos using blocking reqwest.
fn discover_from_github() -> Result<Vec<DiscoveredSkill>> {
    let queries = ["corvus+skill", "ai+agent+skill"];
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Corvus-SkillForge/0.1")
        .build()?;

    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();

    for query in &queries {
        let url = format!(
            "https://api.github.com/search/repositories\
             ?q={query}&sort=stars&order=desc&per_page=30",
        );

        let resp = match client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "GitHub API request failed, skipping");
                continue;
            }
        };

        if !resp.status().is_success() {
            tracing::warn!(status = %resp.status(), "GitHub search returned non-200");
            continue;
        }

        let body: serde_json::Value = match resp.json() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse GitHub response");
                continue;
            }
        };

        let items = body
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for item in &items {
            let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let url = item
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !seen.insert(url.clone()) {
                continue;
            }

            results.push(DiscoveredSkill {
                name: name.to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                url,
                stars: item
                    .get("stargazers_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            });
        }
    }

    Ok(results)
}

/// Install an official skill resolved from the catalog index.
/// Only this code path may produce `SkillSource::Official` — URL installs
/// of the same repo MUST remain `ThirdParty` (AD5).
fn handle_catalog_install(
    workspace_dir: &Path,
    name: &str,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    // 1. Resolve catalog index
    let index = catalog::resolve_index(workspace_dir, config)?;

    // 2. Look up skill in catalog
    let entry = match index.skills.get(name) {
        Some(e) => e,
        None => {
            let suggestions = catalog::search(&index, name);
            if suggestions.is_empty() {
                anyhow::bail!(
                    "Skill '{name}' not found in the official catalog. \
                     Try 'corvus skills search {name}' or install by URL."
                );
            }
            let names: Vec<&str> = suggestions.iter().map(|s| s.name.as_str()).collect();
            anyhow::bail!(
                "Skill '{name}' not found in the official catalog. \
                 Did you mean: {}? \
                 Or install by URL with 'corvus skills install <url>'.",
                names.join(", ")
            );
        }
    };

    // Defense in depth: validate catalog name even though catalog entries should be valid
    if let Err(err) = validation::validate_skill_name(name) {
        anyhow::bail!("Invalid skill name in catalog: {err}");
    }

    println!(
        "  {} Installing official skill '{}' (v{})...",
        console::style("→").cyan().bold(),
        entry.name,
        entry.version.as_deref().unwrap_or("latest"),
    );

    // 3. Clone from official repo (shallow) and copy skill subdirectory
    let skills_path = skills_dir(workspace_dir);
    std::fs::create_dir_all(&skills_path)?;
    let skill_dir = skills_path.join(name);

    if skill_dir.exists() {
        anyhow::bail!(
            "Skill '{name}' is already installed. \
             Use 'corvus skills update {name}' to update."
        );
    }

    let repo_url = catalog::OFFICIAL_REPO;
    let temp_dir = std::env::temp_dir().join(format!("corvus-catalog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let clone_status = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            repo_url,
            temp_dir.to_str().unwrap_or("corvus-catalog-tmp"),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match clone_status {
        Ok(s) if s.success() => {}
        _ => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            anyhow::bail!("Failed to clone official skills repository");
        }
    }

    // Copy just the skill subdirectory
    let source_path = temp_dir.join(&entry.path);
    if !source_path.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        anyhow::bail!(
            "Skill path '{}' not found in the official repository",
            entry.path
        );
    }

    let copy_result = copy_dir_recursive(&source_path, &skill_dir);
    let _ = std::fs::remove_dir_all(&temp_dir);
    copy_result?;

    // 4. Validate SKILL.md exists
    let skill_md = skill_dir.join("SKILL.md");
    if !skill_md.exists() {
        let _ = std::fs::remove_dir_all(&skill_dir);
        anyhow::bail!("No SKILL.md found in official skill '{name}'");
    }

    // 5. Compute hash and write lockfile — NO trust gate for Official
    let content = std::fs::read_to_string(&skill_md)?;
    let hash = lockfile::compute_content_hash(content.as_bytes());
    let source_str = format!(
        "official:{}",
        repo_url.trim_start_matches("https://github.com/"),
    );
    let lock_entry = lockfile::build_lock_entry(
        trust::SkillTrust::Official,
        &source_str,
        None,
        Some(hash),
        None,
        Some(entry.path.clone()),
    );
    lockfile::write_lock_entry(workspace_dir, name, lock_entry)?;

    println!(
        "  {} Official skill '{}' installed successfully.",
        console::style("✓").green().bold(),
        name,
    );
    Ok(())
}

fn handle_install_command(
    workspace_dir: &Path,
    source: &str,
    trust_flag: bool,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    // Bare catalog name detection — must come before URL/path checks (AD5)
    if catalog::is_bare_name(source) {
        return handle_catalog_install(workspace_dir, source, config);
    }

    println!("Installing skill from: {source}");

    let skills_path = skills_dir(workspace_dir);
    std::fs::create_dir_all(&skills_path)?;

    if source.starts_with("http://") {
        anyhow::bail!("Refusing insecure remote skill source: {source}");
    }

    // 1. Resolve source and derive trust tier
    let skill_source = resolve_skill_source(source);
    let skill_trust = trust::SkillTrust::from(&skill_source);

    // 2. Install (clone or symlink)
    let skill_dir = if is_remote_skill_source(source) {
        install_remote_skill(&skills_path, source)?
    } else {
        install_local_skill(&skills_path, source)?
    };

    // 3. Validate structure and parse frontmatter
    let skill_md_path = skill_dir.join("SKILL.md");
    let (fm, content_hash) = if skill_md_path.exists() {
        let content = std::fs::read_to_string(&skill_md_path)?;
        let fm = frontmatter::parse_frontmatter(&content);

        // Abort if frontmatter name doesn't match directory name
        let dir_name = skill_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(ref fm_name) = fm.name {
            if fm_name != dir_name {
                let _ = std::fs::remove_dir_all(&skill_dir);
                anyhow::bail!(
                    "Skill name '{}' in SKILL.md does not match directory '{}'. \
                     Rename the skill or directory to match.",
                    fm_name,
                    dir_name,
                );
            }
        }

        let hash = lockfile::compute_content_hash(content.as_bytes());
        (fm, Some(hash))
    } else {
        let _ = std::fs::remove_dir_all(&skill_dir);
        anyhow::bail!(
            "No SKILL.md found in installed skill directory. \
             Skills must contain a SKILL.md file."
        );
    };

    // 3b. Validate skill name per Agent Skills standard
    let dir_name_for_validation = skill_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let skill_name_to_validate = fm.name.as_deref().unwrap_or(dir_name_for_validation);
    if let Err(err) = validation::validate_skill_name(skill_name_to_validate) {
        let _ = std::fs::remove_dir_all(&skill_dir);
        anyhow::bail!("Invalid skill name: {err}");
    }

    // 3c. Scan for prompt injection (ThirdParty only)
    if skill_trust == trust::SkillTrust::ThirdParty {
        if let Ok(content) = std::fs::read_to_string(skill_dir.join("SKILL.md")) {
            let scan = scanner::scan_skill_content(&content);
            if scan.exceeds_threshold(scanner::DEFAULT_SCAN_THRESHOLD) && !trust_flag {
                // Report findings and abort
                for finding in &scan.findings {
                    println!(
                        "  {} [line {}] {} (score: +{})",
                        console::style("\u{26a0}").yellow().bold(),
                        finding.line,
                        finding.pattern,
                        finding.severity,
                    );
                }
                let _ = std::fs::remove_dir_all(&skill_dir);
                anyhow::bail!(
                    "Skill content scored {} (threshold: {}). \
               Use --trust to install anyway.",
                    scan.score,
                    scanner::DEFAULT_SCAN_THRESHOLD,
                );
            } else if scan.exceeds_threshold(scanner::DEFAULT_SCAN_THRESHOLD) {
                // --trust flag: warn but proceed
                tracing::warn!(
                    "skill content scored {} (threshold: {}) — proceeding with --trust",
                    scan.score,
                    scanner::DEFAULT_SCAN_THRESHOLD,
                );
            }
        }
    }

    // 4. Trust gate: ThirdParty skills with tools require explicit consent
    if skill_trust == trust::SkillTrust::ThirdParty && !fm.allowed_tools.is_empty() && !trust_flag {
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            let skill_name = skill_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            if !confirm_trust_install(skill_name, &fm.allowed_tools) {
                let _ = std::fs::remove_dir_all(&skill_dir);
                anyhow::bail!("Installation declined by user.");
            }
        } else {
            let _ = std::fs::remove_dir_all(&skill_dir);
            anyhow::bail!(
                "This third-party skill requests tools: {}. \
                 Use --trust to allow installation in \
                 non-interactive mode.",
                fm.allowed_tools.join(", ")
            );
        }
    }

    // 5. Build and write lock entry
    let skill_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let source_str = match &skill_source {
        trust::SkillSource::Local | trust::SkillSource::LinkedLocal { .. } => "local".to_string(),
        trust::SkillSource::GitRepo { url } => url.clone(),
        trust::SkillSource::Official { repo, .. } | trust::SkillSource::Discovered { repo, .. } => {
            repo.clone()
        }
    };
    let allowed_tools = if fm.allowed_tools.is_empty() {
        None
    } else {
        Some(fm.allowed_tools)
    };
    let entry = lockfile::build_lock_entry(
        skill_trust,
        &source_str,
        None,
        content_hash,
        allowed_tools,
        None,
    );
    if let Err(err) = lockfile::write_lock_entry(workspace_dir, &skill_name, entry) {
        tracing::warn!("failed to write lockfile entry for '{skill_name}': {err}");
    }

    // 6. Print success with trust info
    println!(
        "  {} Skill installed successfully! (trust: {})",
        console::style("✓").green().bold(),
        skill_trust.as_str(),
    );
    println!("  Restart `corvus channel start` to activate.");
    Ok(())
}

/// Resolve the skill source type from the install source string.
fn resolve_skill_source(source: &str) -> trust::SkillSource {
    if source.starts_with("https://") {
        trust::SkillSource::GitRepo {
            url: source.to_string(),
        }
    } else {
        trust::SkillSource::Local
    }
}

/// Interactive confirmation for installing a third-party skill with tools.
fn confirm_trust_install(skill_name: &str, tools: &[String]) -> bool {
    println!(
        "  {} This third-party skill '{}' requests tools: {}",
        console::style("⚠").yellow().bold(),
        skill_name,
        tools.join(", ")
    );
    print!("  Allow installation? (y/N) ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let answer = input.trim().to_ascii_lowercase();
        return answer == "y" || answer == "yes";
    }
    false
}

fn is_remote_skill_source(source: &str) -> bool {
    source.starts_with("https://")
}

fn install_remote_skill(skills_path: &Path, source: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["clone", "--depth", "1", source])
        .current_dir(skills_path)
        .output()?;

    if output.status.success() {
        let name = source
            .rsplit('/')
            .next()
            .unwrap_or("skill")
            .trim_end_matches(".git");
        return Ok(skills_path.join(name));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("Git clone failed: {stderr}")
}

fn install_local_skill(skills_path: &Path, source: &str) -> Result<PathBuf> {
    let src = PathBuf::from(source);
    if !src.exists() {
        anyhow::bail!("Source path does not exist: {source}");
    }

    let name = src
        .file_name()
        .filter(|name| !name.is_empty())
        .filter(|name| *name != std::ffi::OsStr::new("."))
        .filter(|name| *name != std::ffi::OsStr::new(".."))
        .ok_or_else(|| anyhow::anyhow!("Invalid skill source path: {source}"))?;
    let dest = skills_path.join(name);

    link_or_copy_local_skill(&src, &dest)?;
    Ok(dest)
}

#[cfg(unix)]
fn link_or_copy_local_skill(src: &Path, dest: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, dest)?;
    print_skill_location("linked", dest);
    Ok(())
}

#[cfg(windows)]
fn link_or_copy_local_skill(src: &Path, dest: &Path) -> Result<()> {
    use std::os::windows::fs::symlink_dir;

    if symlink_dir(src, dest).is_ok() {
        print_skill_location("linked", dest);
        return Ok(());
    }

    let junction_result = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dest)
        .arg(src)
        .output();

    if junction_result
        .as_ref()
        .is_ok_and(|output| output.status.success())
    {
        print_skill_location("linked (junction)", dest);
        return Ok(());
    }

    copy_dir_recursive(src, dest)?;
    print_skill_location("copied", dest);
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn link_or_copy_local_skill(src: &Path, dest: &Path) -> Result<()> {
    copy_dir_recursive(src, dest)?;
    print_skill_location("copied", dest);
    Ok(())
}

fn print_skill_location(action: &str, dest: &Path) {
    println!(
        "  {} Skill {}: {}",
        console::style("✓").green().bold(),
        action,
        dest.display()
    );
}

fn handle_update_command(
    workspace_dir: &Path,
    name: Option<&str>,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    let lockfile = lockfile::read_lockfile(workspace_dir);

    if lockfile.skills.is_empty() {
        println!("No installed skills to update.");
        return Ok(());
    }

    // If a specific name given, update just that one
    if let Some(skill_name) = name {
        if !lockfile.skills.contains_key(skill_name) {
            anyhow::bail!("Skill '{skill_name}' is not installed.");
        }
        return update_single_skill(workspace_dir, skill_name, &lockfile, config);
    }

    // Update all
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    let skill_names: Vec<String> = lockfile.skills.keys().cloned().collect();

    for skill_name in &skill_names {
        match update_single_skill(workspace_dir, skill_name, &lockfile, config) {
            Ok(()) => updated += 1,
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("skipping") {
                    skipped += 1;
                    println!("  {} {}", console::style("→").dim(), msg);
                } else {
                    failed += 1;
                    println!(
                        "  {} Failed to update '{}': {}",
                        console::style("✗").red().bold(),
                        skill_name,
                        msg,
                    );
                }
            }
        }
    }

    println!(
        "\n  Updated: {}, Skipped: {}, Failed: {}",
        updated, skipped, failed,
    );
    Ok(())
}

fn update_single_skill(
    workspace_dir: &Path,
    name: &str,
    lockfile: &lockfile::SkillsLockfile,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    let entry = lockfile
        .skills
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Skill '{name}' not in lockfile"))?;

    let origin = lockfile::lock_entry_to_origin(entry);
    let trust_tier = trust::SkillTrust::from(&origin.source);

    match trust_tier {
        trust::SkillTrust::Local => {
            anyhow::bail!("Local skill '{name}' — skipping (not managed by a remote source)",);
        }
        trust::SkillTrust::Official => update_official_skill(workspace_dir, name, entry, config),
        trust::SkillTrust::ThirdParty => update_thirdparty_skill(workspace_dir, name, entry),
    }
}

fn update_official_skill(
    workspace_dir: &Path,
    name: &str,
    entry: &lockfile::LockEntry,
    config: &crate::config::SkillsConfig,
) -> Result<()> {
    let index = catalog::resolve_index(workspace_dir, config)?;

    let catalog_entry = index
        .skills
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Skill '{name}' no longer in official catalog"))?;

    // Compare content hashes
    if let (Some(installed_hash), Some(catalog_hash)) =
        (&entry.content_hash, &catalog_entry.content_hash)
    {
        if installed_hash == catalog_hash {
            println!(
                "  {} '{}' is up to date.",
                console::style("✓").green().bold(),
                name,
            );
            return Ok(());
        }
    }

    println!(
        "  {} Updating official skill '{}'...",
        console::style("→").cyan().bold(),
        name,
    );

    let skills_path = skills_dir(workspace_dir);
    let skill_dir = skills_path.join(name);

    let repo_url = catalog::OFFICIAL_REPO;
    // Clone to temp first (atomic update)
    let temp_base = std::env::temp_dir().join(format!("corvus-update-{name}"));
    let _ = std::fs::remove_dir_all(&temp_base);

    let clone_status = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            repo_url,
            temp_base.to_str().unwrap_or("corvus-update-tmp"),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match clone_status {
        Ok(s) if s.success() => {}
        _ => {
            let _ = std::fs::remove_dir_all(&temp_base);
            anyhow::bail!("Failed to clone official skills repository for update",);
        }
    }

    let source_path = temp_base.join(&catalog_entry.path);
    if !source_path.exists() {
        let _ = std::fs::remove_dir_all(&temp_base);
        anyhow::bail!(
            "Skill path '{}' not found in official repo",
            catalog_entry.path,
        );
    }

    // Copy skill subdirectory to staging
    let staging_dir = skill_dir.with_extension("staging");
    let _ = std::fs::remove_dir_all(&staging_dir);
    copy_dir_recursive(&source_path, &staging_dir)?;
    let _ = std::fs::remove_dir_all(&temp_base);

    // Atomic swap
    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)?;
    }
    std::fs::rename(&staging_dir, &skill_dir)?;

    // Update lockfile
    let skill_md = skill_dir.join("SKILL.md");
    let hash = if skill_md.exists() {
        let content = std::fs::read_to_string(&skill_md)?;
        Some(lockfile::compute_content_hash(content.as_bytes()))
    } else {
        None
    };

    let source_str = format!(
        "official:{}",
        repo_url.trim_start_matches("https://github.com/"),
    );
    let new_entry = lockfile::build_lock_entry(
        trust::SkillTrust::Official,
        &source_str,
        None,
        hash,
        None,
        Some(catalog_entry.path.clone()),
    );
    lockfile::write_lock_entry(workspace_dir, name, new_entry)?;

    println!(
        "  {} '{}' updated successfully.",
        console::style("✓").green().bold(),
        name,
    );
    Ok(())
}

fn update_thirdparty_skill(
    workspace_dir: &Path,
    name: &str,
    entry: &lockfile::LockEntry,
) -> Result<()> {
    let source_url = &entry.source;
    if source_url == "local" || !source_url.starts_with("http") {
        anyhow::bail!("Cannot update '{name}' — no remote source URL in lockfile");
    }

    println!(
        "  {} Updating third-party skill '{}' from {}...",
        console::style("→").cyan().bold(),
        name,
        source_url,
    );

    let skills_path = skills_dir(workspace_dir);
    let skill_dir = skills_path.join(name);

    // Clone to temp directory first (atomic update)
    let temp_dir = skill_dir.with_extension("tmp");
    let _ = std::fs::remove_dir_all(&temp_dir); // Clean any leftover temp

    let clone_status = std::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            source_url,
            temp_dir.to_str().unwrap_or("corvus-skill-tmp"),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match clone_status {
        Ok(s) if s.success() => {
            // Remove .git from temp
            let git_dir = temp_dir.join(".git");
            if git_dir.exists() {
                let _ = std::fs::remove_dir_all(&git_dir);
            }
            // Atomic swap: remove old, rename temp
            if skill_dir.exists() {
                std::fs::remove_dir_all(&skill_dir)?;
            }
            std::fs::rename(&temp_dir, &skill_dir)?;
        }
        _ => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            anyhow::bail!("Failed to clone '{source_url}' for update");
        }
    }

    // Validate after update
    let skill_md = skill_dir.join("SKILL.md");
    if !skill_md.exists() {
        tracing::warn!("updated skill '{name}' is missing SKILL.md");
    }

    // Run scanner on updated content
    if let Ok(content) = std::fs::read_to_string(&skill_md) {
        let scan = scanner::scan_skill_content(&content);
        if scan.exceeds_threshold(scanner::DEFAULT_SCAN_THRESHOLD) {
            tracing::warn!(
                "updated skill '{}' scored {} in injection scan \
                 (threshold: {}). Review the skill content.",
                name,
                scan.score,
                scanner::DEFAULT_SCAN_THRESHOLD,
            );
        }
    }

    // Update lockfile
    let hash = if skill_md.exists() {
        let content = std::fs::read_to_string(&skill_md)?;
        Some(lockfile::compute_content_hash(content.as_bytes()))
    } else {
        None
    };

    let new_entry = lockfile::build_lock_entry(
        trust::SkillTrust::ThirdParty,
        source_url,
        None,
        hash,
        entry.allowed_tools.clone(),
        None,
    );
    lockfile::write_lock_entry(workspace_dir, name, new_entry)?;

    println!(
        "  {} '{}' updated successfully.",
        console::style("✓").green().bold(),
        name,
    );
    Ok(())
}

fn handle_remove_command(workspace_dir: &Path, name: &str) -> Result<()> {
    validate_skill_name_path_safety(name)?;

    let skills_path = skills_dir(workspace_dir);
    let skill_path = skills_path.join(name);
    ensure_skill_path_stays_within_root(&skills_path, &skill_path, name)?;

    if !skill_path.exists() {
        anyhow::bail!("Skill not found: {name}");
    }

    std::fs::remove_dir_all(&skill_path)?;

    // Clean lockfile entry (advisory — failure doesn't block removal)
    if let Err(err) = lockfile::remove_lock_entry(workspace_dir, name) {
        tracing::warn!("failed to remove lockfile entry for '{name}': {err}");
    }

    println!(
        "  {} Skill '{}' removed.",
        console::style("✓").green().bold(),
        name
    );
    Ok(())
}

fn validate_skill_name_path_safety(name: &str) -> Result<()> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        anyhow::bail!("Invalid skill name: {name}");
    }

    Ok(())
}

fn ensure_skill_path_stays_within_root(root: &Path, skill_path: &Path, name: &str) -> Result<()> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    if let Ok(canonical_skill) = skill_path.canonicalize() {
        if !canonical_skill.starts_with(&canonical_root) {
            anyhow::bail!("Skill path escapes skills directory: {name}");
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_empty_skills_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills = load_skills(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn load_skill_from_md() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        let skill_dir = skills_dir.join("md-skill");
        fs::create_dir_all(&skill_dir).unwrap();

        fs::write(
            skill_dir.join("SKILL.md"),
            "# My Skill\nThis skill does cool things.\n",
        )
        .unwrap();

        let skills = load_skills(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "md-skill");
        assert!(skills[0].description.contains("cool things"));
    }

    #[test]
    fn skills_to_prompt_empty() {
        let prompt = skills_to_prompt(&[]);
        assert!(prompt.is_empty());
    }

    #[test]
    fn skills_to_prompt_with_skills() {
        let skills = vec![Skill {
            name: "test".to_string(),
            description: "A test".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec!["Do the thing.".to_string()],
            location: None,
            trust: trust::SkillTrust::Local,
            origin: trust::SkillOrigin::default(),
            allowed_tools: Vec::new(),
        }];
        let prompt = skills_to_prompt(&skills);
        assert!(prompt.contains("test"));
        assert!(prompt.contains("Do the thing"));
    }

    #[test]
    fn init_skills_creates_readme() {
        let dir = tempfile::tempdir().unwrap();
        init_skills_dir(dir.path()).unwrap();
        assert!(dir.path().join("skills").join("README.md").exists());
    }

    #[test]
    fn init_skills_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        init_skills_dir(dir.path()).unwrap();
        init_skills_dir(dir.path()).unwrap(); // second call should not fail
        assert!(dir.path().join("skills").join("README.md").exists());
    }

    #[test]
    fn load_nonexistent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("nonexistent");
        let skills = load_skills(&fake);
        assert!(skills.is_empty());
    }

    #[test]
    fn load_ignores_files_in_skills_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        // A file, not a directory — should be ignored
        fs::write(skills_dir.join("not-a-skill.txt"), "hello").unwrap();
        let skills = load_skills(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn load_ignores_dir_without_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        let empty_skill = skills_dir.join("empty-skill");
        fs::create_dir_all(&empty_skill).unwrap();
        // Directory exists but no SKILL.md
        let skills = load_skills(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn load_multiple_skills() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        for name in ["alpha", "beta", "gamma"] {
            let skill_dir = skills_dir.join(name);
            fs::create_dir_all(&skill_dir).unwrap();
            fs::write(
                skill_dir.join("SKILL.md"),
                format!("# {name}\nSkill {name} description.\n"),
            )
            .unwrap();
        }

        let skills = load_skills(dir.path());
        assert_eq!(skills.len(), 3);
    }

    #[test]
    fn md_skill_heading_only() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        let skill_dir = skills_dir.join("heading-only");
        fs::create_dir_all(&skill_dir).unwrap();

        fs::write(skill_dir.join("SKILL.md"), "# Just a Heading\n").unwrap();

        let skills = load_skills(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].description, "No description");
    }

    #[test]
    fn skills_to_prompt_includes_tools() {
        let skills = vec![Skill {
            name: "weather".to_string(),
            description: "Get weather".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![SkillTool {
                name: "get_weather".to_string(),
                description: "Fetch forecast".to_string(),
                kind: "shell".to_string(),
                command: "curl wttr.in".to_string(),
                args: HashMap::new(),
                sandboxed: false,
            }],
            prompts: vec![],
            location: None,
            trust: trust::SkillTrust::Local,
            origin: trust::SkillOrigin::default(),
            allowed_tools: Vec::new(),
        }];
        let prompt = skills_to_prompt(&skills);
        assert!(prompt.contains("weather"));
        assert!(prompt.contains("get_weather"));
        assert!(prompt.contains("Fetch forecast"));
        assert!(prompt.contains("shell"));
    }

    #[test]
    fn skills_dir_path() {
        let base = std::path::Path::new("/home/user/.corvus");
        let dir = skills_dir(base);
        assert_eq!(dir, PathBuf::from("/home/user/.corvus/skills"));
    }

    #[test]
    fn remote_skill_source_requires_https() {
        assert!(is_remote_skill_source("https://example.com/skill.git"));
        assert!(!is_remote_skill_source("http://example.com/skill.git"));
    }

    #[test]
    fn install_local_skill_rejects_invalid_source_name() {
        let temp = tempfile::tempdir().unwrap();
        let skills_path = temp.path().join("skills");
        fs::create_dir_all(&skills_path).unwrap();

        let error = install_local_skill(&skills_path, "/").unwrap_err();
        assert!(error.to_string().contains("Invalid skill source path"));
    }

    // ── Catalog install rejects unknown skill (R20.3) ────────────

    #[test]
    fn catalog_install_rejects_unknown_skill() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("skills")).unwrap();
        // handle_catalog_install should fail for a non-existent skill
        let config = crate::config::SkillsConfig::default();
        let result = handle_catalog_install(dir.path(), "nonexistent-skill-xyz", &config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found") || err.contains("catalog"),
            "error should mention 'not found' or 'catalog', got: {err}",
        );
    }

    // ── SKILL.toml-only directory is skipped on load (R20.4) ─────

    #[test]
    fn toml_only_directory_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(skills_dir.join("toml-only")).unwrap();
        fs::write(
            skills_dir.join("toml-only/SKILL.toml"),
            "[skill]\nname = \"toml-only\"\n",
        )
        .unwrap();
        // No SKILL.md — should be skipped
        let skills = load_skills(dir.path());
        assert!(
            skills.iter().all(|s| s.name != "toml-only"),
            "SKILL.toml-only directory should not appear in loaded skills",
        );
    }
}

#[cfg(test)]
mod symlink_tests;
