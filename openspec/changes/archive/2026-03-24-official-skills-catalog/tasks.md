# Tasks: Official Skills Catalog

## Phase 1: Foundation (Catalog Index, Build, Config, Frontmatter)

- [x] **1.1** Create `src/skills/catalog_index.toml` — committed index snapshot with `[meta]` (
  version=1, generated_at, commit placeholder) and no skill entries yet
    - **Files**: `clients/agent-runtime/src/skills/catalog_index.toml` (new)
    - **Dependencies**: None
    - **Acceptance**: Valid TOML with `meta.version = 1`; parseable by `toml` crate (R7.1)
    - **Complexity**: S
    - **Status**: pending

- [x] **1.2** Create `build.rs` — copy `src/skills/catalog_index.toml` to `OUT_DIR` via `fs::copy`;
  print `cargo:rerun-if-changed`
    - **Files**: `clients/agent-runtime/build.rs` (new), `clients/agent-runtime/Cargo.toml` (modify:
      add `build = "build.rs"`)
    - **Dependencies**: 1.1
    - **Acceptance**: `include_str!(concat!(env!("OUT_DIR"), "/catalog_index.toml"))` compiles; no
      new crate dependencies (R8.1)
    - **Complexity**: S
    - **Status**: pending

- [x] **1.3** Create `skills/catalog.rs` — define `CatalogIndex`, `CatalogMeta`, `CatalogEntry`
  structs with serde derives; define `OFFICIAL_REPO`, `OFFICIAL_INDEX_URL`,
  `DEFAULT_CACHE_TTL_HOURS`, `INDEX_FETCH_TIMEOUT_SECS` constants; implement `parse_index()` with
  schema version validation (reject version != 1); implement `is_bare_name()` heuristic (no `/`,
  `\`, `.`, `:`)
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (new),
      `clients/agent-runtime/src/skills/mod.rs` (modify: add `pub mod catalog;`)
    - **Dependencies**: 1.1, 1.2
    - **Acceptance**: Valid index parses to `CatalogIndex` (R7.1 scenario 1); unknown version
      rejected with error including version number (R7.1 scenario 2); missing required field
      rejected with field+skill name (R7.1 scenario 3); `is_bare_name("git-expert")` → true,
      `is_bare_name("https://url")` → false (R9.1)
    - **Complexity**: M
    - **Status**: pending

- [x] **1.4** Implement `resolve_index()` in `catalog.rs` — cache TTL check via file mtime, HTTP
  fetch with 3s timeout (existing `reqwest` blocking), embedded fallback chain; implement
  `try_cached_index()`, `try_fetch_index()` helpers; `EMBEDDED_INDEX` const via `include_str!`
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (modify)
    - **Dependencies**: 1.3
    - **Acceptance**: Fresh cache used without network (R8.3 scenario 2); stale cache triggers
      fetch (R8.3 scenario 3); network failure falls back to embedded (R8.3 scenario 4); both
      corrupted fails gracefully without panic (R8.4 scenario 5); lazy parsing — not parsed at
      startup (R8.2)
    - **Complexity**: M
    - **Status**: pending

- [x] **1.5** Implement `search()` in `catalog.rs` — case-insensitive substring match against name,
  description, and tags
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (modify)
    - **Dependencies**: 1.3
    - **Acceptance**: `search(index, "git")` matches `git-expert` and `git-flow` but not
      `rust-expert` (R9.5 scenario 14); search works against embedded index offline (R9.5 scenario
        15)
    - **Complexity**: S
    - **Status**: pending

- [x] **1.6** Extend `SkillsConfig` in `config/schema.rs` — add `catalog_repo_url: Option<String>`
  and `catalog_cache_ttl_hours: Option<u64>` fields with `#[serde(default)]`
    - **Files**: `clients/agent-runtime/src/config/schema.rs` (modify)
    - **Dependencies**: None
    - **Acceptance**: Existing configs without new fields parse without error; new fields accessible
      when set (R8.3 TTL configurability)
    - **Complexity**: S
    - **Status**: done

- [ ] **1.7** Extend `SkillFrontmatter` in `frontmatter.rs` — add `version: Option<String>`,
  `author: Option<String>`, `tags: Vec<String>` fields; extend `parse_frontmatter_block()` with
  match arms for `"version"`, `"author"`, `"tags"` (tags reuses `allowed-tools` list pattern)
    - **Files**: `clients/agent-runtime/src/skills/frontmatter.rs` (modify)
    - **Dependencies**: None
    - **Acceptance**: Frontmatter with `version: 1.2.0`, `author: "Jane Doe"`, `tags: [git, vcs]`
      parsed correctly (R10.1 scenario 21); missing new fields default to None/empty (R10.1 scenario
        22)
    - **Complexity**: S
    - **Status**: pending

- [x] **1.8** Update `lock_entry_to_origin()` in `lockfile.rs` — recognize `"official:"` prefix in
  source field; reconstruct `SkillSource::Official { repo, path }`; update `build_lock_entry()` to
  accept optional `path` parameter
    - **Files**: `clients/agent-runtime/src/skills/lockfile.rs` (modify)
    - **Dependencies**: None
    - **Acceptance**: `source = "official:dallay/corvus-skills"` with `path = "skills/git-expert"` →
      `SkillSource::Official` with correct fields (R3.6 scenario 31); git URL source still maps to
      `GitRepo`/`ThirdParty` (R3.6 scenario 32)
    - **Complexity**: S
    - **Status**: pending

## Phase 2: Catalog UX (Install, Search, Update, List, Discover)

- [x] **2.1** Expand `SkillCommands` enum in `lib.rs` — add `Search { query }`,
  `Update { name: Option<String> }`, `Lock { cmd: LockCommands }`,
  `Discover { query: Option<String> }` variants; modify `List` to include
  `#[arg(long)] catalog: bool`; add `LockCommands` sub-enum with `Repair` variant
    - **Files**: `clients/agent-runtime/src/lib.rs` (modify), `clients/agent-runtime/src/main.rs` (
      modify: route new variants)
    - **Dependencies**: None
    - **Acceptance**: `corvus skills search`, `corvus skills update`, `corvus skills lock repair`,
      `corvus skills discover`, `corvus skills list --catalog` are valid CLI invocations
    - **Complexity**: S
    - **Status**: pending

- [x] **2.2** Update install flow for bare-name detection — in `handle_install_command()` in
  `skills/mod.rs`, call `catalog::is_bare_name()`; on true, call `resolve_index()` + lookup name; on
  catalog hit, clone from `OFFICIAL_REPO` at entry path; set `SkillSource::Official`; write lock
  entry with `"official:"` prefix source and `trust = "official"`; no trust gate for Official
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify)
    - **Dependencies**: 1.3, 1.4, 1.8, 2.1
    - **Acceptance**: `skills install git-expert` resolves from catalog, sets Official trust, no
      trust gate (R9.2, R9.3 scenario 11); catalog miss fails with helpful error suggesting
      search/URL (R9.4 scenario 12); URL input bypasses catalog (R9.1 scenario 13); privilege
      escalation via URL to official repo prevented — produces ThirdParty (R9.3 scenario 17, AD5)
    - **Complexity**: L
    - **Status**: done

