# Design: Skills Hardening (Phase 3)

## Technical Approach

Phase 3 completes the skills security posture by removing deprecated code paths, enforcing content
integrity at load time, adding prompt injection scanning, validating skill names, and sandboxing
third-party tool execution. The implementation follows the proposal's three priority tiers (P0 →
P1 → P2) and touches primarily `clients/agent-runtime/src/skills/` with minor changes in
`config/schema.rs`.

The strategy is: **delete first, harden second, gate third**. Removing open-skills and SKILL.toml
eliminates ~300 lines of dead attack surface before any new code is added. Integrity verification,
name validation, and the scanner are then layered into the existing `load_skills_with_config()` and
`handle_install_command()` paths. Sandboxing wraps the tool executor for ThirdParty skills.

## Architecture Decisions

### AD1: Integrity as Warning-and-Downgrade (not Block)

**Choice**: Hash mismatch warns and downgrades ThirdParty skills to instruction-only (clears
`allowed_tools`). Official/Local mismatches warn only.

**Alternatives considered**: (A) Block load entirely on mismatch — rejected because a stale
lockfile after manual edit would break the user's setup. (B) Warn only, no downgrade — rejected
because it provides no security benefit for ThirdParty skills.

**Rationale**: Corvus is a single-user tool where availability matters more than strict consistency.
The lockfile is advisory (AD2 from Phase 1). Downgrading trust on mismatch limits blast radius
without breaking the runtime. Users can run `corvus skills lock repair` to re-hash after
intentional edits. The `verify_integrity` config flag provides an escape hatch.

### AD2: Removal Over Deprecation

**Choice**: Delete all open-skills and SKILL.toml code paths entirely — no feature flags, no
fallback.

**Alternatives considered**: (A) Keep code behind `cfg` flags — rejected because dead code is
attack surface and maintenance burden. (B) Move to a separate crate — rejected because there are
no consumers.

**Rationale**: Both features have been deprecated for two full phases with warnings. Open-skills
was default-OFF since Phase 1. The official catalog is the replacement. The `directories` crate
dependency can be removed if unused elsewhere after deletion (it's still used in `config/schema.rs`
for `UserDirs`, so it stays).

### AD3: Scoring-Based Scanner (not Binary)

**Choice**: Scanner produces a numeric risk score from accumulated findings. Threshold is
configurable; default is 50.

**Alternatives considered**: (A) Binary pass/fail on any pattern match — rejected because
legitimate skills routinely contain phrases like "act as a code reviewer" or "you are now going
to process PDFs". (B) ML-based classifier — rejected as overkill for this phase and adds a heavy
dependency.

**Rationale**: Scoring with per-category severity weights lets us tune sensitivity without code
changes. Install-time blocks above threshold (hard gate with user-visible findings report).
Load-time warns and downgrades trust above threshold (soft gate). Default threshold of 50 requires
multiple findings or one high-severity match, reducing false positives on clean skills.

### AD4: Sandbox as Path Validation (not OS-Level)

**Choice**: Sandbox = set `cwd` to skill directory + validate all path arguments stay within scope

+ resolve symlinks. No seccomp/landlock/pledge.

**Alternatives considered**: (A) OS-level sandboxing with seccomp (Linux) and sandbox-exec
(macOS) — rejected as too complex and platform-specific for this phase. (B) Docker container per
tool execution — rejected as too heavy for interactive use.

**Rationale**: Path validation is defense-in-depth, not a hard boundary. It catches the common
case (accidental or naive traversal) and raises the bar for targeted attacks. OS-level sandboxing
is deferred to Phase 4 where it can be designed and tested properly per platform.

## Data Flow

### Load Flow with Integrity Verification

