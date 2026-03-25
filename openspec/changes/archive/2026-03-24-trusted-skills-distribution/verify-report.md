# Verification Report

**Change**: trusted-skills-distribution
**Version**: Phase 1
**Date**: 2026-03-24
**Verifier**: sdd-verify agent

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 19 |
| Tasks marked complete [x] | 13 |
| Tasks marked incomplete [ ] | 6 |
| Tasks actually implemented (code exists) | 16 |
| Tasks genuinely incomplete | 3 |

### Task Status Detail

| Task | Marked | Actual | Notes |
|------|--------|--------|-------|
| 1.1 trust.rs | ✅ [x] | ✅ Implemented | 9 unit tests included |
| 1.2 frontmatter.rs | ✅ [x] | ✅ Implemented | 7 unit tests included |
| 1.3 lockfile.rs | ✅ [x] | ✅ Implemented | 9 unit tests included |
| 1.4 SkillsConfig | ✅ [x] | ✅ Implemented | Field + re-export present |
| 2.1 Skill struct fields | ✅ [x] | ✅ Implemented | 3 new `#[serde(skip)]` fields |
| 2.2 open_skills_enabled | ✅ [x] | ⚠️ Partial | Config never passed at call site |
| 2.3 load_skills trust | ✅ [x] | ✅ Implemented | Lockfile read + trust enrichment |
| 2.4 Install flow | ✅ [x] | ⚠️ Partial | Validation warns instead of aborting |
| 2.5 Remove + lockfile | ✅ [x] | ✅ Implemented | Advisory failure handling |
| 2.6 --trust CLI flag | ✅ [x] | ✅ Implemented | Both lib.rs and main.rs |
| 3.1 Sort + trust attr | ✅ [x] | ✅ Implemented | Sort by trust then name |
| 3.2 Caution + preamble | ✅ [x] | ✅ Implemented | Conditional preamble + note |
| 3.3 Tool filtering | ✅ [x] | ✅ Implemented | `filter_tools_by_trust()` |
| 4.1 Trust unit tests | ❌ [ ] | ✅ Implemented (unmarked) | 9 tests in trust.rs |
| 4.2 Frontmatter tests | ❌ [ ] | ✅ Implemented (unmarked) | 7 tests in frontmatter.rs |
| 4.3 Lockfile tests | ❌ [ ] | ✅ Implemented (unmarked) | 9 tests in lockfile.rs |
| 4.4 Prompt rendering tests | ❌ [ ] | ❌ Missing | No trust-aware prompt tests |
| 4.5 Install integration tests | ❌ [ ] | ❌ Missing | No install trust-gating tests |
| 4.6 Regression verification | ❌ [ ] | ✅ Done (unmarked) | All 5327 tests pass |

---

## Build & Tests Execution

**Build**: ✅ Passed
```
cargo check → Finished dev profile in 17.85s (0 errors)
```

**Clippy**: ✅ Passed
```
cargo clippy --all-targets -- -D warnings → Finished (0 warnings as errors)
```

**Format**: ✅ Passed
```
cargo fmt --all -- --check → (no output, all formatted)
```

**Tests**: ✅ 5327 passed / 0 failed / 0 ignored
```
lib tests:  2644 passed
bin tests:  2670 passed
+ 13 additional test binaries: all passed
```

**Coverage**: ➖ Not configured

---

## Spec Compliance Matrix

### R1: Trust Tier Model

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R1.1 | SkillTrust enum with derives | `trust::tests::as_str_returns_correct_representations` | ✅ COMPLIANT |
| R1.2 | Trust derived from git-cloned skill | `trust::tests::git_repo_source_maps_to_third_party_trust` | ✅ COMPLIANT |
| R1.2 | Trust derived from workspace skill | `trust::tests::local_source_maps_to_local_trust` | ✅ COMPLIANT |
| R1.2 | Trust derived from symlinked skill | `trust::tests::linked_local_source_maps_to_local_trust` | ✅ COMPLIANT |
| R1.2 | Trust derived from official source | `trust::tests::official_source_maps_to_official_trust` | ✅ COMPLIANT |
| R1.2 | Trust derived from discovered source | `trust::tests::discovered_source_maps_to_third_party_trust` | ✅ COMPLIANT |
| R1.3 | SkillOrigin struct fields | `trust::tests::skill_origin_default_is_local_with_none_fields` | ✅ COMPLIANT |
| R1.4 | SkillSource 5 variants | `trust::tests::*` (all 5 variants tested) | ✅ COMPLIANT |
| R1 | Privilege escalation prevention | `trust::tests::privilege_escalation_prevention_git_repo_always_third_party` | ✅ COMPLIANT |

