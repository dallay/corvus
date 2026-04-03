# Proposal: Trusted Skills Distribution — Phase 1

## Intent

Corvus's skills system has no trust model. The `besoeasy/open-skills` third-party repository is
auto-loaded by default into every agent's system prompt — unreviewed, unversioned, and with no
integrity checks. There is no distinction between official, local, and third-party skills. Any
commit to that external repo silently becomes part of every Corvus user's agent prompt, creating a
direct prompt-injection and arbitrary-code-execution attack surface.

This change introduces the security-critical foundation for trusted skills distribution: a trust
tier model derived from skill origin, deprecation of the unsafe open-skills default, a lockfile for
install integrity, trust-aware prompt rendering, and `allowed-tools` gating for third-party skills.

This is **Phase 1** of a three-phase plan. It focuses exclusively on closing the immediate security
gaps without requiring external infrastructure (no official skills repo, no registry API).

See [exploration.md](./exploration.md) for the full investigation.

## Scope

### In Scope

1. **Trust enum and origin tracking** — Add `SkillTrust` enum (`Official`, `Local`, `ThirdParty`)
   and `SkillOrigin` struct (source, installed timestamp, pinned ref, content hash) to the `Skill`
   type. Trust tier is **derived from origin at load time**, never stored as an independent mutable
   field. This prevents privilege escalation — a git-cloned skill cannot claim `Official` status.

2. **Open-skills deprecation** — Default `open_skills_enabled()` to `false`. When a user enables it
   via environment variable, emit a deprecation warning at startup. Add `skills.legacy_open_skills`
   configuration option as an explicit opt-in escape hatch.

3. **Skills lockfile** — Write `~/.corvus/workspace/skills.lock` (TOML) on install and update. Each
   entry records: trust tier, source, pinned git ref, SHA-256 content hash of `SKILL.md`, and
   install timestamp. The lockfile is the source of truth for installed skill metadata.

4. **Trust-aware prompt rendering** — Add a `trust` attribute to each `<skill>` element in the
   rendered `<available_skills>` XML block. Render skills in trust-priority order: official first,
   local second, third-party last. Append a caution note to third-party skill entries. When any
   third-party skills are present, add a preamble noting that they have not been reviewed by Corvus
   maintainers.

5. **`allowed-tools` parsing** — Parse the `allowed-tools` field from SKILL.md YAML frontmatter
   (per the Agent Skills standard). For third-party skills, only tools declared in `allowed-tools`
   are exposed to the agent. For official and local skills, all tools are allowed regardless of
   declaration. Third-party skills without an `allowed-tools` declaration become instruction-only
   (no tools exposed).

6. **Install flow improvements** — Resolve trust tier at install time based on source. For
   third-party skills that declare tools: require `--trust` CLI flag or interactive TTY
   confirmation. Write lock entry on successful install. Validate skill structure (SKILL.md exists,
   frontmatter is valid, directory name matches skill name).

### Out of Scope

- Official skills repository creation (`corvus-dev/corvus-skills`) — Phase 2
- Embedded skill index in binary at build time — Phase 2
- `SKILL.toml` format deprecation — Phase 2
- SkillForge trust boundary enforcement — Phase 2
- Full Agent Skills standard validation (name constraints, `compatibility`, `metadata`) — Phase 3
- Third-party tool sandboxing (filesystem scoping, network restrictions) — Phase 3
- Content integrity verification beyond hash-on-install comparison — Phase 3
- Automated content scanning for prompt injection patterns — Future

## Approach

### Trust Model