```
load_skills_with_config(workspace_dir, config)
    │
    ├── [REMOVED: ensure_open_skills_repo()]
    │
    ├── read_lockfile(workspace_dir)
    │
    ├── load_workspace_skills(workspace_dir)
    │     │
    │     └── load_skills_from_directory(skills_dir)
    │           │
    │           for each subdirectory:
    │           ├── validate_skill_name(dir_name)
    │           │     └── invalid? → warn + skip
    │           │
    │           ├── [REMOVED: SKILL.toml branch]
    │           │     └── SKILL.toml only? → error log + migration msg + skip
    │           │
    │           ├── SKILL.md exists? → load_skill_md()
    │           │
    │           └── validate_name_matches_directory(fm.name, dir_name)
    │                 └── mismatch? → warn + skip
    │
    ├── for each skill:
    │     ├── enrich from lockfile (trust, origin, allowed_tools)
    │     │
    │     ├── if config.verify_integrity:
    │     │     ├── read SKILL.md bytes
    │     │     ├── compute_content_hash()
    │     │     ├── compare vs lockfile entry.content_hash
    │     │     └── mismatch?
    │     │           ├── ThirdParty → warn + clear allowed_tools
    │     │           └── Official/Local → warn only
    │     │
    │     └── if config.scan_threshold.is_some():
    │           ├── scan_skill_content(skill_md_content)
    │           └── score > threshold?
    │                 ├── ThirdParty → warn + clear allowed_tools
    │                 └── Official/Local → warn only
    │
    └── return skills
```

### Install Flow with Scanner Gate

```
handle_install_command(workspace_dir, source, trust_flag)
    │
    ├── catalog name? → handle_catalog_install()
    │
    ├── resolve_skill_source()
    │
    ├── clone / symlink skill
    │
    ├── validate SKILL.md exists
    │
    ├── validate_skill_name(fm.name)           ← NEW
    │     └── invalid? → remove dir + bail
    │
    ├── validate_name_matches_directory()       (existing)
    │
    ├── scan_skill_content(content)             ← NEW
    │     └── score > threshold?
    │           └── remove dir + bail with findings report
    │
    ├── trust gate (existing --trust check)
    │
    ├── compute_content_hash + write lockfile
    │
    └── success message
```

### Tool Execution with Sandbox Check

```
execute_skill_tool(skill, tool, args)
    │
    ├── filter_tools_by_trust(skill)
    │
    ├── if skill.trust == ThirdParty && tool.kind == "shell":
    │     ├── build SandboxPolicy { skill_dir, allowed_paths }
    │     ├── validate_tool_paths(args, policy)
    │     │     └── violation? → reject with error
    │     └── set command.cwd = skill_dir
    │
    └── execute command
```

## File Changes

| File                                              | Action | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
|---------------------------------------------------|--------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/skills/mod.rs`         | Modify | Remove open-skills code (~200 lines: constants, `load_open_skills`, `open_skills_enabled`, `resolve_open_skills_dir`, `ensure_open_skills_repo`, `clone_open_skills_repo`, `pull_open_skills_repo`, `should_sync_open_skills`, `mark_open_skills_synced`, `load_open_skill_md`). Remove SKILL.toml code (~80 lines: `SkillManifest`, `SkillMeta`, `default_version`, `load_skill_toml`, SKILL.toml branch in `load_skills_from_directory`). Add integrity verification and scanner calls in `load_skills_with_config`. Add name validation calls in `load_skills_from_directory` and `handle_install_command`. Update `init_skills_dir` README to remove SKILL.toml references. Remove SKILL.toml tests (~6 tests). Add `pub mod validation;` and `pub mod scanner;` and `pub mod sandbox;`. |
| `clients/agent-runtime/src/skills/scanner.rs`     | Create | Prompt injection scoring scanner: `ScanResult`, `ScanFinding`, `ScanCategory`, `scan_skill_content()`, pattern matchers, threshold comparison.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `clients/agent-runtime/src/skills/validation.rs`  | Create | Name validation: `validate_skill_name()`, `validate_name_matches_directory()`, `SkillValidationError`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `clients/agent-runtime/src/skills/sandbox.rs`     | Create | Tool sandboxing: `SandboxPolicy`, `SandboxViolation`, `validate_tool_paths()`, `apply_sandbox()`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `clients/agent-runtime/src/skills/lockfile.rs`    | Modify | Add `verify_integrity()` function. Remove `SKILL.toml` existence check from `repair_lockfile` (line 151-153).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `clients/agent-runtime/src/config/schema.rs`      | Modify | Remove `legacy_open_skills` from `SkillsConfig`. Add `verify_integrity: bool` (default true) and `scan_threshold: Option<u32>` (default Some(50)).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `clients/agent-runtime/src/skills/trust.rs`       | None   | No changes. Trust derivation remains as-is.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `clients/agent-runtime/src/skills/frontmatter.rs` | None   | No changes. Validation is external to parsing (separation of concerns).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `clients/agent-runtime/src/skills/catalog.rs`     | None   | No changes expected.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

## Interfaces / Contracts

### `skills/validation.rs`

```rust
use std::fmt;