- [x] **2.3** Implement `handle_search_command()` in `skills/mod.rs` — resolve catalog index, call
  `catalog::search()`, display results table (name, version, description)
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify)
    - **Dependencies**: 1.4, 1.5, 2.1
    - **Acceptance**: Partial match returns matching entries with name/version/description (R9.5
      scenario 14); works offline via embedded fallback (R9.5 scenario 15); first run with no cache
      uses embedded (R8.3 scenario 6)
    - **Complexity**: S
    - **Status**: pending

- [x] **2.4** Implement `handle_list_catalog()` in `skills/mod.rs` — resolve catalog index, display
  all entries, mark installed skills with `[installed]` by cross-referencing lockfile
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify)
    - **Dependencies**: 1.4, 2.1
    - **Acceptance**: All catalog skills listed; installed ones visually marked; uninstalled ones
      not marked (R9.6 scenario 16); cached index used when fresh, no network request (R8.3 scenario
        7)
    - **Complexity**: S
    - **Status**: pending

- [x] **2.5** Implement `handle_update_command()` in `skills/mod.rs` — for Official skills: resolve
  catalog index, compare `content_hash`, re-fetch if different, update lockfile; for GitRepo:
  re-clone from source URL, update lockfile; for Local/LinkedLocal: skip with info message; support
  update-all (no name arg) and update-by-name
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify)
    - **Dependencies**: 1.4, 1.8, 2.1
    - **Acceptance**: Official skill with newer hash re-fetched and lockfile updated (R13.3 scenario
      26); up-to-date Official skill reported as current (R13.3 scenario 27); ThirdParty re-fetched
      from source URL (R13.4 scenario 28); local skill skipped with message (R13.5 scenario 29);
      update-all processes all types correctly (R13.1 scenario 30); offline update compares against
      embedded/cached (R13 scenario 31); nonexistent skill fails with clear error (R13.2 scenario
      32); trust and source fields unchanged after update (R13.6)
    - **Complexity**: L
    - **Status**: pending

