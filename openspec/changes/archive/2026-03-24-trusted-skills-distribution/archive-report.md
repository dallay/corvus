# Archive Report: Trusted Skills Distribution — Phase 1

**Change**: trusted-skills-distribution
**Archived**: 2026-03-24
**Archived by**: sdd-archive agent

---

## Phase Completion Summary

| Phase   | Status             | Notes                                                                           |
|---------|--------------------|---------------------------------------------------------------------------------|
| Explore | Done               | Full investigation of security gaps in open-skills model                        |
| Propose | Done               | Phase 1 scope defined: trust model, lockfile, prompt rendering, install gating  |
| Spec    | Done               | 6 requirements (R1–R6) with 30+ scenarios in `skills-trust/spec.md`             |
| Design  | Done               | 4 architecture decisions (AD1–AD4), file-level change plan                      |
| Tasks   | Done               | 19 tasks across 4 phases; 16/19 implemented, 3 genuinely incomplete (test gaps) |
| Apply   | Done               | 3 new modules + 5 modified files in `clients/agent-runtime/`                    |
| Verify  | PASS WITH WARNINGS | 3 critical issues fixed; 25 new tests, 5327 total passing                       |

---

## Delta Specs Synced

| Domain       | Action                 | Target                                |
|--------------|------------------------|---------------------------------------|
| skills-trust | **Created** (new spec) | `openspec/specs/skills-trust/spec.md` |

This is a new spec domain. The full spec (480 lines, 6 requirements R1–R6) was copied directly
from the delta to main specs. No merge was needed.

---

## Implementation Summary

### New Files Created

| File                                              | Purpose                                                                 |
|---------------------------------------------------|-------------------------------------------------------------------------|
| `clients/agent-runtime/src/skills/trust.rs`       | SkillTrust enum, SkillSource enum, SkillOrigin struct, trust derivation |
| `clients/agent-runtime/src/skills/frontmatter.rs` | Hand-rolled YAML frontmatter parser for SKILL.md (no serde_yaml dep)    |
| `clients/agent-runtime/src/skills/lockfile.rs`    | Skills lockfile (TOML), read/write/remove, SHA-256 content hashing      |

### Modified Files

| File                                         | Changes                                                                                                                                                                             |
|----------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/skills/mod.rs`    | Trust/origin/allowed_tools fields on Skill, open_skills_enabled(), load_skills() trust enrichment, install flow trust gating, remove flow lockfile cleanup, filter_tools_by_trust() |
| `clients/agent-runtime/src/agent/prompt.rs`  | Trust-aware rendering: sort by tier, trust attribute, caution note, conditional preamble                                                                                            |
| `clients/agent-runtime/src/config/schema.rs` | SkillsConfig struct with legacy_open_skills field                                                                                                                                   |
| `clients/agent-runtime/src/config/mod.rs`    | Re-export of SkillsConfig                                                                                                                                                           |
| `clients/agent-runtime/src/lib.rs`           | --trust CLI flag on SkillCommands::Install                                                                                                                                          |
| `clients/agent-runtime/src/main.rs`          | Pass --trust flag through to handler                                                                                                                                                |
| `clients/agent-runtime/src/channels/mod.rs`  | Tool filtering integration                                                                                                                                                          |

### Test Coverage

- **25 new unit tests**: trust.rs (9), frontmatter.rs (7), lockfile.rs (9)
- **5327 total tests passing**, 0 failures, 0 ignored
- **Gaps**: Prompt rendering tests (R4) and install flow integration tests (R6) not yet written

---

## Verification Result

**Verdict**: PASS WITH WARNINGS

### Critical Issues (Fixed)

1. **R2.2: Config mechanism unreachable** — `ensure_open_skills_repo()` was passing `None` to
   `open_skills_enabled()`, making `skills.legacy_open_skills` config path unreachable. Fixed by
   passing `&config.skills` through the call chain.

2. **R6.3: Missing SKILL.md didn't abort** — Install validation only warned on missing SKILL.md
   instead of aborting as required by spec. Fixed by replacing warning with `anyhow::bail!`.

3. **R6.3: Name mismatch didn't abort** — Install validation only warned on frontmatter name
   mismatch instead of aborting. Fixed by replacing warning with `anyhow::bail!`.

### Remaining Warnings

- Tasks 4.1–4.3 and 4.6 implemented but not marked `[x]` in tasks.md (cosmetic)
- No dedicated tests for R2 open-skills scenarios
- No tests for prompt rendering trust attributes (R4)
- No tests for install flow trust gating (R6.2)
- `filter_tools_by_trust()` (security boundary) has zero test coverage

---

## Architecture Decisions Followed

| Decision | Description                                                          | Status   |
|----------|----------------------------------------------------------------------|----------|
| AD1      | Trust as derived property (never stored independently)               | Followed |
| AD2      | Lockfile as advisory (missing/corrupt never blocks loading)          | Followed |
| AD3      | Backward compatibility (no lock entry → Local, SKILL.toml supported) | Followed |
| AD4      | No serde_yaml dependency (hand-rolled parser)                        | Followed |

---

## Follow-Up Changes Identified

### Phase 2: Official Skills Catalog

- Create official skills repository with `Official` source recognition
- Implement `skills update` command with lockfile ref pinning
- Add `skills audit` command to verify content hashes against lockfile

### Phase 3: Security Hardening

- Add unit tests for prompt rendering trust attributes (R4.1–R4.4)
- Add integration tests for install flow trust gating (R6.2–R6.3)
- Add unit tests for `filter_tools_by_trust()` security boundary
- Add tests for `open_skills_enabled()` config/env precedence (R2.2)
- Consider `Option<bool>` for `legacy_open_skills` to distinguish absent from explicit false

### Phase 4: Ecosystem

- SkillForge discovery integration with `Discovered` source variant
- Skill signing and verification (content hash → cryptographic signatures)
- Trust promotion workflow (ThirdParty → Official via review process)

---

## Archive Contents

| Artifact                   | Present |
|----------------------------|---------|
| proposal.md                | Yes     |
| exploration.md             | Yes     |
| design.md                  | Yes     |
| tasks.md                   | Yes     |
| specs/skills-trust/spec.md | Yes     |
| verify-report.md           | Yes     |
| archive-report.md          | Yes     |

---

## SDD Cycle Complete

The trusted-skills-distribution change has been fully planned, specified, designed, implemented,
verified, and archived. The source of truth for the skills trust model is now at
`openspec/specs/skills-trust/spec.md`. Ready for the next change.
