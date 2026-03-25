# Delta for Skills Trust — Official Skills Catalog

## ADDED Requirements

### R7: Catalog Index Format

The system MUST define a catalog index format for describing available official skills.

#### R7.1: CatalogIndex Structure

The catalog index MUST be a valid TOML document containing:

- A `[meta]` table with:
  - `version` — Integer schema version. MUST be `1` for this specification.
  - `generated_at` — ISO 8601 timestamp of index generation.
  - `commit` — Git SHA of the skills repository at generation time.
- A `[skills.<name>]` table for each official skill (see R7.2).

The `meta.version` field MUST be checked on parse. If the version is not recognized, the parser
MUST reject the index with a clear error message.

#### R7.2: CatalogEntry Structure

Each `[skills.<name>]` entry MUST contain:

- `description` — Human-readable summary of the skill's purpose.
- `version` — SemVer string (e.g., `"1.0.0"`).
- `content_hash` — `"sha256:<hex-digest>"` of the skill's SKILL.md content.
- `path` — Relative path within the skills repository (e.g., `"skills/git-expert"`).
- `tags` — Array of lowercase tag strings for categorization and search.

Each entry MAY contain:

- `author` — Author name or identifier.

#### R7.3: Parseability

The index MUST be parseable using the `toml` crate without network access. The index MUST NOT
reference external resources that require fetching to complete parsing.

##### Scenario: Valid catalog index parsed successfully

- GIVEN a TOML file with `[meta]` containing `version = 1`, `generated_at`, and `commit`
- AND a `[skills.git-expert]` entry with `description`, `version`, `content_hash`, `path`, and `tags`
- WHEN the index is parsed
- THEN a `CatalogIndex` MUST be returned with the `git-expert` entry accessible by name
- AND `meta.version` MUST equal `1`

##### Scenario: Index with unknown schema version rejected

- GIVEN a TOML file with `[meta]` containing `version = 99`
- WHEN the index is parsed
- THEN parsing MUST fail with an error message indicating unsupported schema version
- AND the error MUST include the version number found

##### Scenario: Index with missing required field rejected

- GIVEN a TOML file with a `[skills.broken-skill]` entry missing the `content_hash` field
- WHEN the index is parsed
- THEN parsing MUST fail with an error identifying the missing field and skill name

---

### R8: Embedded Index

The agent-runtime binary MUST embed a catalog index snapshot at build time to enable offline
catalog operations on first run.

#### R8.1: Build-Time Embedding

A `build.rs` script MUST embed the content of a committed `catalog_index.toml` file as a
compile-time constant via `include_str!`. The embedded content MUST be a valid catalog index
(see R7).

#### R8.2: Lazy Parsing

The runtime MUST parse the embedded index lazily on first use. The index MUST NOT be parsed at
binary startup if no catalog operation is requested.

#### R8.3: Cached Index with TTL

The runtime SHOULD cache a refreshed index at `{workspace}/catalog-index-cache.toml`. The cache
TTL MUST default to 24 hours and SHOULD be configurable via `skills.catalog_cache_ttl_hours` in
the configuration.

The index resolution strategy MUST be:

1. Check cached index — use if younger than TTL.
2. If stale or missing, attempt HTTP fetch from the official repository raw content (timeout: 3
   seconds).
3. On success: update cache file, use fresh index.
4. On failure: fall back to embedded index.

#### R8.4: Fallback Chain

When the cached index is unavailable or stale and network fetch fails, the runtime MUST fall back
to the embedded index. When both the cached index and the embedded index are unparseable (e.g.,
corrupted cache and a build defect), catalog operations MUST fail gracefully with a clear error
message. The runtime MUST NOT panic or crash.

##### Scenario: First run with no cache uses embedded index

- GIVEN a fresh installation with no `catalog-index-cache.toml` on disk
- AND the network is unavailable
- WHEN the user runs `skills search git`
- THEN the search MUST execute against the embedded index
- AND results from the embedded index MUST be displayed