## Phase 3: Cleanup (Deprecation, SkillForge Trust, Lock Repair)

- [x] **3.1** Add SKILL.toml deprecation warning — in the skill loading path in `skills/mod.rs`,
  when loading from `SKILL.toml` (not `SKILL.md`), emit `tracing::warn!` containing
  `"SKILL.toml is deprecated"` and migration suggestion
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify)
    - **Dependencies**: None
    - **Acceptance**: Loading SKILL.toml emits warning containing "SKILL.toml is deprecated" (R10.2
      scenario 23); SKILL.toml continues to load correctly (R10.4)
    - **Complexity**: S
    - **Status**: pending

- [x] **3.2** Update SkillForge integrator — modify `integrate()` in `skillforge/integrate.rs` to
  generate only SKILL.md with YAML frontmatter (name, description, version, author, tags); remove
  SKILL.toml generation (delete `generate_toml()` call and file write); update `generate_md()` to
  include frontmatter block
    - **Files**: `clients/agent-runtime/src/skillforge/integrate.rs` (modify)
    - **Dependencies**: 1.7
    - **Acceptance**: SkillForge generates SKILL.md with YAML frontmatter; no SKILL.toml generated (
      R10.3 scenario 24)
    - **Complexity**: M
    - **Status**: pending

- [x] **3.3** Implement `skills discover` command — add `discover()` method to `SkillForge` in
  `skillforge/mod.rs` that runs Scout → Evaluate pipeline and returns results without writing to
  disk; display results as table (name, URL, score); print hint to install via
  `skills install <url>`
    - **Files**: `clients/agent-runtime/src/skillforge/mod.rs` (modify),
      `clients/agent-runtime/src/skills/mod.rs` (modify: wire `Discover` variant)
    - **Dependencies**: 2.1
    - **Acceptance**: Results displayed without writing files or modifying lockfile (R11.1, R11.2
      scenario 25); user installs discovered skill via URL → ThirdParty trust gate (R11.3 scenario
        26)
    - **Complexity**: M
    - **Status**: done

- [x] **3.4** Deprecate `auto_integrate` config — in `SkillForgeConfig` or SkillForge startup, when
  `auto_integrate` is set to true, emit deprecation warning and ignore the setting; auto-integration
  must not execute
    - **Files**: `clients/agent-runtime/src/skillforge/mod.rs` (modify)
    - **Dependencies**: None
    - **Acceptance**: `skillforge.auto_integrate = true` in config emits deprecation warning; no
      auto-integration pipeline runs (R11.4 scenario 27)
    - **Complexity**: S
    - **Status**: done

- [x] **3.5** Implement `skills lock repair` — add `repair_lockfile()` in `lockfile.rs`: scan
  `{workspace}/skills/` for dirs with SKILL.md or SKILL.toml; read existing lockfile (tolerate
  corruption → start empty); for each disk skill: verify/create/update entry; remove orphaned
  entries; recompute content hashes; write repaired lockfile; return `RepairSummary` with counts;
  wire to `handle_lock_repair_command()` in `skills/mod.rs`
    - **Files**: `clients/agent-runtime/src/skills/lockfile.rs` (modify: add `RepairSummary`
      struct + `repair_lockfile()` fn), `clients/agent-runtime/src/skills/mod.rs` (modify: add
      handler)
    - **Dependencies**: 1.8, 2.1
    - **Acceptance**: Missing entries added with `trust = "local"` (R12.2 scenario 33); orphaned
      entries removed (R12.3 scenario 34); hash mismatches updated (R12.4 scenario 35); corrupt
      lockfile rebuilt from scratch (R12.6 scenario 36); empty skills dir → zero entries (R12.3
      scenario 37); summary reports correct counts for verified/added/removed/updated (R12.5)
    - **Complexity**: L
    - **Status**: pending

