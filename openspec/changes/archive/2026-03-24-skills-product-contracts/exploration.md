## Exploration: skills-product-contracts

### Current State

The Corvus skills trust system is substantially implemented across 3 phases (commits `682fbeed` and `ef20eb39`). The main spec (`openspec/specs/skills-trust/spec.md`) defines R1-R20, all of which have corresponding code in `clients/agent-runtime/src/skills/`. This change formalizes **product contracts** for 4 child workstreams that close out the trust initiative.

Key implementation files reviewed:
- `skills/mod.rs` — Core loader, CLI handlers, prompt builder (1597+ lines)
- `skills/trust.rs` — SkillTrust enum, SkillSource, SkillOrigin, derivation (163 lines)
- `skills/validation.rs` — Name validation `[a-z0-9-]`, 1-64 chars, no `--` (125 lines)
- `skills/scanner.rs` — 5-category injection scanner, scoring-based (248 lines)
- `skills/sandbox.rs` — Path traversal prevention, symlink resolution (176 lines)
- `skills/lockfile.rs` — skills.lock TOML, integrity verification, repair (710 lines)
- `skills/catalog.rs` — Embedded index, cache+TTL, search, bare-name resolution (397 lines)
- `skills/frontmatter.rs` — Custom YAML parser for SKILL.md (240 lines)
- `lib.rs:137-184` — SkillCommands enum (List, Install, Remove, Search, Update, Discover, Lock)
- `config/schema.rs:238-266` — SkillsConfig (catalog_repo_url, catalog_cache_ttl_hours, verify_integrity, scan_threshold)

---

### Workstream 1: Local Custom Skills UX

#### Implemented
| Feature | Status | Location |
|---------|--------|----------|
| Skills directory: `{workspace}/skills/` | Done | `mod.rs:309-311` (`skills_dir()`) |
| Init with README scaffold | Done | `mod.rs:314-348` (`init_skills_dir()`) |
| Local install: symlink (Unix), junction/copy fallback (Windows) | Done | `mod.rs:1041-1081` |
| Minimum structure: `SKILL.md` with optional YAML frontmatter | Done | `mod.rs:178-191`, `frontmatter.rs` |
| SKILL.toml-only dirs skipped with migration warning | Done | `mod.rs:184-189` |
| Name validation (warn on load, error on install) | Done | `mod.rs:173-176`, `mod.rs:868-873` |
| Frontmatter fields: name, description, version, author, tags, allowed-tools | Done | `frontmatter.rs:6-14` |
| List command shows name, version, description, tools, tags | Done | `mod.rs:458-478` |
| Remove command with path safety checks | Done | `mod.rs:1344-1388` |
| Trust defaults to Local for unlocked skills | Done | `mod.rs:78-79` |

#### Gaps
| Gap | Type | Priority |
|-----|------|----------|
| No `corvus skills init <name>` scaffolding command to create a new skill from template | Product decision | Low — nice-to-have, not blocking |
| No explicit "local skills authoring guide" documentation beyond README | Documentation | Medium |
| List command doesn't show trust tier for each skill | Code (minor) | Low |
| No `--link` flag on install to force symlink (currently auto-detects local path) | Product decision | Low |

#### Assessment
**95% complete.** The directory layout, install flow, validation, listing, and removal are all working. The remaining items are UX polish and documentation. No blocking code changes needed — only product contract documentation to formalize the existing behavior.

---

### Workstream 2: Third-Party Source Policy

#### Implemented
| Feature | Status | Location |
|---------|--------|----------|
| Third-party = any non-catalog git URL → `SkillSource::GitRepo` | Done | `trust.rs:46`, `mod.rs:972-980` |
| `--trust` flag for explicit consent on install | Done | `lib.rs:149-150`, `mod.rs:909` |
| Interactive TTY prompt when `--trust` absent | Done | `mod.rs:910-929` |
| Non-TTY abort requiring `--trust` | Done | `mod.rs:920-928` |
| Instruction-only third-party skills install without trust gate | Done | `mod.rs:909` (check: `!fm.allowed_tools.is_empty()`) |
| Scanner blocks high-risk installs (overridable with `--trust`) | Done | `mod.rs:876-906` |
| Trust-aware prompt rendering (caution notes for ThirdParty) | Done | Referenced in spec R4 |
| Privilege escalation prevention (URL to official repo → still ThirdParty) | Done | `trust.rs:150-162` |
| HTTP-only sources rejected | Done | `mod.rs:822-824` |

#### Gaps
| Gap | Type | Priority | Decision Needed |
|-----|------|----------|-----------------|
| **Source allowlisting** — no config for pre-approved third-party sources | Code + Design | **Medium** | Per-workspace, global, or both? Config key name? |
| No way to revoke trust after install (must remove + reinstall) | Product decision | Low | Is remove+reinstall sufficient? |
| No `--trust` equivalent for `skills update` (re-consent on updated third-party) | Code (minor) | Low | Should update re-trigger trust gate if allowed_tools changed? |