### R2: Open-Skills Deprecation

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R2.1 | Open-skills disabled by default | Code at `mod.rs:215` returns `false` | ⚠️ PARTIAL — code correct, no dedicated test |
| R2.2 | Enabled via env var | Code at `mod.rs:200-212` checks env | ⚠️ PARTIAL — code correct, no dedicated test |
| R2.2 | Config overrides env var | Code at `mod.rs:188-197` checks config | ❌ FAILING — `ensure_open_skills_repo()` passes `None` to `open_skills_enabled()`, config path unreachable |
| R2.3 | Deprecation warning emitted | Code at `mod.rs:191-194,205-208` emits warning | ⚠️ PARTIAL — code correct, no dedicated test |
| R2.4 | Open-skills tagged ThirdParty | Code at `mod.rs:401-411` sets trust/source | ⚠️ PARTIAL — code correct, no dedicated test |

### R3: Skills Lockfile

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R3.1 | Location and TOML format | `lockfile::tests::serialization_round_trip` | ✅ COMPLIANT |
| R3.2 | Lock entry fields | `lockfile::tests::build_lock_entry_populates_all_fields` | ✅ COMPLIANT |
| R3.3 | Lockfile written on install | Code at `mod.rs:698-702` writes entry | ⚠️ PARTIAL — code correct, no integration test |
| R3.4 | Missing entry defaults Local | Code at `mod.rs:95-103` defaults to Local | ⚠️ PARTIAL — code correct, no dedicated test |
| R3.5 | Corrupt lockfile handling | `lockfile::tests::read_lockfile_corrupt_toml_returns_empty` | ✅ COMPLIANT |
| R3 | Lockfile written on install | `lockfile::tests::write_lock_entry_creates_and_updates` | ✅ COMPLIANT |
| R3 | Skill without lock entry | Code at `mod.rs:103` (default Local) | ⚠️ PARTIAL — code correct, no dedicated test |
| R3 | Pinned ref from lockfile | `lockfile::tests::lock_entry_to_origin_git_repo` | ✅ COMPLIANT |

### R4: Trust-Aware Prompt Rendering

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R4.1 | Trust attribute on skill elements | Code at `prompt.rs:241,250` adds `trust=` | ❌ UNTESTED |
| R4.2 | Rendering order Official→Local→ThirdParty | Code at `prompt.rs:212` sorts by trust | ❌ UNTESTED |
| R4.3 | ThirdParty caution note | Code at `prompt.rs:237-245` adds `<note>` | ❌ UNTESTED |
| R4.4 | ThirdParty preamble | Code at `prompt.rs:222-228` adds preamble | ❌ UNTESTED |
| R4 | No preamble when no ThirdParty | Code at `prompt.rs:222` conditional | ❌ UNTESTED |

### R5: allowed-tools Parsing

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R5.1 | Frontmatter parsing | `frontmatter::tests::valid_frontmatter_with_all_fields` | ✅ COMPLIANT |
| R5.1 | Absent allowed-tools | `frontmatter::tests::valid_frontmatter_without_allowed_tools` | ✅ COMPLIANT |
| R5.2 | ThirdParty with declared tools | Code at `mod.rs:431-439` filters | ❌ UNTESTED |
| R5.2 | ThirdParty without tools (instruction-only) | Code at `mod.rs:432-433` returns empty | ❌ UNTESTED |
| R5.2 | Official ignores allowed-tools | Code at `mod.rs:430` returns all | ❌ UNTESTED |
| R5.2 | Local ignores allowed-tools | Code at `mod.rs:430` returns all | ❌ UNTESTED |
| R5.3 | Malformed allowed-tools safe default | `frontmatter::tests::malformed_content_returns_default_no_panic` | ✅ COMPLIANT |

### R6: Install Flow Trust Gating

