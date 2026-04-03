# Proposal: Official Skills Catalog

## Intent

Phase 1 established the trust model, lockfile, and security boundaries for Corvus skills. However,
`SkillSource::Official` exists as a dead variant — no code path produces it. Users cannot discover
or install curated skills from a trusted catalog, and SkillForge auto-integrates discovered skills
without trust enforcement.

This change delivers the official skills catalog infrastructure: an in-repo index format, embedded
offline-capable index, catalog-aware install/search/update commands, SKILL.toml deprecation,
SkillForge trust boundary enforcement, and lockfile repair tooling. Together, these close the gap
between the trust model and the user experience of finding and installing trusted skills.

## Scope

### In Scope

1. **Official catalog index format** — Define `index.toml` schema and implement `CatalogIndex` /
   `CatalogEntry` data structures. The contract for `dallay/corvus-skills` is defined here; the
   actual repo creation is infrastructure (out of scope).

2. **Embedded index with offline fallback** — Add `build.rs` to embed a committed `index.toml`
   snapshot via `include_str!`. Runtime parses lazily on first use. Cached index refreshes
   periodically (24h TTL) from GitHub raw content. Falls back to embedded when offline.

3. **Catalog install path** — When `corvus skills install <name>` receives a bare name (no `/`,
   `\`, `.`, or `:`), resolve against the catalog index, clone from the official repo, set
   `SkillSource::Official`, and assign `Official` trust. New `corvus skills search <query>`
   subcommand for fuzzy-matching against the catalog. New `corvus skills list --catalog` to show
   all available official skills.

4. **SKILL.toml deprecation** — Extend the frontmatter parser to handle `version`, `author`,
   `tags` fields. Emit a deprecation warning when loading SKILL.toml. Update SkillForge
   integrator to stop generating SKILL.toml.

5. **SkillForge trust boundaries** — Replace auto-integrate pipeline with explicit
   `corvus skills discover` command. Discovered skills display results without auto-installing.
   Installation goes through standard ThirdParty trust-gated flow. Deprecate `auto_integrate`
   config option.

6. **Lockfile maintenance** — `corvus skills lock repair` command: scan disk, rebuild missing
   entries, remove orphans, recompute hashes, report summary.

7. **Skills update command** — `corvus skills update [name]` to update installed skills. Official
   skills check the catalog for newer versions. Third-party skills re-fetch from source URL.

### Out of Scope

- Actual `dallay/corvus-skills` GitHub repository creation (infrastructure, not runtime code)
- CI pipelines for the skills repo (validate.yml, publish.yml)
- Full Agent Skills standard validation (Phase 3)
- Tool sandboxing / capability restrictions beyond `allowed-tools` (Phase 3)
- `corvus skills migrate` command for SKILL.toml-to-SKILL.md conversion (Phase 3)
- Removal of SKILL.toml support (Phase 3 — Phase 2 only warns)

## Approach

### Task Group 1: Catalog Index Data Model

Define the `index.toml` schema and corresponding Rust types:

```rust
/// Top-level catalog parsed from index.toml
struct CatalogIndex {
    meta: CatalogMeta,
    skills: BTreeMap<String, CatalogEntry>,
}

struct CatalogMeta {
    version: u32,           // Schema version, must be 1
    generated_at: String,   // ISO 8601
    commit: String,         // Git SHA of the skills repo at generation time
}

struct CatalogEntry {
    description: String,
    version: String,        // SemVer
    content_hash: String,   // "sha256:<hex>"
    path: String,           // Relative path in skills repo (e.g., "skills/git-expert")
    tags: Vec<String>,
}
```

Parse with the existing `toml` crate. Implement in a new `skills/catalog.rs` module.

### Task Group 2: Embedded Index and Cache

Add `build.rs` that generates a `catalog_index.rs` const from a committed snapshot file
(`src/skills/catalog_index.toml`):

```rust
pub const EMBEDDED_INDEX: &str = include_str!("skills/catalog_index.toml");
```

Runtime index resolution strategy (implemented in `catalog.rs`):

1. Check cached index at `{workspace}/catalog-index-cache.toml` — use if < 24h old.
2. If stale/missing, attempt HTTP GET from GitHub raw content (3s timeout).
3. On success: update cache, use fresh index.
4. On failure: fall back to embedded index.

No new dependencies — use existing `ureq` or `reqwest` if available, otherwise defer network
fetch to git-based mechanisms already in the codebase. The `sha2` and `hex` crates are already
present.

### Task Group 3: Catalog Install and Search

**Bare-name detection heuristic**: If the source string contains no `/`, `\`, `.`, or `:`,
treat as a catalog name lookup.

**Install flow** (`skills install <bare-name>`):

1. Resolve name in catalog index (cached → embedded).
2. Construct official repo URL from hardcoded constant (`dallay/corvus-skills`).
3. Sparse checkout or shallow clone of the skill path.
4. Copy to `{workspace}/skills/<name>/`.
5. Set `SkillSource::Official { repo, path }`.
6. Write lockfile entry with `source = "official:dallay/corvus-skills"`, `trust = "official"`.
7. No trust gate needed (Official trust).

**`lock_entry_to_origin()` update** (`lockfile.rs`): Recognize `"official:"` prefix in the
source field to reconstruct `SkillSource::Official { repo, path }`.

**Search** (`skills search <query>`): Fuzzy-match against name, description, and tags in the
catalog index. Works offline via embedded index fallback.

**List catalog** (`skills list --catalog`): Display all official skills, marking installed ones.

**CLI expansion**:

```rust
enum SkillCommands {
    List,                                             // existing
    Install { source: String, trust: bool },          // existing
    Remove { name: String },                          // existing
    Search { query: String },                         // NEW
    Update { name: Option<String> },                  // NEW
    Lock { cmd: LockCommands },                       // NEW
}

enum LockCommands {
    Repair,
}
```

### Task Group 4: SKILL.toml Deprecation

Extend `frontmatter.rs` to parse three new optional fields: `version: String`,
`author: String`, `tags: Vec<String>`. These are simple scalars/lists the existing hand-rolled
parser can handle without adding `serde_yaml`.

In `load_skill_toml()`, emit deprecation warning:
`"SKILL.toml is deprecated. Migrate to SKILL.md with YAML frontmatter."`

Update `skillforge/integrate.rs` to generate only SKILL.md with frontmatter (drop SKILL.toml
generation).

### Task Group 5: SkillForge Trust Boundaries

Replace the auto-integrate pipeline with an explicit `corvus skills discover` command:

1. Runs Scout → Evaluate pipeline (existing code).
2. Displays results in a table (name, URL, score) without writing to disk.
3. User explicitly installs via `corvus skills install <url>` (standard ThirdParty flow).

Deprecate `auto_integrate` config option. When set, emit warning and ignore.

### Task Group 6: Lockfile Repair

`corvus skills lock repair`:

1. Scan `{workspace}/skills/` for directories with SKILL.md (or SKILL.toml).
2. Read existing lockfile (tolerate corruption — start empty if corrupt).
3. For each skill on disk: verify existing entry or create new one (default Local).
4. Remove orphaned entries (in lockfile but not on disk).
5. Recompute content hashes for mismatched entries.
6. Write repaired lockfile and print summary.

### Task Group 7: Skills Update

`corvus skills update [name]`:

- **Official skills**: Check catalog index for newer version (compare content_hash or version).
  If newer, re-fetch from official repo, update lockfile entry.
- **Third-party (GitRepo)**: Re-clone from source URL, update content_hash and pinned_ref.
- **Local/LinkedLocal**: Skip with message ("local skills are managed manually").
- **Update all**: When no name given, iterate all installed skills.

### Recommended Implementation Order

Following the exploration's phased sub-release recommendation:

**Phase 2a (Foundation)**: Task Groups 1, 2, 4 — No external infra needed. Unblocks everything.

**Phase 2b (Catalog UX)**: Task Groups 3, 7 — Requires the embedded index snapshot (from 2a).

**Phase 2c (Cleanup)**: Task Groups 5, 6 — Independent cleanup, can ship any time after 2a.

## Affected Areas

| Area                                                  | Impact   | Description                                                          |
|-------------------------------------------------------|----------|----------------------------------------------------------------------|
| `clients/agent-runtime/src/skills/catalog.rs`         | New      | Catalog index types, parsing, cache, search                          |
| `clients/agent-runtime/src/skills/mod.rs`             | Modified | Catalog install path, bare-name detection, `SkillCommands` expansion |
| `clients/agent-runtime/src/skills/trust.rs`           | Modified | Official source detection in `lock_entry_to_origin()`                |
| `clients/agent-runtime/src/skills/lockfile.rs`        | Modified | Official variant in `lock_entry_to_origin()`, repair command         |
| `clients/agent-runtime/src/skills/frontmatter.rs`     | Modified | Parse `version`, `author`, `tags` fields                             |
| `clients/agent-runtime/src/skillforge/mod.rs`         | Modified | Discover command, deprecate auto-integrate                           |
| `clients/agent-runtime/src/skillforge/integrate.rs`   | Modified | Stop generating SKILL.toml                                           |
| `clients/agent-runtime/src/lib.rs`                    | Modified | New CLI subcommands (search, update, lock repair, discover)          |
| `clients/agent-runtime/src/config/schema.rs`          | Modified | `SkillsConfig` expansion (catalog_url, cache settings)               |
| `clients/agent-runtime/build.rs`                      | New      | Embedded index const generation                                      |
| `clients/agent-runtime/src/skills/catalog_index.toml` | New      | Committed index snapshot (initially empty/minimal)                   |
| `clients/agent-runtime/Cargo.toml`                    | Modified | build.rs declaration (no new deps expected)                          |

## Risks

| Risk                                                     | Likelihood | Mitigation                                                                                                                     |
|----------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------------------------------------|
| Embedded index becomes stale if manual sync is forgotten | Medium     | CI workflow to auto-open PR when skills repo changes; embedded index is floor not ceiling — runtime fetches latest when online |
| Sparse git checkout fails on older git versions          | Low        | Fall back to shallow clone of entire repo; only copy target skill directory                                                    |
| Catalog name squatting in official repo                  | Low        | Review process for official repo; reserved namespace for core skills                                                           |
| build.rs adds build-time complexity                      | Low        | Keep minimal — just const generation from committed file, no network, no complex logic                                         |
| SKILL.toml deprecation warnings disrupt existing users   | Low        | Warnings are informational only; SKILL.toml continues to work throughout Phase 2                                               |
| SkillForge discover command breaks existing workflows    | Low        | Auto-integrate was already disabled by default; deprecation warning before removal                                             |
| Bare-name heuristic misclassifies unusual paths          | Low        | Heuristic is conservative (any `/`, `\`, `.`, `:` triggers URL/path mode); document edge cases                                 |

## Rollback Plan

Each task group is independently revertible:

1. **Catalog types** (`catalog.rs`): Delete the module. No runtime code depends on it until
   install path is wired.
2. **Embedded index** (`build.rs`): Remove `build.rs` and the snapshot file. The const is only
   referenced by catalog code.
3. **Catalog install/search**: Revert `SkillCommands` changes and `mod.rs` install logic.
   Bare-name installs would error with "not a valid URL or path" (pre-existing behavior).
4. **SKILL.toml deprecation**: Remove warning log line and revert frontmatter field additions.
   No breaking change — SKILL.toml still works.
5. **SkillForge boundaries**: Re-enable auto-integrate path. Existing config still honored.
6. **Lockfile repair**: Remove subcommand. Lockfile continues to function without repair tool.
7. **Skills update**: Remove subcommand. Users can `remove` + `install` manually.

Full rollback: revert the PR(s). No database migrations, no schema changes, no external service
dependencies.

## Dependencies

- Phase 1 trust model (shipped) — `SkillTrust`, `SkillSource`, `SkillOrigin`, lockfile, frontmatter
  parser, trust-aware prompt rendering, tool gating, install trust gate.
- `toml` crate (already in dependency tree).
- `sha2` + `hex` crates (already in dependency tree).
- Git CLI available at runtime (existing requirement for skill install).

## Success Criteria

- [ ] `CatalogIndex` and `CatalogEntry` types parse a well-formed `index.toml` correctly
- [ ] `build.rs` embeds the committed snapshot; `EMBEDDED_INDEX` const is available at runtime
- [ ] Cached index refreshes when stale and falls back to embedded when offline
- [ ] `corvus skills install git-expert` resolves from catalog, sets `SkillSource::Official`
- [ ] `corvus skills search git` returns matching entries from catalog (works offline)
- [ ] `corvus skills list --catalog` shows all official skills with install status
- [ ] `lock_entry_to_origin()` reconstructs `SkillSource::Official` from `"official:"` prefix
- [ ] Official skills installed via catalog get `Official` trust without trust gate prompt
- [ ] Installing official repo URL directly (`skills install https://...`) still produces
  `ThirdParty` — privilege escalation prevented
- [ ] Frontmatter parser handles `version`, `author`, `tags` fields
- [ ] Loading SKILL.toml emits deprecation warning
- [ ] SkillForge integrator generates only SKILL.md (no SKILL.toml)
- [ ] `corvus skills discover` shows results without auto-installing
- [ ] Discovered skills install as `ThirdParty` through standard trust gate
- [ ] `corvus skills lock repair` rebuilds lockfile from disk state correctly
- [ ] `corvus skills update` updates official skills from catalog and third-party from source URL
- [ ] All existing Phase 1 tests continue to pass
- [ ] No new dependencies added to `Cargo.toml`

## Follow-Up (Phase 3)

- `corvus skills migrate` command for automated SKILL.toml → SKILL.md conversion
- Remove SKILL.toml support entirely
- Full Agent Skills standard validation (schema enforcement on install)
- Tool sandboxing / capability model beyond `allowed-tools` filtering
- Remove SkillForge auto-integrate code path entirely
- Skills signing (cryptographic verification of official skills)