/// Error returned when a skill name fails validation.
#[derive(Debug, Clone)]
pub enum SkillValidationError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidCharacter { char: char, position: usize },
    LeadingHyphen,
    TrailingHyphen,
    ConsecutiveHyphens,
    NameDirectoryMismatch { name: String, directory: String },
}

impl fmt::Display for SkillValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "skill name must not be empty"),
            Self::TooLong { len, max } => {
                write!(f, "skill name is {len} chars, max is {max}")
            }
            Self::InvalidCharacter { char, position } => {
                write!(f, "invalid character '{char}' at position {position} \
                       (only [a-z0-9-] allowed)")
            }
            Self::LeadingHyphen => write!(f, "skill name must not start with a hyphen"),
            Self::TrailingHyphen => write!(f, "skill name must not end with a hyphen"),
            Self::ConsecutiveHyphens => {
                write!(f, "skill name must not contain consecutive hyphens")
            }
            Self::NameDirectoryMismatch { name, directory } => {
                write!(f, "skill name '{name}' does not match directory '{directory}'")
            }
        }
    }
}

impl std::error::Error for SkillValidationError {}

const MAX_SKILL_NAME_LEN: usize = 64;

/// Validate a skill name against the Agent Skills standard.
/// Rules: 1-64 chars, [a-z0-9-], no leading/trailing hyphens, no consecutive hyphens.
pub fn validate_skill_name(name: &str) -> Result<(), SkillValidationError> {
    if name.is_empty() {
        return Err(SkillValidationError::Empty);
    }
    if name.len() > MAX_SKILL_NAME_LEN {
        return Err(SkillValidationError::TooLong {
            len: name.len(),
            max: MAX_SKILL_NAME_LEN,
        });
    }
    if name.starts_with('-') {
        return Err(SkillValidationError::LeadingHyphen);
    }
    if name.ends_with('-') {
        return Err(SkillValidationError::TrailingHyphen);
    }
    if name.contains("--") {
        return Err(SkillValidationError::ConsecutiveHyphens);
    }
    for (i, c) in name.chars().enumerate() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(SkillValidationError::InvalidCharacter {
                char: c,
                position: i,
            });
        }
    }
    Ok(())
}

/// Check that frontmatter `name` matches the directory name.
pub fn validate_name_matches_directory(
    name: &str,
    dir_name: &str,
) -> Result<(), SkillValidationError> {
    if name != dir_name {
        return Err(SkillValidationError::NameDirectoryMismatch {
            name: name.to_string(),
            directory: dir_name.to_string(),
        });
    }
    Ok(())
}

/// Convenience: returns true if the name is valid.
pub fn is_valid_skill_name(name: &str) -> bool {
    validate_skill_name(name).is_ok()
}
```

### `skills/scanner.rs`

```rust
use std::sync::LazyLock;
use regex::Regex;

/// Category of a scan finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCategory {
    /// "ignore previous", "forget instructions", "new system prompt"
    SystemPromptOverride,
    /// "you are now", "act as", "your new role"
    RoleManipulation,
    /// "this skill is official", "trust level: official", "bypass trust"
    TrustEscalation,
    /// Base64 blocks above threshold length, encoded instructions
    EncodedPayload,
    /// Zero-width chars, homoglyphs, invisible Unicode
    UnicodeAnomaly,
    /// Excessive tool declarations (> 20)
    ExcessiveTools,
}

/// A single finding from the scanner.
#[derive(Debug, Clone)]
pub struct ScanFinding {
    pub category: ScanCategory,
    pub pattern: String,
    pub line: usize,
    pub severity: u32,
}

/// Result of scanning a skill's content.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub score: u32,
    pub findings: Vec<ScanFinding>,
}

impl ScanResult {
    /// Returns true if the score exceeds the given threshold.
    pub fn exceeds_threshold(&self, threshold: u32) -> bool {
        self.score > threshold
    }