Trust is a **derived property**, not a stored one. The `SkillTrust` enum is computed from
`SkillOrigin` at load time:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTrust {
    Official,    // From official Corvus skills repo (Phase 2 — no skills qualify yet)
    Local,       // Created by user in workspace, or symlinked from local path
    ThirdParty,  // Installed from any external git source
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOrigin {
    pub source: SkillSource,
    pub installed_at: Option<String>,  // ISO 8601
    pub pinned_ref: Option<String>,    // Git commit SHA or tag
    pub content_hash: Option<String>,  // SHA-256 of SKILL.md content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    Official { repo: String, path: String },
    Local,
    LinkedLocal { target: PathBuf },
    GitRepo { url: String },
    Discovered { source: String, repo: String },
}
```

`Skill` struct gains two new fields: `trust: SkillTrust` and `origin: SkillOrigin`. These are
populated during `load_skills()` based on how the skill was loaded (open-skills path, workspace
path with lockfile data, or local creation).

### Open-Skills Deprecation

In `skills/mod.rs`, change `open_skills_enabled()` to return `false` by default (currently returns
`true` unless `cfg!(test)`). The function checks, in order:

1. Config file `skills.legacy_open_skills` — explicit boolean
2. Environment variable `CORVUS_OPEN_SKILLS` — `"true"` / `"false"`
3. Default: `false`

When open-skills is enabled by either mechanism, log a deprecation warning:
`"open-skills is deprecated and will be removed in a future release. Install individual skills with 'corvus skills install <url>' instead."`

Skills loaded from the open-skills path are tagged as `ThirdParty` trust with
`SkillSource::GitRepo { url: "https://github.com/besoeasy/open-skills" }`.

### Skills Lockfile

Location: `~/.corvus/workspace/skills.lock`

Format (TOML):

```toml
[skills.git-expert]
trust = "official"
source = "corvus-dev/corvus-skills"
path = "skills/git-expert"
ref = "abc123def456"
content_hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
installed_at = "2026-03-24T10:00:00Z"

[skills.my-custom-skill]
trust = "local"
source = "local"

[skills.some-community-skill]
trust = "third-party"
source = "https://github.com/user/skill-repo"
ref = "def456abc789"
content_hash = "sha256:a1b2c3d4..."
installed_at = "2026-03-20T15:30:00Z"
allowed_tools = ["Read", "Grep", "Glob"]
```

The lockfile is written on `skills install` and `skills update`. On `load_skills()`, lockfile
entries are read to populate `SkillOrigin` for workspace skills. Skills present on disk but absent
from the lockfile are treated as `Local` (backward compatibility for pre-lockfile installs).

### Trust-Aware Prompt Rendering

Modify `render_skills_section` in `prompt.rs` to:

1. Sort skills by trust tier: `Official` → `Local` → `ThirdParty`
2. Add `trust` attribute to each `<skill>` XML element
3. For `ThirdParty` skills, append a `<note>` child element
4. When any third-party skills are present, prepend a preamble

```xml
<available_skills>
  <skill trust="official">
    <name>git-expert</name>
    <description>Git operations expert</description>
    <location>file:///...</location>
  </skill>
  <skill trust="local">
    <name>my-workflow</name>
    <description>Custom workflow</description>
    <location>file:///...</location>
  </skill>
  <skill trust="third-party">
    <name>community-tool</name>
    <description>Community contributed</description>
    <location>file:///...</location>
    <note>This skill is from a third-party source. Its instructions have not been
    reviewed by Corvus maintainers. Exercise caution.</note>
  </skill>
</available_skills>
```

### `allowed-tools` Parsing

Parse the `allowed-tools` field from SKILL.md YAML frontmatter. The field follows the Agent Skills
standard format — a list of tool names the skill is pre-approved to use.

```yaml
---
name: example-skill
description: An example
allowed-tools:
  - Read
  - Grep
  - Glob
---
```

Enforcement rules:

| Trust Tier | `allowed-tools` declared | Behavior                              |
|------------|--------------------------|---------------------------------------|
| Official   | Any                      | All tools allowed                     |
| Local      | Any                      | All tools allowed                     |
| ThirdParty | Yes                      | Only declared tools exposed to agent  |
| ThirdParty | No                       | Instruction-only (no tools available) |

At runtime, when building the tool list for a skill activation, filter `SkillTool` entries against
the `allowed-tools` list for third-party skills.

### Install Flow

Updated `skills install <source>` flow:

1. **Resolve source** — parse URL/path, determine `SkillSource` variant
2. **Determine trust tier** — derive `SkillTrust` from `SkillSource`
3. **Fetch metadata** — for git sources, shallow-clone and read SKILL.md frontmatter
4. **Validate structure** — SKILL.md exists, frontmatter parses, name matches directory
5. **Trust gate** — if `ThirdParty` and `allowed-tools` declares tools:
    - If `--trust` flag provided: proceed
    - If TTY available: prompt user with tool list and ask for confirmation
    - If neither: abort with message explaining `--trust` flag
6. **Install** — clone/copy to `~/.corvus/workspace/skills/<name>/`
7. **Compute integrity** — SHA-256 hash of SKILL.md content
8. **Write lock entry** — append/update entry in `skills.lock`

New CLI flags on `skills install`:

- `--trust` — acknowledge third-party trust for skills with tools

Expand `SkillCommands` enum in `lib.rs` to support the `--trust` flag on install.

## Affected Areas

| Area                                        | Impact   | Description                                                                                                                                                                                                                           |
|---------------------------------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/skills/mod.rs`   | Modified | Add `SkillTrust`, `SkillOrigin`, `SkillSource` types. Modify `Skill` struct. Update `load_skills()` to populate trust/origin. Add lockfile read/write. Update install flow with trust gating. Change `open_skills_enabled()` default. |
| `clients/agent-runtime/src/agent/prompt.rs` | Modified | Update `render_skills_section` for trust-aware rendering: sort by tier, add `trust` attribute, add third-party caution note.                                                                                                          |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Pass skills trust configuration to prompt builder if needed.                                                                                                                                                                          |
| `clients/agent-runtime/src/main.rs`         | Modified | Add `--trust` flag to `skills install` CLI command.                                                                                                                                                                                   |
| `clients/agent-runtime/src/lib.rs`          | Modified | Expand `SkillCommands` enum with `--trust` option on install.                                                                                                                                                                         |
| `clients/agent-runtime/src/config/`         | Modified | Add `skills.legacy_open_skills` configuration option.                                                                                                                                                                                 |

## Risks

| Risk                                                          | Likelihood | Mitigation                                                                                                                                                  |
|---------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Breaking existing users who rely on open-skills content       | Medium     | Soft deprecation: default off with config escape hatch (`skills.legacy_open_skills`), deprecation warning, documentation. Hard removal deferred to Phase 2. |
| Lockfile corruption or desync with actual installed skills    | Low        | Treat lockfile as advisory — skills present on disk without lock entries default to `Local`. Add `skills lock repair` in Phase 2 if needed.                 |
| Prompt token increase from trust attributes and caution notes | Low        | Additions are minimal: one XML attribute per skill, one-line note only for third-party. Measured impact expected < 100 tokens.                              |
| `allowed-tools` parsing fragility with varied YAML formats    | Low        | Follow Agent Skills standard exactly. Treat parse failures as "no tools declared" (safe default).                                                           |
| Users frustrated by `--trust` gate on third-party install     | Low        | Gate only triggers when third-party skills declare tools. Instruction-only skills install without friction. Clear error message explains the flag.          |
| Phase 2 delays leave no official skills catalog               | Medium     | Phase 1 is self-contained and valuable without an official repo. Local and third-party workflows work independently.                                        |

## Rollback Plan

All changes are **additive** to the `Skill` struct — existing serialization is unaffected.

The only **breaking change** is `open_skills_enabled()` defaulting to `false`. Rollback:

1. Revert `open_skills_enabled()` to return `true` by default
2. Remove trust-tier filtering from `render_skills_section` (revert to flat rendering)
3. The `SkillTrust`, `SkillOrigin`, and lockfile code can remain inert — they add no overhead if
   not consumed

The `skills.legacy_open_skills` config option serves as an immediate user-level rollback without
requiring a code change.

## Dependencies

- None. Phase 1 is self-contained — no external repos, registries, or infrastructure required.
- Rust standard library provides SHA-256 via existing `sha2` crate (already in dependency tree) or
  equivalent.
- TOML serialization for lockfile via existing `toml` crate dependency.

## Success Criteria

- [ ] `SkillTrust` enum and `SkillOrigin` struct are added to the `Skill` type, populated at load
  time, and never independently mutable
- [ ] `open_skills_enabled()` returns `false` by default; enabling it emits a deprecation warning
- [ ] `skills.legacy_open_skills` config option exists and works as an opt-in override
- [ ] `skills.lock` is written on `skills install` with trust tier, source, pinned ref, content
  hash, and install timestamp
- [ ] `<skill>` XML elements in prompt output include a `trust` attribute
- [ ] Skills are rendered in trust-priority order: official → local → third-party
- [ ] Third-party skills include a caution note in rendered prompt
- [ ] `allowed-tools` is parsed from SKILL.md YAML frontmatter
- [ ] Third-party skills without `allowed-tools` are instruction-only (no tools exposed)
- [ ] Third-party skills with `allowed-tools` only expose declared tools
- [ ] `skills install` for third-party skills with tools requires `--trust` flag or interactive
  confirmation
- [ ] All existing tests pass (no regression from trust model additions)
- [ ] New unit tests cover: trust derivation from origin, lockfile serialization/deserialization,
  `allowed-tools` parsing, prompt rendering with trust tiers

## Follow-Up Changes

### Phase 2: Official Skills Catalog

- Create `corvus-dev/corvus-skills` repository with curated skills and `index.toml`
- Embed skill index (names, descriptions, hashes) in binary at build time for zero-network cold
  start
- Improve `corvus skills install` with catalog search and version pinning
- Deprecate `SKILL.toml` format in favor of `SKILL.md` with YAML frontmatter
- Add SkillForge trust boundaries (discovered skills always `ThirdParty`, require explicit opt-in)
- Add `skills lock repair` command for lockfile maintenance

### Phase 3: Full Standard Compliance and Hardening

- Full Agent Skills standard validation (name constraints, `compatibility`, `metadata` fields)
- Convert SkillForge to explicit discovery workflow (`corvus skills discover`)
- Content integrity verification on every load (compare hash against lockfile)
- Third-party tool sandboxing (filesystem scoping to skill directory, network restrictions)
- Automated content scanning for prompt injection patterns

## References

- [Exploration document](./exploration.md)
- [Agent Skills Standard — Specification](https://agentskills.io/specification)
- [Agent Runtime CLI contract](../../../specs/client-surfaces/surface-contracts/agent-runtime-cli.md)
- Current implementation: `clients/agent-runtime/src/skills/mod.rs`,
  `clients/agent-runtime/src/agent/prompt.rs`
