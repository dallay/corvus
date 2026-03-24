# Proposal: Skills Hardening (Phase 3)

## Intent

Phases 1 and 2 of the skills trust model shipped the trust enum, lockfile, catalog index,
install/search/update commands, and deprecation warnings for SKILL.toml and open-skills. Phase 3
completes the security posture by removing deprecated code paths (open-skills, SKILL.toml),
enforcing content integrity at load time, adding prompt injection scanning, validating skill names
against the Agent Skills standard, sandboxing third-party tool execution, and closing deferred
test gaps. These are the final hardening steps before the skills system can be considered
production-grade for untrusted third-party content.

## Scope

### In Scope

**P0 — Security Critical:**

1. **Content integrity verification on load** — Re-hash SKILL.md on every `load_skills()` call
   and compare against the lockfile `content_hash`. On mismatch for ThirdParty skills: warn and
   downgrade trust to instruction-only (no tools). On mismatch for Official/Local: warn only.
   New config flag: `skills.verify_integrity` (default: `true`).

2. **Open-skills removal** — Delete all `besoeasy/open-skills` code: constants
   (`OPEN_SKILLS_REPO_URL`, sync marker, interval), `ensure_open_skills_repo()`,
   `clone_open_skills_repo()`, `sync_open_skills()`, `load_open_skills()`,
   `load_open_skill_md()`, `resolve_open_skills_dir()`, env var handling for
   `CORVUS_OPEN_SKILLS_ENABLED` / `CORVUS_OPEN_SKILLS`. Remove `legacy_open_skills` from
   `SkillsConfig`. Remove `open_skills_enabled()`. (~200 lines deleted.)

**P1 — High Value:**

3. **SKILL.toml removal** — Remove `SkillManifest`, `SkillMeta`, `load_skill_toml()`,
   `default_version()`, and the SKILL.toml branch in `load_skills_from_directory`. Skills that
   only have SKILL.toml get an error log with migration instructions pointing to SKILL.md YAML
   frontmatter. Remove SKILL.toml-related tests. Update `repair_lockfile` and `init_skills_dir`
   README. (~100 lines deleted.)

4. **Agent Skills name validation** — Validate skill names: `[a-z0-9-]`, 1–64 chars, no
   leading/trailing hyphens, no consecutive hyphens, must match directory name. Enforce on
   install (reject invalid) and on load (warn and skip invalid).

5. **Prompt injection scanning** — Scoring-based scanner in a new `skills/scanner.rs` module.
   Checks SKILL.md content for: system prompt overrides (`ignore previous`, `forget
   instructions`), role manipulation (`you are now`, `act as`), trust escalation (`this skill is
   official`, `bypass trust`), encoded payloads (base64 blocks > threshold length), excessive
   tool declarations, and zero-width / homoglyph Unicode characters. Accumulates a risk score;
   configurable threshold. Run on install (block if score > threshold) and on load (warn +
   downgrade trust if score > threshold). New config: `skills.scan_threshold` (default TBD via
   tuning).

6. **Deferred Phase 2 tests** — Implement the 4 missing test tasks adapted for current code:
   - 4.2: Unit tests for `resolve_index()` — cache hit, cache miss, fetch failure, embedded
     fallback.
   - 4.6: Unit tests for `repair_lockfile()` — added, removed, updated, unchanged scenarios.
   - 4.7: Integration test for `handle_catalog_install()` end-to-end flow.
   - 4.8: Adapted from "test SKILL.toml deprecation warning" to "test SKILL.toml rejection with
     migration error message."

**P2 — Medium Value:**

7. **Tool sandboxing for third-party skills** — Third-party shell tools execute with
   `cwd` set to the skill directory. Path arguments validated to prevent traversal outside
   `skill_dir` (reject `../`, absolute paths outside scope, symlinks escaping scope). New field
   on `SkillTool`: `sandboxed: bool` (default `true` for ThirdParty, `false` for
   Official/Local). New module: `skills/sandbox.rs`. OS-level sandboxing (seccomp, landlock)
   deferred.

### Out of Scope

- OS-level process sandboxing (seccomp, landlock, sandbox-exec)
- Network restrictions for skill tools (allowlist/blocklist)
- Full `agentskills.io` compatibility field parsing (`license`, `compatibility`, `metadata`)
- Skills migration CLI tool (`corvus skills migrate`)
- Skill signing/verification with cryptographic keys
- Moving `version`/`author`/`tags` under `metadata` to match agentskills.io (deferred to spec
  alignment phase)

## Approach

### Task Group A: P0 — Security Critical (do first)

**A1. Content integrity verification**

In `load_skills_with_config()`, after enriching each skill from the lockfile:

```
1. Read SKILL.md bytes
2. Compute SHA-256 → current_hash
3. Compare current_hash against lockfile entry.content_hash
4. On match → proceed normally
5. On mismatch:
   - If ThirdParty → log::warn!, set skill.trust = ThirdParty,
     clear allowed_tools (instruction-only)
   - If Official or Local → log::warn! only (user likely edited)
6. If config.skills.verify_integrity == false → skip steps 2-5
```

The `compute_content_hash()` function already exists in `lockfile.rs` — reuse it.

**A2. Open-skills removal**

Pure deletion. Remove in order:
1. Constants and env var names from `mod.rs`
2. All `open_skills_*` functions from `mod.rs` (~160 lines)
3. `load_open_skill_md()` from `mod.rs`
4. `legacy_open_skills` field from `SkillsConfig` in `config/schema.rs`
5. The `ensure_open_skills_repo` call in `load_skills_with_config()`
6. Check if `directories` crate is still used elsewhere; remove from `Cargo.toml` if not

### Task Group B: P1 — High Value (do second)

**B1. SKILL.toml removal**

1. Delete `SkillManifest`, `SkillMeta` structs and `load_skill_toml()`, `default_version()`
2. In `load_skills_from_directory`: remove the SKILL.toml-first branch; if only SKILL.toml
   exists, log error with migration instructions and skip
3. Update `repair_lockfile` to only scan for SKILL.md
4. Update `init_skills_dir` README template
5. Delete SKILL.toml tests; add test for rejection error message

**B2. Name validation**

New module: `skills/validation.rs`

```rust
pub fn validate_skill_name(name: &str) -> Result<(), SkillValidationError> {
    // 1. Length: 1..=64
    // 2. Charset: [a-z0-9-]
    // 3. No leading/trailing hyphen
    // 4. No consecutive hyphens
    Ok(())
}

pub fn validate_name_matches_directory(name: &str, dir_name: &str)
    -> Result<(), SkillValidationError>;
```

Call `validate_skill_name` + `validate_name_matches_directory`:
- In `handle_install` → reject on failure (before writing lockfile)
- In `load_skills_from_directory` → warn and skip on failure

**B3. Prompt injection scanner**

New module: `skills/scanner.rs`

```rust
pub struct ScanResult {
    pub score: u32,
    pub findings: Vec<ScanFinding>,
}

pub struct ScanFinding {
    pub category: FindingCategory,
    pub pattern: String,
    pub line: usize,
    pub severity: u32,  // points added to score
}

pub enum FindingCategory {
    SystemPromptOverride,
    RoleManipulation,
    TrustEscalation,
    DataExfiltration,
    EncodedPayload,
    UnicodeAnomaly,
}

pub fn scan_skill_content(content: &str) -> ScanResult;
```

Integration points:
- `handle_install` → call `scan_skill_content`; if `score > threshold`, block install with
  findings report
- `load_skills_from_directory` → call `scan_skill_content`; if `score > threshold`, warn and
  downgrade trust to instruction-only

Patterns compiled as `lazy_static` or `once_cell::Lazy` regexes for performance.

**B4. Deferred tests**

Write tests using the existing test infrastructure. Task 4.8 transforms: instead of testing
deprecation warning, test that SKILL.toml-only directories produce an error log and are skipped.

### Task Group C: P2 — Medium Value (do last)

**C1. Tool sandboxing**

New module: `skills/sandbox.rs`

```rust
pub struct SandboxPolicy {
    pub enabled: bool,
    pub allowed_paths: Vec<PathBuf>,  // skill_dir + designated temp
}

pub fn validate_tool_paths(
    args: &[String],
    policy: &SandboxPolicy,
) -> Result<(), SandboxViolation>;

pub fn apply_sandbox(
    command: &mut std::process::Command,
    policy: &SandboxPolicy,
);
```

In the tool executor for shell-type tools:
1. If `skill.trust == ThirdParty`, construct `SandboxPolicy` with `skill_dir` as the only
   allowed path
2. Call `apply_sandbox` to set `cwd` to skill dir
3. Call `validate_tool_paths` on command arguments before execution
4. On violation → reject execution with error