    /// Returns true if no findings were detected.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Default scan threshold. Requires multiple low-severity matches or one high-severity match.
pub const DEFAULT_SCAN_THRESHOLD: u32 = 50;

/// Severity weights per category.
const SEVERITY_SYSTEM_PROMPT_OVERRIDE: u32 = 40;
const SEVERITY_TRUST_ESCALATION: u32 = 40;
const SEVERITY_ROLE_MANIPULATION: u32 = 15;
const SEVERITY_ENCODED_PAYLOAD: u32 = 30;
const SEVERITY_UNICODE_ANOMALY: u32 = 25;
const SEVERITY_EXCESSIVE_TOOLS: u32 = 20;

/// Minimum base64 block length to flag (chars). Short blocks are common in docs.
const BASE64_MIN_LENGTH: usize = 200;

struct PatternSet {
    category: ScanCategory,
    severity: u32,
    patterns: Vec<Regex>,
}

static PATTERN_SETS: LazyLock<Vec<PatternSet>> = LazyLock::new(|| {
    vec![
        PatternSet {
            category: ScanCategory::SystemPromptOverride,
            severity: SEVERITY_SYSTEM_PROMPT_OVERRIDE,
            patterns: vec![
                Regex::new(r"(?i)ignore\s+(all\s+)?previous\s+instructions").unwrap(),
                Regex::new(r"(?i)forget\s+(all\s+)?(your\s+)?instructions").unwrap(),
                Regex::new(r"(?i)new\s+system\s+prompt").unwrap(),
                Regex::new(r"(?i)disregard\s+(all\s+)?prior").unwrap(),
                Regex::new(r"(?i)override\s+system\s+prompt").unwrap(),
            ],
        },
        PatternSet {
            category: ScanCategory::RoleManipulation,
            severity: SEVERITY_ROLE_MANIPULATION,
            patterns: vec![
                Regex::new(r"(?i)you\s+are\s+now\s+(an?\s+)?(?:unrestricted|unfiltered|jailbroken)").unwrap(),
                Regex::new(r"(?i)your\s+new\s+role\s+is").unwrap(),
                Regex::new(r"(?i)pretend\s+you\s+are\s+(?:not\s+)?an?\s+ai").unwrap(),
            ],
        },
        PatternSet {
            category: ScanCategory::TrustEscalation,
            severity: SEVERITY_TRUST_ESCALATION,
            patterns: vec![
                Regex::new(r"(?i)this\s+skill\s+is\s+official").unwrap(),
                Regex::new(r"(?i)trust\s+level[:\s]+official").unwrap(),
                Regex::new(r"(?i)bypass\s+trust").unwrap(),
                Regex::new(r"(?i)escalate\s+(?:to\s+)?(?:official|admin)").unwrap(),
                Regex::new(r"(?i)grant\s+(?:full|all)\s+(?:access|permissions)").unwrap(),
            ],
        },
        PatternSet {
            category: ScanCategory::EncodedPayload,
            severity: SEVERITY_ENCODED_PAYLOAD,
            // Handled separately via base64 block detection
            patterns: vec![],
        },
        PatternSet {
            category: ScanCategory::UnicodeAnomaly,
            severity: SEVERITY_UNICODE_ANOMALY,
            // Handled separately via char-level scan
            patterns: vec![],
        },
    ]
});

/// Scan skill content for prompt injection patterns. Returns a scored result.
pub fn scan_skill_content(content: &str) -> ScanResult {
    let mut findings = Vec::new();

    // Regex-based pattern matching
    for set in PATTERN_SETS.iter() {
        for pattern in &set.patterns {
            for (line_idx, line) in content.lines().enumerate() {
                if let Some(m) = pattern.find(line) {
                    findings.push(ScanFinding {
                        category: set.category,
                        pattern: m.as_str().to_string(),
                        line: line_idx + 1,
                        severity: set.severity,
                    });
                }
            }
        }
    }

    // Base64 block detection
    scan_base64_blocks(content, &mut findings);

    // Unicode anomaly detection (zero-width chars, homoglyphs)
    scan_unicode_anomalies(content, &mut findings);

    let score: u32 = findings.iter().map(|f| f.severity).sum();

    ScanResult { score, findings }
}

fn scan_base64_blocks(content: &str, findings: &mut Vec<ScanFinding>) {
    static BASE64_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[A-Za-z0-9+/]{200,}={0,3}").unwrap()
    });

    for (line_idx, line) in content.lines().enumerate() {
        for m in BASE64_RE.find_iter(line) {
            if m.as_str().len() >= BASE64_MIN_LENGTH {
                findings.push(ScanFinding {
                    category: ScanCategory::EncodedPayload,
                    pattern: format!(
                        "base64 block ({} chars)",
                        m.as_str().len()
                    ),
                    line: line_idx + 1,
                    severity: SEVERITY_ENCODED_PAYLOAD,
                });
            }
        }
    }
}