## Phase 4: Testing

- [x] **4.1** Unit tests for catalog parsing — valid index parse (scenario 1), unknown schema
  version rejection (scenario 2), missing required field rejection (scenario 3), `is_bare_name()`
  positive/negative cases
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (modify: add
      `#[cfg(test)] mod tests`)
    - **Dependencies**: 1.3
    - **Acceptance**: All R7 scenarios covered; `is_bare_name` exhaustive assertions (R9.1)
    - **Complexity**: M
    - **Status**: pending

- [ ] **4.2** Unit tests for index resolution — cached index used when fresh (scenario 7), first run
  uses embedded (scenario 6), both corrupted fails gracefully (scenario 10)
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (modify: extend tests)
    - **Dependencies**: 1.4
    - **Acceptance**: Tempdir-based tests verify fallback chain; no panic on any path (R8.3, R8.4)
    - **Complexity**: M
    - **Status**: pending

- [x] **4.3** Unit tests for catalog search — partial match returns correct results (scenario 14),
  case-insensitive, empty result for no match, tag matching
    - **Files**: `clients/agent-runtime/src/skills/catalog.rs` (modify: extend tests)
    - **Dependencies**: 1.5
    - **Acceptance**: R9.5 scenarios verified against test index
    - **Complexity**: S
    - **Status**: pending

- [ ] **4.4** Unit tests for extended frontmatter — version/author/tags parsed (scenario 21),
  missing fields default to None (scenario 22), existing tests still pass
    - **Files**: `clients/agent-runtime/src/skills/frontmatter.rs` (modify: extend existing
      `mod tests`)
    - **Dependencies**: 1.7
    - **Acceptance**: R10.1 scenarios 21-22 pass; all 7 existing frontmatter tests unchanged
    - **Complexity**: S
    - **Status**: pending

- [x] **4.5** Unit tests for lockfile official source — `lock_entry_to_origin()` with `"official:"`
  prefix (scenario 31), git URL still maps to ThirdParty (scenario 32), `build_lock_entry()` with
  path field
    - **Files**: `clients/agent-runtime/src/skills/lockfile.rs` (modify: extend existing
      `mod tests`)
    - **Dependencies**: 1.8
    - **Acceptance**: R3.6 scenarios 31-32 pass; existing lockfile tests unchanged
    - **Complexity**: S
    - **Status**: pending

- [ ] **4.6** Unit tests for lockfile repair — adds missing entries (scenario 33), removes orphans (
  scenario 34), updates hash mismatch (scenario 35), rebuilds from corrupt lockfile (scenario 36),
  empty skills dir (scenario 37)
    - **Files**: `clients/agent-runtime/src/skills/lockfile.rs` (modify: extend tests)
    - **Dependencies**: 3.5
    - **Acceptance**: All R12 scenarios pass with tempdir-based tests; `RepairSummary` counts
      verified
    - **Complexity**: M
    - **Status**: pending

- [ ] **4.7** Integration test for catalog install flow — bare name → index lookup → clone (
  mocked/tempdir) → lockfile entry with Official source and trust; privilege escalation test: URL to
  official repo → ThirdParty
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify: add integration test) or
      `clients/agent-runtime/tests/` (new)
    - **Dependencies**: 2.2
    - **Acceptance**: R9.2/R9.3 scenario 11 end-to-end; R6.1 scenario 38 (Official bypass trust
      gate); privilege escalation scenario 17 verified
    - **Complexity**: M
    - **Status**: pending