##### Scenario: Cached index used when fresh

- GIVEN a `catalog-index-cache.toml` exists and was written 2 hours ago
- AND the configured TTL is 24 hours
- WHEN the user runs `skills list --catalog`
- THEN the cached index MUST be used
- AND no network request SHALL be made

##### Scenario: Stale cache triggers refresh

- GIVEN a `catalog-index-cache.toml` exists but was written 25 hours ago
- AND the network is available
- WHEN the user runs `skills search rust`
- THEN the runtime MUST attempt to fetch a fresh index from the network
- AND on success, the cache file MUST be updated with the fresh index
- AND the fresh index MUST be used for the search

##### Scenario: Network failure falls back to embedded

- GIVEN a `catalog-index-cache.toml` does not exist or is stale
- AND the network fetch times out after 3 seconds
- WHEN the user runs `skills install git-expert`
- THEN the embedded index MUST be used for catalog resolution
- AND the install MUST proceed if `git-expert` exists in the embedded index

##### Scenario: Both cache and embedded corrupted

- GIVEN the `catalog-index-cache.toml` contains invalid TOML
- AND the embedded index is also unparseable (build defect)
- WHEN the user runs `skills search git`
- THEN the operation MUST fail with a clear error message indicating no catalog index is available
- AND the runtime MUST NOT panic

---

### R9: Catalog Install Path

Bare-name skill installation MUST resolve against the official catalog and produce `Official`
trust.

#### R9.1: Bare-Name Detection

When the source argument to `skills install` contains none of the characters `/`, `\`, `.`, or
`:`, the runtime MUST treat it as a catalog name lookup. Any source containing these characters
MUST be handled by the existing URL or path install flows.

#### R9.2: Catalog Resolution

A bare-name install MUST resolve the name against the catalog index (using the resolution strategy
from R8.3). The resolved entry MUST provide the repository URL and path for cloning.

#### R9.3: Official Source and Trust

A skill installed via catalog resolution MUST be assigned `SkillSource::Official { repo, path }`
where `repo` is the hardcoded official repository identifier (e.g., `"dallay/corvus-skills"`) and
`path` is the entry's path from the catalog index. The derived trust MUST be `Official` per R1.2.

Official skills installed via the catalog MUST NOT require the `--trust` flag or interactive
confirmation (no trust gate).

#### R9.4: Catalog Miss

When a bare name is not found in the catalog index, the install MUST fail with a clear error
message. The error SHOULD suggest:

- Checking the spelling.
- Running `skills search <query>` to find similar skills.
- Using a full URL to install from a third-party source.

#### R9.5: Skills Search Command

`skills search <query>` MUST fuzzy-match the query against catalog entry names, descriptions, and
tags. Results MUST display the skill name, version, and description. The search MUST work offline
using the embedded index fallback (per R8.3).

#### R9.6: Skills List Catalog

`skills list --catalog` MUST display all official skills available in the catalog index. Installed
skills MUST be visually distinguished (e.g., marked `[installed]`). This command MUST work offline
using the embedded index fallback.

##### Scenario: Install by bare name from catalog

- GIVEN the catalog index contains an entry for `git-expert` with path `"skills/git-expert"`
- WHEN the user runs `skills install git-expert`
- THEN the skill MUST be cloned from the official repository at path `skills/git-expert`
- AND `skill.origin.source` MUST be `Official { repo: "dallay/corvus-skills", path: "skills/git-expert" }`
- AND `skill.trust` MUST be `Official`
- AND no trust gate prompt SHALL be displayed
- AND the lockfile entry MUST contain `source = "official:dallay/corvus-skills"` and `trust = "official"`

##### Scenario: Install bare name not in catalog

- GIVEN the catalog index does not contain an entry for `nonexistent-skill`
- WHEN the user runs `skills install nonexistent-skill`
- THEN the install MUST fail with an error message indicating the skill was not found in the catalog
- AND the error MUST suggest using `skills search` or installing via URL

##### Scenario: Bare-name detection distinguishes catalog from URL/path

- GIVEN a source argument `https://github.com/user/skill`
- WHEN `skills install https://github.com/user/skill` is executed
- THEN the source MUST be treated as a URL (contains `/` and `:`)
- AND the catalog MUST NOT be consulted