#### Key Decision: Source Allowlisting

**Options:**

| Approach | Pros | Cons | Effort |
|----------|------|------|--------|
| A. Config-based allowlist (`skills.trusted_sources = ["github.com/org/*"]`) | Simple, declarative, works in CI | Glob matching adds complexity | Medium |
| B. Per-workspace `.corvus/trusted-sources` file | Workspace-scoped, versionable | Another config file to manage | Medium |
| C. Defer — document `--trust` as the only path | No code needed, simplest | CI/automation friction | Low |

**Recommendation:** Option C (defer) for this change. The `--trust` flag + lockfile already cover the primary use case. Source allowlisting can be a follow-up when CI automation demand materializes. Document this as a future enhancement in the product contract.

#### Assessment
**90% complete.** The core trust gating is solid. Source allowlisting is the only meaningful gap, and it can be deferred with proper documentation. No blocking code changes needed for the product contract.

---

### Workstream 3: Skill Install Lifecycle

#### Implemented
| Feature | Status | Location |
|---------|--------|----------|
| Canonical identity: skill name (directory name = frontmatter name) | Done | `mod.rs:843-855` |
| Local registry: `skills.lock` (TOML) | Done | `lockfile.rs` |
| Source resolution: bare name → catalog, URL → git, path → local | Done | `mod.rs:811-815`, `catalog.rs:70-76` |
| Install validation: SKILL.md exists, frontmatter parses, name matches dir | Done | `mod.rs:838-873` |
| Content hash computed and stored on install | Done | `mod.rs:857-858`, `lockfile.rs:74-78` |
| Update flow: official (catalog hash compare), third-party (re-fetch), local (skip) | Done | `mod.rs:1092-1342` |
| Lock repair: add missing, remove orphaned, recompute hashes | Done | `lockfile.rs:134-210` |
| Integrity verification on load with per-tier behavior | Done | `mod.rs:82-122` |
| Scanner check on load (ThirdParty only) | Done | `mod.rs:124-139` |
| Sandbox flag derived from trust tier | Done | `mod.rs:141-144` |

#### Gaps
| Gap | Type | Priority | Decision Needed |
|-----|------|----------|-----------------|
| **Structured CLI output** — all commands use `println!`, no `--json` or `--format` flag | Code | **Medium** | Should all commands support `--json`? Or just list/search? |
| **Agent-driven auto-install** — no policy for agents to install skills programmatically | Product decision | **High** | Allow? Require confirmation? Restrict to catalog-only? |
| No `skills info <name>` command to show detailed skill metadata | Code (minor) | Low | Useful but not blocking |
| No `skills verify` command to check integrity on-demand | Code (minor) | Low | `lock repair` partially covers this |

#### Key Decision: Agent-Driven Auto-Install

This is the most important open question across all workstreams.

**Options:**

| Approach | Pros | Cons | Effort |
|----------|------|------|--------|
| A. **Catalog-only auto-install** — agents can install Official skills by name without user confirmation | Safe (only trusted catalog), enables seamless agent workflows | Still requires network/catalog access | Medium |
| B. **No auto-install** — agents must ask user to run `corvus skills install` manually | Maximum safety, no surprise installs | Poor agent UX, breaks flow | Low |
| C. **Confirmation-gated auto-install** — agent proposes install, user confirms in-band | Balance of safety and UX | Requires confirmation UI in agent loop | High |

**Recommendation:** Option A (catalog-only) with a config toggle `skills.agent_auto_install = "catalog-only" | "none"` (default: `"catalog-only"`). This is safe because catalog skills are Official and already trusted. Document the policy boundary clearly: agents MUST NOT auto-install third-party skills.

#### Key Decision: Structured CLI Output

**Recommendation:** Add `--json` flag to `list`, `search`, and `list --catalog` commands. These are the most likely to be consumed by scripts or agents. Install/update/remove can remain human-readable for now. Use `serde_json` (already a dependency) to serialize skill metadata.

#### Assessment
**85% complete.** The install lifecycle is fully functional. The two meaningful gaps are structured output (code) and agent-driven auto-install (policy). Both need product contract documentation; structured output also needs a small code change.

---

### Workstream 4: Official Catalog Model