Path validation checks:
- Canonicalize paths and verify they start with an allowed prefix
- Reject arguments containing `../` before canonicalization (defense in depth)
- Follow symlinks and verify resolved target is within scope

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/skills/mod.rs` | Modified | Remove open-skills (~200 lines), SKILL.toml (~100 lines), add integrity check and scanner/validation calls in load path |
| `clients/agent-runtime/src/skills/lockfile.rs` | Modified | Remove SKILL.toml references in repair, reuse `compute_content_hash` |
| `clients/agent-runtime/src/skills/frontmatter.rs` | Modified | Minor — validation calls after parse |
| `clients/agent-runtime/src/skills/trust.rs` | Modified | Add sandbox policy metadata |
| `clients/agent-runtime/src/skills/validation.rs` | New | Name validation functions |
| `clients/agent-runtime/src/skills/scanner.rs` | New | Prompt injection scoring scanner |
| `clients/agent-runtime/src/skills/sandbox.rs` | New | Tool sandboxing policy and path validation |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Remove `legacy_open_skills`, add `verify_integrity` and `scan_threshold` |
| `clients/agent-runtime/src/skillforge/mod.rs` | Modified | Minor — remove SKILL.toml assertion cleanup |
| `clients/agent-runtime/src/skillforge/integrate.rs` | Modified | Minor — remove SKILL.toml test assertions |
| `openspec/specs/skills-trust/spec.md` | Modified | Update R2 (open-skills removed), R10 (SKILL.toml removed), add R14-R18 for new requirements |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| SKILL.toml removal breaks users who haven't migrated | Low | Two phases of deprecation warnings shipped; clear migration error message with instructions |
| Open-skills removal breaks opt-in users | Very Low | Default OFF since Phase 1; opt-in since Phase 2; minimal user base |
| Hash verification false positives on manual edits | Low | Official/Local skills only warn (no downgrade); `corvus skills lock repair` re-hashes; configurable via `verify_integrity: false` |
| Prompt injection scanner false positives | Medium | Scoring-based approach (not binary); tunable threshold; load-time action is warn+downgrade, not block; install-time shows findings for user review |
| Sandbox bypass via symlinks or indirect execution | Medium | Defense-in-depth layer, not a hard boundary; canonicalize + resolve symlinks; document known limitations; OS-level sandboxing deferred to future phase |
| Large diff across multiple modules | Low | Grouped by priority tier; P0 is pure deletion + simple addition; reviewable incrementally |

## Rollback Plan

1. **Content integrity**: Set `skills.verify_integrity = false` in config to disable without code
   changes. For code rollback: revert the hash check in `load_skills_with_config()` — isolated
   to one function.

2. **Open-skills removal**: Revert the deletion commits. Since open-skills was default OFF, no
   runtime impact from re-adding dead code. Users who depended on it can re-enable.

3. **SKILL.toml removal**: Revert deletion commits to restore `load_skill_toml()`. Since the
   function was already deprecated, restoring it returns to Phase 2 behavior.

4. **Name validation**: Remove validation calls from install and load paths. Single-point
   integration makes revert trivial.

5. **Prompt injection scanner**: Set threshold to `u32::MAX` to effectively disable, or remove
   `scan_skill_content()` calls from install/load paths. Scanner module can remain as dead code.

6. **Tool sandboxing**: Remove `apply_sandbox` / `validate_tool_paths` calls from the tool
   executor. `sandboxed` field defaults to `false` if removed.

All changes are backward-compatible in the sense that reverting any individual item returns to
Phase 2 behavior without data loss.

## Dependencies

- Phases 1 and 2 fully shipped (confirmed)
- `sha2` crate already in `Cargo.toml` (used by `compute_content_hash`)
- `regex` crate already in `Cargo.toml` (needed for scanner patterns)
- No new external dependencies required

## Success Criteria

- [ ] All `open_skills_*` code removed; `CORVUS_OPEN_SKILLS_ENABLED` / `CORVUS_OPEN_SKILLS` env
      vars no longer recognized; `legacy_open_skills` config field removed
- [ ] All SKILL.toml loading code removed; SKILL.toml-only directories produce error log with
      migration instructions and are skipped
- [ ] `load_skills_with_config()` re-hashes SKILL.md and detects tampering; ThirdParty mismatch
      downgrades to instruction-only; Official/Local mismatch warns only
- [ ] Skill names validated against `[a-z0-9-]` 1–64 char format on install (reject) and load
      (warn+skip)
- [ ] `scan_skill_content()` detects at least: system prompt overrides, role manipulation, trust
      escalation, encoded payloads, Unicode anomalies
- [ ] Scanner blocks install above threshold; warns and downgrades trust on load above threshold
- [ ] Third-party shell tools execute with `cwd` = skill directory; path traversal outside skill
      dir rejected
- [ ] Deferred tests 4.2, 4.6, 4.7, 4.8 (adapted) all pass
- [ ] `make test` passes with no regressions
- [ ] `make lint-kotlin && cargo clippy` clean (no new warnings)
- [ ] Net lines of code reduced (deletions from open-skills + SKILL.toml > additions)

## Follow-Up (Phase 4 — Future)

- **OS-level sandboxing**: seccomp (Linux), sandbox-exec (macOS) for third-party tool processes
- **Network restrictions**: Allowlist/blocklist for HTTP tool destinations
- **Cryptographic skill signing**: Author signs SKILL.md; verify signature on install/load
- **agentskills.io field alignment**: Move `version`/`author`/`tags` under `metadata`; parse
  `license`, `compatibility` fields
- **Skills migration CLI**: `corvus skills migrate` to auto-convert SKILL.toml → SKILL.md
- **Scanner pattern updates**: Community-driven pattern database for prompt injection detection