##### Scenario: Search with partial match

- GIVEN the catalog contains `git-expert`, `git-flow`, and `rust-expert`
- WHEN the user runs `skills search git`
- THEN the results MUST include `git-expert` and `git-flow`
- AND the results MUST NOT include `rust-expert`
- AND each result MUST display name, version, and description

##### Scenario: Search works offline

- GIVEN no `catalog-index-cache.toml` exists
- AND the network is unavailable
- WHEN the user runs `skills search docker`
- THEN the search MUST execute against the embedded index
- AND matching results MUST be displayed

##### Scenario: List catalog shows install status

- GIVEN the catalog contains 5 skills
- AND `git-expert` and `rust-expert` are installed locally
- WHEN the user runs `skills list --catalog`
- THEN all 5 catalog skills MUST be listed
- AND `git-expert` and `rust-expert` MUST be marked as installed
- AND the remaining 3 MUST NOT be marked as installed

##### Scenario: Privilege escalation via URL to official repo prevented

- GIVEN a user runs `skills install https://github.com/dallay/corvus-skills`
- WHEN the install resolves trust
- THEN `skill.origin.source` MUST be `GitRepo { url: "https://github.com/dallay/corvus-skills" }`
- AND `skill.trust` MUST be `ThirdParty`
- AND the `Official` trust tier MUST NOT be granted via URL-based install

---

### R10: SKILL.toml Deprecation

SKILL.toml is deprecated in favor of SKILL.md with YAML frontmatter. Phase 2 warns but continues
to support SKILL.toml.

#### R10.1: Extended Frontmatter Fields

The frontmatter parser MUST support three additional optional fields:

- `version` — SemVer string.
- `author` — Author name or identifier string.
- `tags` — List of tag strings.

These fields MUST be parsed alongside the existing `name`, `description`, and `allowed-tools`
fields.

#### R10.2: Deprecation Warning on SKILL.toml Load

When the runtime loads a skill from a `SKILL.toml` file, it MUST emit a deprecation warning. The
warning MUST contain the text `"SKILL.toml is deprecated"` and SHOULD suggest migrating to
SKILL.md with YAML frontmatter.

#### R10.3: SkillForge Integrator Output

The SkillForge integrator MUST NOT generate `SKILL.toml` files. It MUST generate only SKILL.md
files with YAML frontmatter containing all relevant metadata fields.

#### R10.4: Continued SKILL.toml Support

SKILL.toml loading MUST continue to function correctly throughout Phase 2. The deprecation is
warning-only; removal is deferred to Phase 3.

##### Scenario: Frontmatter with new fields parsed correctly

- GIVEN a SKILL.md with frontmatter containing `version: 1.2.0`, `author: "Jane Doe"`, and `tags: [git, vcs]`
- WHEN the frontmatter is parsed
- THEN `version` MUST be `Some("1.2.0")`
- AND `author` MUST be `Some("Jane Doe")`
- AND `tags` MUST be `Some(["git", "vcs"])`

##### Scenario: Missing new fields default to None

- GIVEN a SKILL.md with frontmatter containing only `name` and `description` (no `version`, `author`, or `tags`)
- WHEN the frontmatter is parsed
- THEN `version` MUST be `None`
- AND `author` MUST be `None`
- AND `tags` MUST be `None`
- AND parsing MUST succeed without errors

##### Scenario: SKILL.toml loaded with deprecation warning

- GIVEN a skill directory contains `SKILL.toml` but no `SKILL.md`
- WHEN the skill is loaded
- THEN the skill MUST load successfully from SKILL.toml
- AND a deprecation warning MUST be emitted containing `"SKILL.toml is deprecated"`