#### Implemented
| Feature | Status | Location |
|---------|--------|----------|
| Official repo: `dallay/corvus-skills` | Done | `catalog.rs:7` |
| Embedded index at build time (`build.rs` + `include_str!`) | Done | `catalog.rs:20` |
| Cache with TTL (default 24h, configurable) | Done | `catalog.rs:80-110` |
| Fallback chain: cache → fetch (3s timeout) → embedded | Done | `catalog.rs:80-110` |
| CatalogEntry: name, description, version, path, content_hash, author, tags | Done | `catalog.rs:40-53` |
| Schema version validation (must be 1) | Done | `catalog.rs:57-66` |
| Search: case-insensitive substring match on name, description, tags | Done | `catalog.rs:181-195` |
| `list --catalog` with [installed] markers | Done | `mod.rs:406-447` |
| Bare-name install resolves against catalog → Official trust | Done | `mod.rs:692-809` |
| Catalog miss: helpful error with suggestions | Done | `mod.rs:700-715` |

#### Gaps
| Gap | Type | Priority | Decision Needed |
|-----|------|----------|-----------------|
| **Review standard** — no defined process for accepting skills into the catalog | Documentation | **High** | What criteria? Who reviews? What's the submission workflow? |
| **Contribution guide** for `dallay/corvus-skills` repo | Documentation | **High** | PR template, required fields, testing expectations |
| No CI pipeline to auto-regenerate `catalog_index.toml` on skill repo changes | Infrastructure | Medium | GitHub Action in `dallay/corvus-skills` |
| Catalog versioning is content_hash-based only — no SemVer enforcement | Product decision | Low | Is content_hash sufficient? |
| No catalog entry removal/deprecation policy | Documentation | Low | What happens when a skill is removed from catalog? |

#### Key Decision: Review Standard

**Recommendation:** Define a lightweight review checklist:
1. Valid SKILL.md with complete frontmatter (name, description, version, tags)
2. Name passes `validate_skill_name()` rules
3. Scanner score = 0 (no injection patterns)
4. Functional description of what the skill does
5. At least one maintainer review (PR-based workflow)

This should be documented in the `dallay/corvus-skills` repo's CONTRIBUTING.md and referenced from the product contract.

#### Assessment
**85% complete.** The catalog system is fully functional. The gaps are entirely documentation and governance — no code changes needed. The review standard and contribution guide are the critical deliverables.

---

### Affected Areas

- `clients/agent-runtime/src/skills/mod.rs` — Structured output addition (`--json` flag)
- `clients/agent-runtime/src/lib.rs` — CLI flag additions (if `--json` added)
- `openspec/specs/skills-trust/spec.md` — May need addenda for new requirements
- `dallay/corvus-skills` repo — Contribution guide, CI pipeline (external)

### Recommended Deliverables

#### Product Contracts (Documentation — primary output)

1. **Local Skills Authoring Contract** — Directory layout, file structure, frontmatter schema, validation rules, local install behavior (symlink vs copy), listing/removal semantics
2. **Third-Party Source Policy Contract** — What counts as third-party, consent model (`--trust`), scanner behavior, trust gate flow, source allowlisting (deferred with rationale)
3. **Skill Install Lifecycle Contract** — Source resolution order, validation pipeline, lockfile semantics, update flow, repair semantics, agent-driven install policy
4. **Official Catalog Model Contract** — Repository governance, CatalogEntry schema, review checklist, submission workflow, embedded index lifecycle, versioning model

#### Code Changes (minimal)

| Change | Effort | Priority |
|--------|--------|----------|
| Add `--json` flag to `list`, `search`, `list --catalog` | Small | Medium |
| Add agent auto-install policy config (`skills.agent_auto_install`) | Small | Medium — can defer to follow-up |

#### Decisions Needed

| Decision | Owner | Impact |
|----------|-------|--------|
| Source allowlisting: defer or implement now? | Product | Low — recommend defer |
| Agent-driven auto-install policy | Product | **High** — affects agent UX |
| Structured CLI output scope (`--json` on which commands?) | Engineering | Medium |
| Catalog review standard and submission workflow | Product + Maintainer | **High** — governs catalog growth |
| Catalog index CI regeneration | Infrastructure | Medium — blocks catalog contributions |

### Risks

- **Agent auto-install without policy** — If agents attempt to install skills and no policy exists, they'll get CLI errors with no recovery path. This is the highest-priority gap.
- **Catalog growth stalled** — Without a review standard and contribution guide, the catalog remains empty or stagnant. The embedded index shipped at build time will be the only source.
- **Structured output debt** — Scripts and CI pipelines parsing `println!` output will break on format changes. The longer `--json` is deferred, the more fragile consumers accumulate.

### Ready for Proposal

**Yes.** The exploration confirms that the implementation is 85-95% complete across all workstreams. The remaining work is primarily documentation (product contracts) with two small optional code changes. The proposal should:
1. Define the 4 product contract documents as primary deliverables
2. Capture the 4 key decisions (source allowlisting, agent auto-install, structured output, review standard)
3. Optionally scope the `--json` code change as a bonus task
4. Reference existing spec R1-R20 as the implementation authority
