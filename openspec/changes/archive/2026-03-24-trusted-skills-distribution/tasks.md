# Tasks: Trusted Skills Distribution — Phase 1

## Phase 1: Infrastructure (New Types and Modules)

Foundational types and modules that all later phases depend on. These are leaf modules with no
internal dependencies beyond the standard library and existing crates.

- [x] 1.1 Create `src/skills/trust.rs` — SkillTrust enum, SkillSource enum, SkillOrigin struct, trust derivation

  **Description**: Create the `trust.rs` submodule with the core trust types: `SkillTrust` enum
  (`Official`, `Local`, `ThirdParty`) with `Ord` ordering, `SkillSource` enum (5 variants),
  `SkillOrigin` struct with `Default` impl, and `impl From<&SkillSource> for SkillTrust` for
  trust derivation. Include `as_str()` method on `SkillTrust` for lockfile/prompt serialization.

  **Files**: Create `clients/agent-runtime/src/skills/trust.rs`; modify `clients/agent-runtime/src/skills/mod.rs` to add `pub mod trust;`

  **Dependencies**: None (leaf module)

  **Acceptance criteria**:
  - `SkillTrust` derives `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Serialize`, `Deserialize` (R1.1)
  - `From<&SkillSource>` maps all 5 variants correctly per R1.2 derivation table
  - `SkillOrigin` contains `source`, `installed_at`, `pinned_ref`, `content_hash` fields (R1.3)
  - `SkillSource` has all 5 variants with correct field types (R1.4)
  - `Default` for `SkillOrigin` returns `Local` source with all `None` optional fields
  - Covers spec scenarios: trust from git-cloned skill, workspace skill, symlinked skill, privilege escalation prevention

  **Complexity**: S

- [x] 1.2 Create `src/skills/frontmatter.rs` — YAML frontmatter parser for SKILL.md

  **Description**: Create the `frontmatter.rs` submodule with a minimal hand-rolled YAML
  frontmatter parser (no `serde_yaml` dependency per AD4). Implement `SkillFrontmatter` struct
  with `name`, `description`, and `allowed_tools` fields. Parse `---` delimited blocks, extract
  key-value pairs and `allowed-tools` list items. Return `Default` on parse failure (safe default).

  **Files**: Create `clients/agent-runtime/src/skills/frontmatter.rs`; modify `clients/agent-runtime/src/skills/mod.rs` to add `pub mod frontmatter;`

  **Dependencies**: None (leaf module)

  **Acceptance criteria**:
  - Parses valid frontmatter with `name`, `description`, `allowed-tools` list (R5.1)
  - Absent `allowed-tools` returns empty vec (treated as `None` semantically) (R5.1)
  - Missing `---` delimiters returns `SkillFrontmatter::default()` (R5.3)
  - Malformed YAML content returns default without panic (R5.3)
  - Covers spec scenarios: ThirdParty with declared allowed-tools, without allowed-tools, malformed allowed-tools

  **Complexity**: S

- [x] 1.3 Create `src/skills/lockfile.rs` — Lockfile struct, TOML serialization, read/write, content hashing

  **Description**: Create the `lockfile.rs` submodule with `SkillsLockfile` and `LockEntry`
  structs, `read_lockfile()` (returns empty on missing/corrupt), `write_lock_entry()`,
  `remove_lock_entry()`, `build_lock_entry()`, `lock_entry_to_origin()`, and
  `compute_content_hash()` (SHA-256 via `sha2` crate). Uses `BTreeMap` for deterministic
  ordering. Lockfile is advisory per AD2.

  **Files**: Create `clients/agent-runtime/src/skills/lockfile.rs`; modify `clients/agent-runtime/src/skills/mod.rs` to add `pub mod lockfile;`

  **Dependencies**: 1.1 (imports `SkillTrust`, `SkillOrigin`, `SkillSource` from `trust.rs`)

  **Acceptance criteria**:
  - Lockfile location is `{workspace}/skills.lock` in TOML format (R3.1)
  - `LockEntry` has all required fields: `trust`, `source` and optional `path`, `ref`, `content_hash`, `installed_at`, `allowed_tools` (R3.2)
  - `read_lockfile()` returns empty default on missing file (R3.4)
  - `read_lockfile()` returns empty default and logs warning on corrupt TOML (R3.5)
  - `write_lock_entry()` creates/updates entries correctly (R3.3)
  - `remove_lock_entry()` removes correct entry while preserving others
  - `compute_content_hash()` returns `"sha256:<64-char-hex>"` format (R6.5)
  - `lock_entry_to_origin()` correctly converts `LockEntry` back to `SkillOrigin`
  - Covers spec scenarios: lockfile written on install, skill on disk without entry, corrupt lockfile, pinned ref

  **Complexity**: M