fn scan_unicode_anomalies(content: &str, findings: &mut Vec<ScanFinding>) {
    for (line_idx, line) in content.lines().enumerate() {
        for c in line.chars() {
            // Zero-width characters
            if matches!(c,
                '\u{200B}'  // zero-width space
                | '\u{200C}' // zero-width non-joiner
                | '\u{200D}' // zero-width joiner
                | '\u{FEFF}' // BOM / zero-width no-break space
                | '\u{2060}' // word joiner
                | '\u{180E}' // Mongolian vowel separator
            ) {
                findings.push(ScanFinding {
                    category: ScanCategory::UnicodeAnomaly,
                    pattern: format!("zero-width character U+{:04X}", c as u32),
                    line: line_idx + 1,
                    severity: SEVERITY_UNICODE_ANOMALY,
                });
                break; // one finding per line is enough
            }
        }
    }
}
```

### `skills/lockfile.rs` — New `verify_integrity` function

```rust
/// Result of verifying a skill's content integrity against the lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityResult {
    /// Hash matches lockfile — content unchanged.
    Match,
    /// Hash mismatch — content was modified since install.
    Mismatch {
        expected: String,
        actual: String,
    },
    /// No hash in lockfile — cannot verify (e.g., old lockfile entry).
    NoBaseline,
    /// Verification disabled by config.
    Disabled,
}

/// Verify the integrity of a skill's SKILL.md against the lockfile hash.
pub fn verify_integrity(
    skill_md_path: &Path,
    lockfile_hash: Option<&str>,
    enabled: bool,
) -> IntegrityResult {
    if !enabled {
        return IntegrityResult::Disabled;
    }

    let Some(expected) = lockfile_hash else {
        return IntegrityResult::NoBaseline;
    };

    let Ok(content) = std::fs::read(skill_md_path) else {
        return IntegrityResult::NoBaseline;
    };

    let actual = compute_content_hash(&content);

    if actual == expected {
        IntegrityResult::Match
    } else {
        IntegrityResult::Mismatch {
            expected: expected.to_string(),
            actual,
        }
    }
}
```

### `skills/sandbox.rs`

```rust
use std::path::{Path, PathBuf};
use std::fmt;

/// Sandbox policy for tool execution.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Whether sandboxing is enabled for this skill.
    pub enabled: bool,
    /// Allowed path prefixes (canonicalized). Typically just the skill directory.
    pub allowed_paths: Vec<PathBuf>,
}

/// Violation detected during sandbox validation.
#[derive(Debug, Clone)]
pub enum SandboxViolation {
    /// Path contains traversal sequences before canonicalization.
    TraversalSequence { path: String },
    /// Canonicalized path escapes allowed scope.
    PathEscape { path: PathBuf, allowed: Vec<PathBuf> },
    /// Symlink resolves outside allowed scope.
    SymlinkEscape { link: PathBuf, target: PathBuf },
}

impl fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraversalSequence { path } => {
                write!(f, "path traversal detected: {path}")
            }
            Self::PathEscape { path, .. } => {
                write!(f, "path escapes sandbox: {}", path.display())
            }
            Self::SymlinkEscape { link, target } => {
                write!(
                    f, "symlink {} resolves outside sandbox to {}",
                    link.display(), target.display()
                )
            }
        }
    }
}

impl std::error::Error for SandboxViolation {}