##### Scenario: SkillForge generates only SKILL.md

- GIVEN SkillForge discovers a skill from GitHub
- WHEN the integrator generates output files
- THEN a SKILL.md file MUST be generated with YAML frontmatter
- AND no SKILL.toml file SHALL be generated

---

### R11: SkillForge Trust Boundaries

SkillForge discovery MUST be an explicit, read-only operation. Auto-integration is deprecated.

#### R11.1: Discover Command

`skills discover` MUST run the Scout and Evaluate pipeline and display results as candidates. It
MUST NOT write skills to disk or modify the lockfile.

#### R11.2: No Auto-Installation

Discovered skills MUST be displayed in a results table (name, URL, score) without automatic
installation. The user MUST explicitly install a discovered skill via `skills install <url>`.

#### R11.3: ThirdParty Trust for Discovered Skills

When a user installs a skill found via `skills discover`, the install MUST go through the standard
ThirdParty trust-gated flow (per R6.2). All discovered skills MUST be tagged as `ThirdParty`
regardless of their source or score.

#### R11.4: auto_integrate Deprecation

The `auto_integrate` configuration option MUST be deprecated. When the option is set, the runtime
MUST emit a deprecation warning and MUST ignore the setting. Auto-integration MUST NOT execute.

##### Scenario: Discover shows results without installing

- GIVEN the user runs `skills discover rust`
- AND the Scout pipeline finds 3 matching skills on GitHub
- WHEN the results are displayed
- THEN the results MUST show name, URL, and evaluation score for each skill
- AND no skill directories SHALL be created on disk
- AND the lockfile MUST NOT be modified

##### Scenario: Installing a discovered skill goes through trust gate

- GIVEN the user runs `skills discover testing` and sees `https://github.com/user/test-helper`
- WHEN the user runs `skills install https://github.com/user/test-helper`
- AND the skill declares `allowed-tools: [Bash]`
- THEN the install MUST trigger the ThirdParty trust gate (per R6.2)
- AND `skill.trust` MUST be `ThirdParty`

##### Scenario: auto_integrate config option ignored with warning

- GIVEN the configuration contains `skillforge.auto_integrate = true`
- WHEN the runtime starts
- THEN a deprecation warning MUST be emitted for `auto_integrate`
- AND no auto-integration pipeline SHALL execute

---

### R12: Lockfile Repair

`skills lock repair` MUST rebuild the lockfile from the actual state of skills on disk.

#### R12.1: Disk Scan

The repair command MUST scan the `{workspace}/skills/` directory for subdirectories containing
`SKILL.md` or `SKILL.toml`.

#### R12.2: Rebuild Missing Entries

For each skill found on disk that has no corresponding lockfile entry, the repair MUST create a
new entry with `trust = "local"`, `source = "local"`, a freshly computed `content_hash`, and the
current timestamp as `installed_at`.

#### R12.3: Remove Orphaned Entries

For each lockfile entry whose corresponding skill directory no longer exists on disk, the repair
MUST remove the entry from the lockfile.

#### R12.4: Recompute Content Hashes

For each skill on disk whose lockfile `content_hash` does not match the current SHA-256 digest of
the SKILL.md file, the repair MUST update the `content_hash` to match the current file content.

#### R12.5: Summary Report

After repair, the command MUST report a summary including counts of:

- Entries verified (unchanged).
- Entries added (new skills on disk).
- Entries removed (orphaned).
- Entries updated (hash mismatch).

#### R12.6: Corrupt Lockfile Tolerance

If the existing lockfile is corrupt or unparseable, the repair MUST start from an empty state and
rebuild all entries from disk. The repair MUST NOT fail due to a corrupt lockfile.

##### Scenario: Repair adds missing entries

- GIVEN `{workspace}/skills/` contains `my-notes/SKILL.md` and `quick-fix/SKILL.md`
- AND the lockfile has no entries for `my-notes` or `quick-fix`
- WHEN the user runs `skills lock repair`
- THEN the lockfile MUST contain entries for both `my-notes` and `quick-fix`
- AND both entries MUST have `trust = "local"` and `source = "local"`
- AND the summary MUST report 2 entries added