- [x] 1.4 Add `SkillsConfig` to config schema — `skills.legacy_open_skills` option

  **Description**: Add a `SkillsConfig` struct with `legacy_open_skills: bool` (default `false`)
  to `src/config/schema.rs`. Add `#[serde(default)] pub skills: SkillsConfig` field to the
  top-level `Config` struct. Re-export from `src/config/mod.rs` if needed.

  **Files**: Modify `clients/agent-runtime/src/config/schema.rs`; possibly modify `clients/agent-runtime/src/config/mod.rs`

  **Dependencies**: None

  **Acceptance criteria**:
  - `SkillsConfig` struct exists with `legacy_open_skills: bool` field defaulting to `false`
  - `Config` struct includes `#[serde(default)] pub skills: SkillsConfig`
  - Existing config deserialization is unaffected (field is optional with default)
  - Supports R2.2 config file mechanism

  **Complexity**: S

## Phase 2: Core Implementation (Modify Existing Code)

Integrate the new infrastructure into existing modules. These tasks modify existing files and
depend on Phase 1 types being available.

- [x] 2.1 Update `Skill` struct with `trust`, `origin`, and `allowed_tools` fields

  **Description**: Add three new `#[serde(skip)]` fields to the `Skill` struct: `trust: SkillTrust`
  (default `Local`), `origin: SkillOrigin` (default), and `allowed_tools: Vec<String>` (default
  empty). These use `serde(skip)` so existing TOML deserialization is unaffected. Update any
  `Skill` construction sites to include defaults or explicit values.

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 1.1 (uses `SkillTrust`, `SkillOrigin` types)

  **Acceptance criteria**:
  - `Skill` struct has `trust: SkillTrust`, `origin: SkillOrigin`, `allowed_tools: Vec<String>` fields (R1.3)
  - All fields are `#[serde(skip)]` — no serialization impact
  - Existing tests compile and pass without modification
  - All existing `Skill` construction sites compile (provide defaults)

  **Complexity**: S

- [x] 2.2 Update `open_skills_enabled()` — default to `false`, config integration, deprecation warning

  **Description**: Modify `open_skills_enabled()` to check in priority order: (1) config file
  `skills.legacy_open_skills`, (2) env var `CORVUS_OPEN_SKILLS` (note: current env var is
  `CORVUS_OPEN_SKILLS_ENABLED` — update to also check `CORVUS_OPEN_SKILLS`), (3) default `false`.
  When enabled, emit deprecation warning via `tracing::warn!`. The function signature may need to
  accept a `&Config` parameter or access config through existing patterns.

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 1.4 (uses `SkillsConfig.legacy_open_skills`)

  **Acceptance criteria**:
  - `open_skills_enabled()` returns `false` by default (R2.1)
  - Config file option takes precedence over env var (R2.2)
  - Deprecation warning emitted when enabled containing "open-skills is deprecated" text (R2.3)
  - Warning suggests `corvus skills install <url>` as replacement (R2.3)
  - Covers spec scenarios: disabled by default, enabled via env, config overrides env

  **Complexity**: S

- [x] 2.3 Update `load_skills()` to populate trust/origin from each loading path

  **Description**: Modify `load_skills()` to: (1) read lockfile via `read_lockfile()`, (2) for
  open-skills path: tag each skill with `ThirdParty` trust and `GitRepo` source pointing to
  `OPEN_SKILLS_REPO_URL` (R2.4), (3) for workspace skills: look up lock entry by name, populate
  `origin` from lock entry via `lock_entry_to_origin()`, derive `trust` from `origin.source`;
  if no lock entry, default to `Local` trust (R3.4), (4) parse frontmatter from SKILL.md files
  to extract `allowed_tools`.

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 1.1, 1.2, 1.3, 2.1, 2.2

  **Acceptance criteria**:
  - Open-skills are tagged `ThirdParty` with correct `GitRepo` source (R2.4)
  - Workspace skills with lock entries get trust/origin from lockfile (R1.2)
  - Workspace skills without lock entries default to `Local` (R3.4)
  - `allowed_tools` populated from frontmatter for SKILL.md-based skills
  - Covers spec scenarios: trust from git-cloned, workspace, symlinked skills; open-skills tagged as ThirdParty

  **Complexity**: M