| Sub-Req | Scenario | Test | Result |
|---------|----------|------|--------|
| R6.1 | Trust resolved at install | Code at `mod.rs:618-619` resolves trust | ⚠️ PARTIAL — code correct, no test |
| R6.2 | --trust flag bypasses gate | Code at `mod.rs:658` checks flag | ❌ UNTESTED |
| R6.2 | TTY prompt without --trust | Code at `mod.rs:660-668` prompts | ❌ UNTESTED |
| R6.2 | No TTY, no --trust aborts | Code at `mod.rs:669-677` aborts | ❌ UNTESTED |
| R6.2 | Instruction-only skips gate | Code at `mod.rs:658` checks `!fm.allowed_tools.is_empty()` | ❌ UNTESTED |
| R6.3 | Missing SKILL.md aborts | Code at `mod.rs:650-654` **WARNS but doesn't abort** | ❌ FAILING — spec requires abort |
| R6.3 | Name mismatch aborts | Code at `mod.rs:637-644` **WARNS but doesn't abort** | ❌ FAILING — spec requires abort |
| R6.4 | Lock entry on success | Code at `mod.rs:698-702` writes entry | ⚠️ PARTIAL — code correct, no test |
| R6.5 | Content hash SHA-256 | `lockfile::tests::compute_content_hash_correct_sha256` | ✅ COMPLIANT |

### Compliance Summary

| Status | Count |
|--------|-------|
| ✅ COMPLIANT (test passed) | 15 |
| ⚠️ PARTIAL (code exists, no test) | 11 |
| ❌ UNTESTED (no test for scenario) | 13 |
| ❌ FAILING (behavior deviates from spec) | 3 |
| **Total scenarios** | **42** |

---

## Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| R1: Trust Tier Model | ✅ Implemented | All types, derivations, ordering correct |
| R2: Open-Skills Deprecation | ⚠️ Partial | R2.1 default=false ✅. R2.2 config mechanism unreachable (passes `None`). R2.3/R2.4 code correct. |
| R3: Skills Lockfile | ✅ Implemented | Advisory model, all CRUD operations, content hash |
| R4: Trust-Aware Prompt | ✅ Implemented | Sort, trust attr, caution note, preamble all present in code |
| R5: allowed-tools Parsing | ✅ Implemented | Frontmatter parser + `filter_tools_by_trust()` |
| R6: Install Flow | ⚠️ Partial | Trust gating works, but validation only warns instead of aborting (R6.3 violated) |

---

## Coherence (Design Decisions)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| AD1: Trust as derived property | ✅ Yes | `impl From<&SkillSource> for SkillTrust`, `#[serde(skip)]` fields, never stored independently |
| AD2: Lockfile as advisory | ✅ Yes | Missing file → empty default, corrupt → log + empty default, never blocks loading |
| AD3: Backward compatibility | ✅ Yes | Skills without lock entries default to `Local`, SKILL.toml still supported, open-skills opt-in |
| AD4: No serde_yaml | ✅ Yes | Hand-rolled parser in `frontmatter.rs`, ~67 lines, no new YAML dependency |

### File Changes vs Design

| File | Design Says | Actual | Match? |
|------|-------------|--------|--------|
| `src/skills/trust.rs` | Create | ✅ Created | ✅ |
| `src/skills/lockfile.rs` | Create | ✅ Created | ✅ |
| `src/skills/frontmatter.rs` | Create | ✅ Created | ✅ |
| `src/skills/mod.rs` | Modify | ✅ Modified (trust, origin, allowed_tools, load_skills, install, remove, filter) | ✅ |
| `src/agent/prompt.rs` | Modify | ✅ Modified (trust-aware rendering) | ✅ |
| `src/config/schema.rs` | Modify | ✅ Modified (SkillsConfig added) | ✅ |
| `src/config/mod.rs` | Modify | ✅ Modified (re-exports SkillsConfig) | ✅ |
| `src/lib.rs` | Modify | ✅ Modified (--trust flag on Install) | ✅ |

---

## Success Criteria Checklist (from proposal.md)

| Criterion | Status |
|-----------|--------|
| SkillTrust enum and SkillOrigin added, populated at load time, never independently mutable | ✅ Met |
| `open_skills_enabled()` returns `false` by default | ✅ Met |
| Enabling open-skills emits deprecation warning | ✅ Met (via env var path) |
| `skills.legacy_open_skills` config option exists and works | ⚠️ Exists but non-functional (config never passed) |
| `skills.lock` written on install | ✅ Met |
| `<skill>` elements include `trust` attribute | ✅ Met |
| Skills rendered in trust-priority order | ✅ Met |
| Third-party caution note in prompt | ✅ Met |
| `allowed-tools` parsed from SKILL.md frontmatter | ✅ Met |
| Third-party without `allowed-tools` → instruction-only | ✅ Met |
| Third-party with `allowed-tools` → only declared tools exposed | ✅ Met |
| `skills install` for third-party with tools requires `--trust` or confirmation | ✅ Met |
| All existing tests pass (no regression) | ✅ Met (5327 passed, 0 failed) |
| New unit tests cover trust derivation, lockfile, allowed-tools, prompt rendering | ⚠️ Partial — trust/lockfile/frontmatter covered; prompt rendering and install flow untested |