##### Scenario: Repair removes orphaned entries

- GIVEN the lockfile contains an entry for `deleted-skill`
- AND no directory `{workspace}/skills/deleted-skill/` exists
- WHEN the user runs `skills lock repair`
- THEN the `deleted-skill` entry MUST be removed from the lockfile
- AND the summary MUST report 1 entry removed

##### Scenario: Repair updates mismatched hash

- GIVEN the lockfile contains an entry for `docker-expert` with `content_hash = "sha256:aaa..."`
- AND the actual SHA-256 of `docker-expert/SKILL.md` is `"sha256:bbb..."`
- WHEN the user runs `skills lock repair`
- THEN the lockfile entry for `docker-expert` MUST be updated with `content_hash = "sha256:bbb..."`
- AND the summary MUST report 1 entry updated

##### Scenario: Repair with corrupt lockfile rebuilds from scratch

- GIVEN `{workspace}/skills.lock` contains invalid TOML
- AND `{workspace}/skills/` contains 3 skill directories
- WHEN the user runs `skills lock repair`
- THEN the lockfile MUST be rebuilt from scratch
- AND it MUST contain 3 entries, all with `trust = "local"`
- AND the summary MUST report 3 entries added

##### Scenario: Repair with empty skills directory

- GIVEN `{workspace}/skills/` is empty (no subdirectories)
- AND the lockfile contains 2 entries
- WHEN the user runs `skills lock repair`
- THEN the lockfile MUST be written with zero entries
- AND the summary MUST report 2 entries removed

---

### R13: Skills Update

`skills update` MUST update installed skills from their upstream source.

#### R13.1: Update All

`skills update` (with no name argument) SHOULD update all installed skills according to their
source type.

#### R13.2: Update by Name

`skills update <name>` MUST update only the named skill. If the skill is not installed, the
command MUST fail with a clear error.

#### R13.3: Official Skill Update

For skills with `SkillSource::Official`, the update MUST:

1. Resolve the current catalog index (per R8.3).
2. Compare the catalog entry's `content_hash` with the installed lockfile entry's `content_hash`.
3. If they differ, re-fetch the skill from the official repository.
4. Update the lockfile entry with the new `content_hash`, `ref`, and `installed_at` timestamp.

If the hashes match, the skill MUST be reported as up-to-date.

#### R13.4: Third-Party Skill Update

For skills with `SkillSource::GitRepo`, the update MUST re-fetch from the source URL recorded in
the lockfile. The lockfile entry MUST be updated with the new `content_hash`, `ref` (if
applicable), and `installed_at` timestamp.

#### R13.5: Local Skill Skip

For skills with `SkillSource::Local` or `SkillSource::LinkedLocal`, the update MUST skip the
skill with an informational message indicating local skills are managed manually.

#### R13.6: Lock Entry Update

After a successful update, the lockfile entry for the skill MUST be updated with the new
`content_hash`, `ref` (if applicable), and `installed_at` timestamp. The `trust` and `source`
fields MUST NOT change during an update.

##### Scenario: Update official skill with newer version available

- GIVEN `git-expert` is installed with `content_hash = "sha256:old..."`
- AND the catalog index contains `git-expert` with `content_hash = "sha256:new..."`
- WHEN the user runs `skills update git-expert`
- THEN the skill MUST be re-fetched from the official repository
- AND the lockfile entry MUST be updated with `content_hash = "sha256:new..."`
- AND the lockfile `installed_at` MUST be updated to the current timestamp

##### Scenario: Update official skill already up-to-date

- GIVEN `git-expert` is installed with `content_hash = "sha256:abc..."`
- AND the catalog index contains `git-expert` with `content_hash = "sha256:abc..."`
- WHEN the user runs `skills update git-expert`
- THEN no re-fetch SHALL occur
- AND the skill MUST be reported as up-to-date