- [x] 2.4 Update install flow with trust resolution, validation, and gating

  **Description**: Modify `handle_install_command()` to: (1) resolve `SkillSource` from URL/path,
  (2) derive `SkillTrust`, (3) validate structure (SKILL.md exists, frontmatter parses, name
  matches directory) (R6.3), (4) parse `allowed-tools` from frontmatter, (5) apply trust gate
  for ThirdParty skills with tools (check `--trust` flag → TTY prompt → abort) (R6.2), (6)
  compute SHA-256 content hash (R6.5), (7) write lock entry on success (R6.4).

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 1.1, 1.2, 1.3, 2.1

  **Acceptance criteria**:
  - Trust resolved from source before install proceeds (R6.1)
  - ThirdParty + tools requires `--trust` or TTY confirmation (R6.2)
  - ThirdParty without tools installs without gate (R6.2)
  - No TTY + no `--trust` aborts with clear message (R6.2)
  - SKILL.md must exist, frontmatter must parse, name must match directory (R6.3)
  - Lock entry written with trust, source, content_hash, installed_at (R6.4)
  - Content hash is `sha256:<hex>` of SKILL.md bytes (R6.5)
  - Covers spec scenarios: install with --trust, install without --trust (TTY), install without --trust (no TTY), instruction-only install, name mismatch, missing SKILL.md, content hash stored

  **Complexity**: L

- [x] 2.5 Update remove flow to clean lockfile entry

  **Description**: Modify `handle_remove_command()` to call `remove_lock_entry()` after
  successfully removing the skill directory. Failure to remove the lock entry should log a
  warning but not fail the remove operation (advisory lockfile model).

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 1.3

  **Acceptance criteria**:
  - `skills remove <name>` removes the `[skills.<name>]` entry from `skills.lock`
  - Lock entry removal failure does not block skill directory removal
  - Existing remove behavior is preserved

  **Complexity**: S

- [x] 2.6 Add `--trust` flag to `SkillCommands::Install` in CLI

  **Description**: Add `#[arg(long)] trust: bool` field to the `Install` variant of
  `SkillCommands` in `src/lib.rs`. Update `handle_command()` in `src/skills/mod.rs` to pass the
  flag through to `handle_install_command()`. Update the match arm in `main.rs` if needed.

  **Files**: Modify `clients/agent-runtime/src/lib.rs`; modify `clients/agent-runtime/src/skills/mod.rs` (handle_command signature); possibly modify `clients/agent-runtime/src/main.rs`

  **Dependencies**: None (can be done in parallel with other Phase 2 tasks)

  **Acceptance criteria**:
  - `corvus skills install <source> --trust` is accepted by CLI parser
  - `--trust` flag value is passed to install handler
  - Existing `corvus skills install <source>` (without flag) continues to work
  - CLI help text describes the flag purpose

  **Complexity**: S

## Phase 3: Prompt Integration (Trust-Aware Rendering)

Update prompt rendering to surface trust information to the agent.

- [x] 3.1 Update `render_skills_section` — sort by trust, add trust attribute

  **Description**: Modify `render_skills_section()` in `src/agent/prompt.rs` to: (1) sort skills
  by trust tier (`Official` → `Local` → `ThirdParty`, alphabetical within tier), (2) add
  `trust="<tier>"` attribute to each `<skill>` XML element. Uses `SkillTrust::Ord` for sorting.

  **Files**: Modify `clients/agent-runtime/src/agent/prompt.rs`

  **Dependencies**: 2.1 (Skill struct has `trust` field)

  **Acceptance criteria**:
  - Skills rendered in order: Official first, Local second, ThirdParty last (R4.2)
  - Within each tier, skills sorted alphabetically by name (R4.2)
  - Each `<skill>` element has `trust="official"`, `trust="local"`, or `trust="third-party"` attribute (R4.1)
  - Covers spec scenario: mixed trust tiers rendered in correct order

  **Complexity**: S