/// Validate that all path arguments stay within the sandbox.
pub fn validate_tool_paths(
    args: &[String],
    policy: &SandboxPolicy,
) -> Result<(), SandboxViolation> {
    if !policy.enabled {
        return Ok(());
    }

    for arg in args {
        // Defense-in-depth: reject raw traversal sequences before canonicalization
        if arg.contains("../") || arg.contains("..\\") {
            return Err(SandboxViolation::TraversalSequence {
                path: arg.clone(),
            });
        }

        // Skip non-path-like arguments (flags, simple values)
        if arg.starts_with('-') || !arg.contains('/') && !arg.contains('\\') {
            continue;
        }

        let candidate = Path::new(arg);

        // Attempt to canonicalize; if it fails (doesn't exist yet), validate components
        if let Ok(canonical) = candidate.canonicalize() {
            if !is_within_allowed(&canonical, &policy.allowed_paths) {
                return Err(SandboxViolation::PathEscape {
                    path: canonical,
                    allowed: policy.allowed_paths.clone(),
                });
            }
        }

        // Check symlinks
        if candidate.is_symlink() {
            if let Ok(target) = std::fs::read_link(candidate) {
                if let Ok(resolved) = target.canonicalize() {
                    if !is_within_allowed(&resolved, &policy.allowed_paths) {
                        return Err(SandboxViolation::SymlinkEscape {
                            link: candidate.to_path_buf(),
                            target: resolved,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

/// Apply sandbox constraints to a Command before execution.
pub fn apply_sandbox(
    command: &mut std::process::Command,
    policy: &SandboxPolicy,
) {
    if !policy.enabled {
        return;
    }

    // Set cwd to the first allowed path (skill directory)
    if let Some(skill_dir) = policy.allowed_paths.first() {
        command.current_dir(skill_dir);
    }
}

/// Build a sandbox policy for a skill.
pub fn build_policy(
    trust: super::trust::SkillTrust,
    skill_dir: &Path,
) -> SandboxPolicy {
    let enabled = trust == super::trust::SkillTrust::ThirdParty;
    let allowed_paths = if enabled {
        // Canonicalize skill_dir for reliable prefix matching
        let canonical = skill_dir.canonicalize().unwrap_or_else(|_| skill_dir.to_path_buf());
        vec![canonical]
    } else {
        Vec::new()
    };
    SandboxPolicy {
        enabled,
        allowed_paths,
    }
}

fn is_within_allowed(path: &Path, allowed: &[PathBuf]) -> bool {
    allowed.iter().any(|prefix| path.starts_with(prefix))
}
```

### Updated `config/schema.rs` — `SkillsConfig`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    // REMOVED: legacy_open_skills

    /// Override the official skills catalog repository URL.
    #[serde(default)]
    pub catalog_repo_url: Option<String>,

    /// Cache TTL in hours for the catalog index (default: 24).
    #[serde(default)]
    pub catalog_cache_ttl_hours: Option<u64>,

    /// Verify SKILL.md content hash against lockfile on load (default: true).
    #[serde(default = "default_true")]
    pub verify_integrity: bool,

    /// Prompt injection scan threshold. None = scanning disabled.
    /// Skills scoring above this value are blocked on install and
    /// downgraded to instruction-only on load. Default: 50.
    #[serde(default = "default_scan_threshold")]
    pub scan_threshold: Option<u32>,
}

fn default_scan_threshold() -> Option<u32> {
    Some(50)
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            catalog_repo_url: None,
            catalog_cache_ttl_hours: None,
            verify_integrity: true,
            scan_threshold: default_scan_threshold(),
        }
    }
}
```

## Testing Strategy

| Layer                  | What to Test                                                                                                       | Approach                                                             |
|------------------------|--------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|
| Unit                   | `validate_skill_name` — valid names, invalid chars, empty, too long, leading/trailing hyphens, consecutive hyphens | Direct function calls with assertions in `validation.rs` tests       |
| Unit                   | `validate_name_matches_directory` — match, mismatch                                                                | Direct function calls                                                |
| Unit                   | `scan_skill_content` — each pattern category triggers correct finding and severity                                 | Craft minimal SKILL.md content per category                          |
| Unit                   | Scanner false-positive check — clean legitimate SKILL.md scores 0                                                  | Use a real skill from `.opencode/skills/` as input                   |
| Unit                   | Scanner threshold logic — `exceeds_threshold` for boundary values                                                  | `ScanResult` with known scores                                       |
| Unit                   | `verify_integrity` — match, mismatch, missing hash, disabled                                                       | Construct `LockEntry` with known hashes, write test SKILL.md files   |
| Unit                   | `validate_tool_paths` — clean paths pass, `../` rejected, absolute escape rejected, symlink escape detected        | `tempfile` dirs with controlled symlinks                             |
| Unit                   | `build_policy` — ThirdParty enables sandbox, Local/Official disables                                               | Direct calls with trust tiers                                        |
| Unit (deferred)        | `resolve_index` — cache hit, cache miss, fetch failure, embedded fallback (task 4.2)                               | Mock HTTP or use `test_catalog_cache` helper                         |
| Unit (deferred)        | `repair_lockfile` — added, removed, updated, unchanged (task 4.6)                                                  | `tempfile` workspace with varied skill/lockfile states               |
| Integration (deferred) | `handle_catalog_install` end-to-end (task 4.7)                                                                     | Mock git clone with local fixture repo                               |
| Integration (deferred) | SKILL.toml rejection with migration error (task 4.8, adapted)                                                      | Create SKILL.toml-only dir, assert error log and skip                |
| Regression             | All existing tests pass after open-skills and SKILL.toml removal                                                   | `cargo test` — tests referencing removed code are deleted or adapted |
| Regression             | `cargo clippy --all-targets -- -D warnings` clean                                                                  | CI check                                                             |

### Tests to Delete

- `load_skill_from_toml` — tests SKILL.toml loading (removed)
- `toml_skill_with_multiple_tools` — tests SKILL.toml multi-tool (removed)
- `toml_skill_minimal` — tests SKILL.toml minimal (removed)
- `toml_skill_invalid_syntax_skipped` — tests SKILL.toml error handling (removed)
- `toml_prefers_over_md` — tests SKILL.toml priority over SKILL.md (removed)

### Tests to Adapt

- `load_ignores_dir_without_manifest` — keep; now only checks for SKILL.md absence
- `md_skill_heading_only` — keep as-is

## Migration / Rollout

### SKILL.toml Migration

Users with SKILL.toml-only skills will see an error log on `load_skills`:

```
ERROR skill 'my-skill' uses SKILL.toml which is no longer supported.
      Migrate to SKILL.md with YAML frontmatter:
      1. Create my-skill/SKILL.md with frontmatter: ---\nname: my-skill\n...
      2. Copy your skill content below the frontmatter
      3. Delete SKILL.toml
      See: https://agentskills.io/specification
```

No automatic migration tool in this phase (deferred to Phase 4).

### Config Migration

- `legacy_open_skills: true` in existing config files will cause a deserialization error due to
  `deny_unknown_fields` on `Config`. This is intentional — users must remove the field.
  However, `SkillsConfig` does NOT use `deny_unknown_fields`, so this will be silently ignored
  via serde default behavior. We should emit a one-time warning if the raw TOML contains
  `legacy_open_skills` — but that requires custom deserialization which is out of scope. Instead,
  document in release notes.

### Rollback

Each component can be individually disabled or reverted:

| Component              | Disable without code change       | Code revert                                        |
|------------------------|-----------------------------------|----------------------------------------------------|
| Integrity verification | `skills.verify_integrity = false` | Remove hash check from `load_skills_with_config`   |
| Scanner                | `skills.scan_threshold = null`    | Remove `scan_skill_content` calls                  |
| Name validation        | N/A (no config toggle)            | Remove validation calls from load/install paths    |
| Sandbox                | N/A (only applies to ThirdParty)  | Remove `apply_sandbox`/`validate_tool_paths` calls |
| Open-skills removal    | Revert deletion commits           | Restores dead code                                 |
| SKILL.toml removal     | Revert deletion commits           | Restores deprecated path                           |

## Open Questions

- [x] Hash mismatch behavior — **Decided**: warn + downgrade (AD1)
- [x] Name validation timing — **Decided**: both install (reject) and load (warn+skip)
- [x] Scanner threshold default — **Decided**: 50 (tunable via config)
- [x] Sandbox scope — **Decided**: path validation only, no OS-level (AD4)
- [ ] `SkillsConfig` uses `#[serde(default)]` without `deny_unknown_fields` — removing
  `legacy_open_skills` won't break existing configs, but users won't get a warning either.
  Accept this or add a custom deserializer? **Recommendation**: accept, document in release notes.
- [ ] Should `RoleManipulation` patterns exclude matches inside code blocks (triple-backtick
  fences)? Legitimate skills may document injection patterns. **Recommendation**: defer to
  threshold tuning; code-block-aware parsing adds complexity.