- [ ] **4.8** Integration test for SKILL.toml deprecation warning — load skill from SKILL.toml,
  capture tracing output, verify warning emitted
    - **Files**: `clients/agent-runtime/src/skills/mod.rs` (modify: add test)
    - **Dependencies**: 3.1
    - **Acceptance**: R10.2 scenario 23 verified via tracing subscriber capture
    - **Complexity**: S
    - **Status**: pending

- [ ] **4.9** Regression verification — run `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`; all existing Phase 1
  tests pass; no new dependencies in `Cargo.toml`
    - **Files**: None (verification only)
    - **Dependencies**: All previous tasks
    - **Acceptance**: Zero test failures; zero clippy warnings; format clean; no new crates added
    - **Complexity**: S
    - **Status**: pending

## Scenario-to-Task Traceability Matrix

| Scenario                                                     | Requirement  | Task(s)       |
|--------------------------------------------------------------|--------------|---------------|
| S1: Valid catalog index parsed successfully                  | R7.1         | 1.3, 4.1      |
| S2: Index with unknown schema version rejected               | R7.1         | 1.3, 4.1      |
| S3: Index with missing required field rejected               | R7.2         | 1.3, 4.1      |
| S4: First run with no cache uses embedded index              | R8.3         | 1.4, 4.2      |
| S5: Cached index used when fresh                             | R8.3         | 1.4, 4.2      |
| S6: Stale cache triggers refresh                             | R8.3         | 1.4, 4.2      |
| S7: Network failure falls back to embedded                   | R8.4         | 1.4, 4.2      |
| S8: Both cache and embedded corrupted                        | R8.4         | 1.4, 4.2      |
| S9: Install by bare name from catalog                        | R9.2, R9.3   | 2.2, 4.7      |
| S10: Install bare name not in catalog                        | R9.4         | 2.2, 4.7      |
| S11: Bare-name detection distinguishes catalog from URL/path | R9.1         | 1.3, 4.1      |
| S12: Search with partial match                               | R9.5         | 1.5, 2.3, 4.3 |
| S13: Search works offline                                    | R9.5         | 1.5, 2.3, 4.3 |
| S14: List catalog shows install status                       | R9.6         | 2.4           |
| S15: Privilege escalation via URL to official repo prevented | R9.3, AD5    | 2.2, 4.7      |
| S16: Frontmatter with new fields parsed correctly            | R10.1        | 1.7, 4.4      |
| S17: Missing new fields default to None                      | R10.1        | 1.7, 4.4      |
| S18: SKILL.toml loaded with deprecation warning              | R10.2        | 3.1, 4.8      |
| S19: SkillForge generates only SKILL.md                      | R10.3        | 3.2           |
| S20: Discover shows results without installing               | R11.1, R11.2 | 3.3           |
| S21: Installing a discovered skill goes through trust gate   | R11.3        | 3.3, 2.2      |
| S22: auto_integrate config option ignored with warning       | R11.4        | 3.4           |
| S23: Repair adds missing entries                             | R12.2        | 3.5, 4.6      |
| S24: Repair removes orphaned entries                         | R12.3        | 3.5, 4.6      |
| S25: Repair updates mismatched hash                          | R12.4        | 3.5, 4.6      |
| S26: Repair with corrupt lockfile rebuilds from scratch      | R12.6        | 3.5, 4.6      |
| S27: Repair with empty skills directory                      | R12.3        | 3.5, 4.6      |
| S28: Update official skill with newer version available      | R13.3        | 2.5           |
| S29: Update official skill already up-to-date                | R13.3        | 2.5           |
| S30: Update third-party skill                                | R13.4        | 2.5           |
| S31: Update skips local skill                                | R13.5        | 2.5           |
| S32: Update all skills                                       | R13.1        | 2.5           |
| S33: Update when offline                                     | R13          | 2.5           |
| S34: Update nonexistent skill                                | R13.2        | 2.5           |
| S35: Official lockfile entry reconstructed correctly         | R3.6         | 1.8, 4.5      |
| S36: Git URL lockfile entry still maps to ThirdParty         | R3.6         | 1.8, 4.5      |
| S37: Official catalog install bypasses trust gate            | R6.1         | 2.2, 4.7      |
| S38: Continued SKILL.toml support                            | R10.4        | 3.1, 4.8      |