- [x] 3.2 Add third-party caution note and preamble

  **Description**: Extend the updated `render_skills_section()` to: (1) append a `<note>` child
  element to each ThirdParty skill with caution text about unreviewed instructions, (2) when any
  ThirdParty skills are present, prepend a preamble before `<available_skills>` noting that some
  skills are from third-party sources. When no ThirdParty skills exist, omit the preamble.

  **Files**: Modify `clients/agent-runtime/src/agent/prompt.rs`

  **Dependencies**: 3.1

  **Acceptance criteria**:
  - ThirdParty skills include `<note>` with "third-party source" and "not been reviewed" text (R4.3)
  - Preamble included when any ThirdParty skill present (R4.4)
  - Preamble omitted when only Official/Local skills (R4.4)
  - Covers spec scenarios: ThirdParty caution note, preamble present, no preamble

  **Complexity**: S

- [x] 3.3 Filter tools based on `allowed-tools` for third-party skills

  **Description**: At the point where tool lists are built for skill activation (in
  `src/skills/mod.rs` or `src/channels/mod.rs` where skills are wired into the agent), filter
  `SkillTool` entries against the `allowed_tools` list for ThirdParty skills. Official and Local
  skills bypass the filter. ThirdParty skills with empty `allowed_tools` expose no tools.

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs` (or wherever tool filtering is applied)

  **Dependencies**: 2.1, 2.3 (Skill has trust and allowed_tools populated)

  **Acceptance criteria**:
  - ThirdParty skills with `allowed_tools` only expose declared tools (R5.2)
  - ThirdParty skills without `allowed_tools` are instruction-only (R5.2)
  - Official skills ignore `allowed_tools` — all tools exposed (R5.2)
  - Local skills ignore `allowed_tools` — all tools exposed (R5.2)
  - Covers spec scenarios: ThirdParty with declared tools, ThirdParty without, Official ignores, Local ignores

  **Complexity**: M

## Phase 4: Testing

Comprehensive test coverage for all new functionality. Tests reference specific spec scenarios.

- [ ] 4.1 Unit tests for trust derivation (`src/skills/trust.rs`)

  **Description**: Add `#[cfg(test)] mod tests` in `trust.rs` with tests for: (1) `From<&SkillSource>`
  mapping for all 5 variants, (2) `SkillTrust::Ord` ordering (`Official < Local < ThirdParty`),
  (3) `as_str()` returns correct string representations, (4) `SkillOrigin::default()` returns
  `Local` source, (5) privilege escalation prevention (frontmatter `trust` field ignored).

  **Files**: Modify `clients/agent-runtime/src/skills/trust.rs`

  **Dependencies**: 1.1

  **Acceptance criteria**:
  - Tests cover all 5 `SkillSource` → `SkillTrust` mappings (R1.2 table)
  - Tests verify `Official < Local < ThirdParty` ordering for sort correctness
  - Tests verify `as_str()` for `"official"`, `"local"`, `"third-party"`
  - Covers spec scenarios: trust from git-cloned, workspace, symlinked, privilege escalation prevention
  - ~8-10 tests, all pass

  **Complexity**: S

- [ ] 4.2 Unit tests for frontmatter parsing (`src/skills/frontmatter.rs`)

  **Description**: Add `#[cfg(test)] mod tests` in `frontmatter.rs` with tests for: (1) valid
  frontmatter with all fields, (2) valid without `allowed-tools`, (3) no frontmatter delimiters,
  (4) malformed YAML content, (5) empty `allowed-tools: []`, (6) `allowed-tools` with quoted
  values.

  **Files**: Modify `clients/agent-runtime/src/skills/frontmatter.rs`

  **Dependencies**: 1.2

  **Acceptance criteria**:
  - Valid frontmatter parses name, description, and allowed-tools list correctly (R5.1)
  - Missing `allowed-tools` results in empty vec (R5.1)
  - Missing delimiters returns default (R5.3)
  - Malformed content returns default without panic (R5.3)
  - Covers spec scenarios: malformed allowed-tools defaults to no tools
  - ~5-6 tests, all pass

  **Complexity**: S