##### Scenario: Update third-party skill

- GIVEN `community-tool` is installed with `source = "https://github.com/user/community-tool"`
- WHEN the user runs `skills update community-tool`
- THEN the skill MUST be re-fetched from `https://github.com/user/community-tool`
- AND the lockfile entry MUST be updated with new `content_hash` and `installed_at`

##### Scenario: Update skips local skill

- GIVEN `my-notes` is installed with `source = "local"`
- WHEN the user runs `skills update my-notes`
- THEN the update MUST be skipped
- AND an informational message MUST indicate that local skills are managed manually

##### Scenario: Update all skills

- GIVEN 3 skills are installed: `git-expert` (Official), `community-tool` (ThirdParty), `my-notes` (Local)
- WHEN the user runs `skills update` (no name argument)
- THEN `git-expert` MUST be checked against the catalog for updates
- AND `community-tool` MUST be re-fetched from its source URL
- AND `my-notes` MUST be skipped with an informational message

##### Scenario: Update when offline

- GIVEN `git-expert` (Official) is installed
- AND the network is unavailable
- AND the embedded index has the same `content_hash` as the installed version
- WHEN the user runs `skills update git-expert`
- THEN the update MUST report the skill as up-to-date (comparing against embedded/cached index)
- AND no error SHALL be raised for the catalog check

##### Scenario: Update nonexistent skill

- GIVEN no skill named `ghost-skill` is installed
- WHEN the user runs `skills update ghost-skill`
- THEN the command MUST fail with a clear error indicating the skill is not installed

---

## MODIFIED Requirements

### R3: Skills Lockfile (Modified — R3.3 and R3.4)

#### R3.3: Write Triggers (Updated)

The lockfile MUST be written (created or updated) when:

- A skill is successfully installed via `skills install`.
- A skill is successfully updated via `skills update`.
- A lockfile repair is performed via `skills lock repair`.

(Previously: only install and update triggered lockfile writes.)

#### R3.6: Official Source in Lock Entries (Added)

The `lock_entry_to_origin()` function MUST recognize the `"official:"` prefix in the lockfile
`source` field. A source value starting with `"official:"` MUST reconstruct
`SkillSource::Official { repo, path }` where `repo` is the string after the prefix and `path`
is the `path` field of the lock entry. Non-"official:" and non-"local" sources MUST continue to
map to `SkillSource::GitRepo { url }`.

##### Scenario: Official lockfile entry reconstructed correctly

- GIVEN a lockfile entry with `source = "official:dallay/corvus-skills"` and `path = "skills/git-expert"`
- WHEN `lock_entry_to_origin()` processes the entry
- THEN the result MUST be `SkillSource::Official { repo: "dallay/corvus-skills", path: "skills/git-expert" }`
- AND the derived trust MUST be `Official`

##### Scenario: Git URL lockfile entry still maps to ThirdParty

- GIVEN a lockfile entry with `source = "https://github.com/user/cool-skill"`
- WHEN `lock_entry_to_origin()` processes the entry
- THEN the result MUST be `SkillSource::GitRepo { url: "https://github.com/user/cool-skill" }`
- AND the derived trust MUST be `ThirdParty`

### R6: Install Flow Trust Gating (Modified — R6.1)

#### R6.1: Trust Resolution at Install (Updated)

The `skills install` command MUST resolve the `SkillTrust` tier from the install source before
proceeding. For bare-name sources (per R9.1), the trust MUST be resolved via catalog lookup
producing `Official` trust. For URL and path sources, the derivation MUST follow the same rules
as R1.2.

(Previously: trust resolution only covered URL and path sources.)

##### Scenario: Official catalog install bypasses trust gate

- GIVEN a user runs `skills install git-expert` (bare name)
- AND `git-expert` exists in the catalog index
- WHEN the install resolves trust
- THEN trust MUST be `Official`
- AND no trust gate prompt SHALL be displayed
- AND the `--trust` flag MUST NOT be required