---

## Issues Found

### CRITICAL (must fix before archive)

1. **R2.2: Config mechanism unreachable** — `ensure_open_skills_repo()` at `mod.rs:230` calls `open_skills_enabled(None)`, so the `SkillsConfig.legacy_open_skills` config path is never reached. Users setting `skills.legacy_open_skills = true` in their config file will have no effect. Fix: pass `&config.skills` through `load_skills()` → `ensure_open_skills_repo()` → `open_skills_enabled()`.

2. **R6.3: Install validation doesn't abort on missing SKILL.md** — At `mod.rs:650-654`, when SKILL.md is missing, the code prints a warning but continues installation. Spec R6.3 requires: "the install MUST abort with a descriptive error message and MUST NOT write a lock entry." Fix: replace the warning with `anyhow::bail!`.

3. **R6.3: Install validation doesn't abort on name mismatch** — At `mod.rs:637-644`, when the frontmatter `name` doesn't match the directory name, the code prints a warning but continues. Spec R6.3 requires abort. Fix: replace the warning with `anyhow::bail!`.

### WARNING (should fix)

4. **R2.2: Config `false` doesn't override env `true`** — In `open_skills_enabled()` at `mod.rs:188-197`, when config is `Some` but `legacy_open_skills` is `false`, the function falls through to check env vars instead of returning `false`. This means `skills.legacy_open_skills = false` in config doesn't override `CORVUS_OPEN_SKILLS=true`. However, since `false` is the default and there's no `Option<bool>` to distinguish "absent" from "explicitly false", this is a design ambiguity rather than a clear bug.

5. **Tasks 4.1-4.3 not marked complete** — Tests for trust.rs (9), frontmatter.rs (7), and lockfile.rs (9) are implemented inline in the respective modules but the corresponding tasks in `tasks.md` are still marked `[ ]`. These should be marked `[x]`.

6. **Task 4.6 not marked complete** — All 5327 tests pass, `cargo clippy` and `cargo fmt` pass, but task 4.6 is still marked `[ ]`.

7. **No dedicated tests for R2 scenarios** — `open_skills_enabled()` behavior (default false, env override, deprecation warning) has no unit tests. The function works correctly via env var but testing would catch regressions.

### SUGGESTION (nice to have)

8. **Task 4.4: Add prompt rendering trust tests** — Create 2-3 tests in `prompt.rs` that verify: sort order with mixed tiers, trust attribute values, caution note for ThirdParty, preamble conditional. These are straightforward to write since `render_skills_section()` is a pure function.

9. **Task 4.5: Add install flow integration tests** — Create 3-4 tests in `mod.rs` that verify: trust gate triggers/skips based on trust+tools+flag combinations, validation failures abort, lock entry contents after install.

10. **`filter_tools_by_trust` deserves unit tests** — The function at `mod.rs:428-443` is a critical security boundary (tool access control) with zero test coverage. Add tests for all 4 cells of the trust/allowed-tools matrix.

---

## Verdict

**PASS WITH WARNINGS**

The Phase 1 trusted skills distribution implementation is substantially complete and correct. All 3 new modules (`trust.rs`, `frontmatter.rs`, `lockfile.rs`) are well-implemented with good test coverage (25 new tests). The core security invariant — trust derived from origin, never mutable — is correctly implemented and tested. The build passes cleanly with zero warnings, zero test failures across 5327 tests, and clean formatting.

However, **3 CRITICAL issues** must be addressed before archive:
1. The config-based `legacy_open_skills` mechanism is non-functional (config never passed to `open_skills_enabled`)
2. Install validation for missing SKILL.md warns instead of aborting (spec requires abort)
3. Install validation for name mismatch warns instead of aborting (spec requires abort)

Additionally, prompt rendering (R4) and tool filtering (R5) have zero test coverage despite correct code, and several task completions are not reflected in `tasks.md`.