- [ ] 4.3 Unit tests for lockfile serialization (`src/skills/lockfile.rs`)

  **Description**: Add `#[cfg(test)] mod tests` in `lockfile.rs` with tests for: (1) serialization
  round-trip (write + read), (2) `read_lockfile()` with missing file, (3) `read_lockfile()` with
  corrupt TOML, (4) `write_lock_entry()` creates and updates entries, (5) `remove_lock_entry()`
  removes correct entry, (6) `compute_content_hash()` returns correct SHA-256 format, (7)
  `lock_entry_to_origin()` converts correctly, (8) `build_lock_entry()` populates all fields.

  **Files**: Modify `clients/agent-runtime/src/skills/lockfile.rs`

  **Dependencies**: 1.3

  **Acceptance criteria**:
  - Serialization round-trip preserves all fields (R3.1, R3.2)
  - Missing file returns empty default (R3.4)
  - Corrupt TOML returns empty default without panic (R3.5)
  - Write creates new entries and updates existing ones (R3.3)
  - Remove deletes correct entry, preserves others
  - Content hash matches expected SHA-256 for known input (R6.5)
  - Covers spec scenarios: lockfile written on install, corrupt lockfile, pinned ref, content hash
  - ~7-8 tests, all pass

  **Complexity**: S

- [ ] 4.4 Unit tests for prompt rendering with trust tiers (`src/agent/prompt.rs`)

  **Description**: Add tests in `prompt.rs` for: (1) mixed trust tiers rendered in correct sort
  order, (2) `trust` attribute present on each `<skill>` element, (3) ThirdParty skills include
  `<note>` caution text, (4) preamble present when ThirdParty skills exist, (5) no preamble when
  only Official/Local skills.

  **Files**: Modify `clients/agent-runtime/src/agent/prompt.rs`

  **Dependencies**: 3.1, 3.2

  **Acceptance criteria**:
  - Sort order verified: Official → Local → ThirdParty (R4.2)
  - `trust` attribute values verified for all tiers (R4.1)
  - Caution note text present for ThirdParty (R4.3)
  - Preamble conditional on ThirdParty presence (R4.4)
  - Covers spec scenarios: mixed tiers order, caution note, preamble present, no preamble
  - ~2-3 tests, all pass

  **Complexity**: S

- [ ] 4.5 Integration test for install flow with trust gating

  **Description**: Add integration-level tests in `skills/mod.rs` for the install flow: (1)
  ThirdParty skill with `--trust` flag installs and writes lock entry, (2) instruction-only
  ThirdParty skill installs without gate, (3) validation failure on missing SKILL.md aborts, (4)
  validation failure on name mismatch aborts, (5) lock entry contains correct trust, source,
  content_hash, installed_at after install.

  **Files**: Modify `clients/agent-runtime/src/skills/mod.rs`

  **Dependencies**: 2.4, 2.6

  **Acceptance criteria**:
  - Install with `--trust` proceeds for ThirdParty+tools (R6.2)
  - Instruction-only ThirdParty installs without gate (R6.2)
  - Missing SKILL.md aborts install (R6.3)
  - Name mismatch aborts install (R6.3)
  - Lock entry written with all required fields on success (R6.4)
  - Covers spec scenarios: install with --trust, instruction-only install, name mismatch, missing SKILL.md, content hash stored
  - ~4-5 tests, all pass

  **Complexity**: M

- [ ] 4.6 Verify existing test suite passes with no regression

  **Description**: Run the full `cargo test` suite for the agent-runtime crate and verify all
  existing tests pass. The new `#[serde(skip)]` fields with `Default` impls should not affect
  existing deserialization. Confirm no compilation warnings related to the new code.

  **Files**: No file changes — validation only

  **Dependencies**: All previous tasks (4.1–4.5)

  **Acceptance criteria**:
  - `cargo test` passes with 0 failures
  - `cargo clippy --all-targets -- -D warnings` passes
  - `cargo fmt --all -- --check` passes
  - No regression in existing skill loading, install, remove, or prompt rendering tests
  - Matches proposal success criterion: "All existing tests pass (no regression from trust model additions)"

  **Complexity**: S

## Dependency Graph

```
1.1 (trust.rs)  ──┐
                   ├── 1.3 (lockfile.rs) ──┐
1.2 (frontmatter) │                        │
                   ├── 2.1 (Skill struct) ──┤
1.4 (config)  ────┤                        ├── 2.3 (load_skills) ── 3.3 (tool filter)
                   ├── 2.2 (open_skills)    │
                   │                        ├── 2.4 (install flow)
2.6 (--trust CLI) ─┤                        │
                   │                        ├── 2.5 (remove flow)
                   │                        │
                   └── 3.1 (sort+attr) ── 3.2 (note+preamble)
                                            │
                       4.1─4.6 (testing) ───┘
```
