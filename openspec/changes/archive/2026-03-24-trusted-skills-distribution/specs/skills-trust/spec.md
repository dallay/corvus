# Skills Trust Specification

## Purpose

Defines the trust model, integrity tracking, and security boundaries for Corvus's skills system.
Skills are categorized into trust tiers derived from their origin, with enforcement rules governing
prompt rendering, tool access, and installation workflows. This specification covers Phase 1:
trust enum, origin tracking, open-skills deprecation, lockfile, trust-aware prompt rendering,
`allowed-tools` parsing, and install flow trust gating.

## Requirements

### R1: Trust Tier Model

#### R1.1: SkillTrust Enum

The system MUST define a `SkillTrust` enum with exactly three variants:

- **Official** — Skills maintained by the Corvus project, sourced from the official repository.
- **Local** — Skills created by the user in their workspace or symlinked from a local path.
- **ThirdParty** — Skills installed from any external git source or discovered via SkillForge.

The enum MUST derive `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, and `Deserialize`.

#### R1.2: Trust Derivation from Origin

The `SkillTrust` value MUST be derived from `SkillOrigin` at load time. Trust MUST NOT be stored
as an independently mutable field. Trust MUST NOT be settable by skill authors or via skill
metadata (SKILL.md frontmatter).

The derivation rules SHALL be:

| SkillSource variant           | Derived SkillTrust |
|-------------------------------|--------------------|
| `Official { repo, path }`     | `Official`         |
| `Local`                       | `Local`            |
| `LinkedLocal { target }`      | `Local`            |
| `GitRepo { url }`             | `ThirdParty`       |
| `Discovered { source, repo }` | `ThirdParty`       |

#### R1.3: SkillOrigin Struct

The `Skill` struct MUST include a `trust: SkillTrust` field and an `origin: SkillOrigin` field.

`SkillOrigin` MUST contain:

- `source: SkillSource` — Where the skill was installed from.
- `installed_at: Option<String>` — ISO 8601 timestamp of install or last update.
- `pinned_ref: Option<String>` — Git commit SHA or tag (for git-sourced skills).
- `content_hash: Option<String>` — SHA-256 hex digest of the SKILL.md file content.

#### R1.4: SkillSource Enum

The system MUST define a `SkillSource` enum with these variants:

- `Official { repo: String, path: String }` — From the official Corvus skills repository.
- `Local` — User-created in the workspace skills directory.
- `LinkedLocal { target: PathBuf }` — Symlinked from a local filesystem path.
- `GitRepo { url: String }` — Cloned from a git repository URL.
- `Discovered { source: String, repo: String }` — Found via SkillForge auto-discovery.

##### Scenario: Trust derived from git-cloned skill

- GIVEN a skill installed via `skills install https://github.com/user/my-skill`
- WHEN the skill is loaded at runtime
- THEN `skill.origin.source` MUST be `GitRepo { url: "https://github.com/user/my-skill" }`
- AND `skill.trust` MUST be `ThirdParty`

##### Scenario: Trust derived from user-created workspace skill

- GIVEN a skill directory created manually at `{workspace}/skills/my-tool/SKILL.md`
- AND no lockfile entry exists for `my-tool`
- WHEN the skill is loaded at runtime
- THEN `skill.origin.source` MUST be `Local`
- AND `skill.trust` MUST be `Local`

##### Scenario: Trust derived from symlinked skill

- GIVEN a skill installed via `skills install --link /home/user/dev/my-skill`
- WHEN the skill is loaded at runtime
- THEN `skill.origin.source` MUST be `LinkedLocal { target: "/home/user/dev/my-skill" }`
- AND `skill.trust` MUST be `Local`

##### Scenario: Privilege escalation prevention

- GIVEN a skill installed from `https://github.com/attacker/fake-official`
- AND the skill's SKILL.md frontmatter contains `trust: official`
- WHEN the skill is loaded at runtime
- THEN `skill.trust` MUST be `ThirdParty` (derived from `GitRepo` source)
- AND the frontmatter `trust` field MUST be ignored

---

### R2: Open-Skills Deprecation

#### R2.1: Default Disabled

The `open_skills_enabled()` function MUST return `false` by default.

#### R2.2: Opt-In Configuration

The system MUST support enabling open-skills via two mechanisms, checked in this order:

1. Config file option: `skills.legacy_open_skills` (boolean)
2. Environment variable: `CORVUS_OPEN_SKILLS` (`"true"` / `"false"`)
3. Default: `false`

If the config file option is set, it MUST take precedence over the environment variable.

#### R2.3: Deprecation Warning

When open-skills is enabled by either mechanism, the system MUST emit a deprecation warning at
startup containing the text: `"open-skills is deprecated and will be removed in a future release"`.

The warning SHOULD suggest using `corvus skills install <url>` as the replacement.

#### R2.4: Open-Skills Trust Tagging

Skills loaded from the open-skills repository path MUST be assigned `ThirdParty` trust with
`SkillSource::GitRepo { url: "https://github.com/besoeasy/open-skills" }`.

##### Scenario: Open-skills disabled by default

- GIVEN a fresh Corvus installation with no configuration overrides
- WHEN `open_skills_enabled()` is called
- THEN it MUST return `false`
- AND no skills from the open-skills repository SHALL be loaded

##### Scenario: Open-skills enabled via environment variable

- GIVEN the environment variable `CORVUS_OPEN_SKILLS` is set to `"true"`
- AND no config file override exists for `skills.legacy_open_skills`
- WHEN the runtime starts
- THEN open-skills MUST be loaded
- AND a deprecation warning MUST be emitted to the log

##### Scenario: Config file overrides environment variable

- GIVEN the environment variable `CORVUS_OPEN_SKILLS` is set to `"true"`
- AND the config file contains `skills.legacy_open_skills = false`
- WHEN `open_skills_enabled()` is called
- THEN it MUST return `false`

##### Scenario: Open-skills tagged as ThirdParty

- GIVEN open-skills is enabled
- WHEN skills are loaded from the open-skills repository path
- THEN each loaded skill MUST have `trust` set to `ThirdParty`
- AND `origin.source` MUST be `GitRepo { url: "https://github.com/besoeasy/open-skills" }`

---

### R3: Skills Lockfile

#### R3.1: Location and Format

The skills lockfile MUST be located at `{workspace}/skills.lock`. The format MUST be TOML.

Each entry MUST be keyed under `[skills.<skill-name>]`.

#### R3.2: Lock Entry Fields

Each lockfile entry MUST contain:

- `trust` — String representation of the trust tier (`"official"`, `"local"`, `"third-party"`).
- `source` — String identifying the skill source (URL for git, `"local"` for local skills).

Each lockfile entry MAY contain:

- `path` — Relative path within a repository (for official repo skills).
- `ref` — Pinned git commit SHA or tag.
- `content_hash` — `"sha256:<hex-digest>"` of the SKILL.md content at install time.
- `installed_at` — ISO 8601 timestamp.
- `allowed_tools` — Array of tool name strings (persisted from frontmatter at install time).

#### R3.3: Write Triggers

The lockfile MUST be written (created or updated) when:

- A skill is successfully installed via `skills install`.
- A skill is successfully updated via `skills update`.

#### R3.4: Missing Lock Entry Defaults

Skills present on disk in the workspace skills directory but absent from the lockfile MUST default
to `Local` trust with `SkillSource::Local`.

#### R3.5: Corrupt Lockfile Handling

If the lockfile cannot be parsed (corrupt, invalid TOML, or I/O error), the system MUST:

- Log a warning about the corrupt lockfile.
- Continue loading skills as if no lockfile exists (all workspace skills default to `Local`).
- NOT block skill loading or runtime startup.

The lockfile SHOULD be treated as advisory, not authoritative for runtime operation.

##### Scenario: Lockfile written on install

- GIVEN a user runs `skills install https://github.com/user/cool-skill`
- WHEN the install completes successfully
- THEN `{workspace}/skills.lock` MUST contain a `[skills.cool-skill]` entry
- AND the entry MUST include `trust = "third-party"`
- AND the entry MUST include `source = "https://github.com/user/cool-skill"`
- AND the entry MUST include a `content_hash` field with a SHA-256 digest
- AND the entry MUST include an `installed_at` timestamp

##### Scenario: Skill on disk without lockfile entry

- GIVEN a skill directory exists at `{workspace}/skills/my-notes/SKILL.md`
- AND there is no `[skills.my-notes]` entry in `skills.lock`
- WHEN the skill is loaded
- THEN `skill.trust` MUST be `Local`
- AND `skill.origin.source` MUST be `Local`

##### Scenario: Corrupt lockfile does not block loading

- GIVEN `{workspace}/skills.lock` contains invalid TOML (e.g., truncated or binary data)
- AND several skill directories exist on disk
- WHEN skills are loaded at runtime
- THEN a warning MUST be logged about the corrupt lockfile
- AND all skills MUST still load successfully
- AND all skills MUST default to `Local` trust

##### Scenario: Lockfile entry with pinned ref

- GIVEN a lockfile entry for `cool-skill` contains `ref = "abc123def456"`
- WHEN the skill is loaded
- THEN `skill.origin.pinned_ref` MUST be `Some("abc123def456")`

---

### R4: Trust-Aware Prompt Rendering

#### R4.1: Trust Attribute on Skill Elements

Each `<skill>` XML element rendered in the `<available_skills>` block MUST include a `trust`
attribute with the value `"official"`, `"local"`, or `"third-party"`.

#### R4.2: Rendering Order

Skills MUST be rendered in the following order within `<available_skills>`:

1. `Official` skills first
2. `Local` skills second
3. `ThirdParty` skills last

Within each trust tier, skills SHOULD be sorted alphabetically by name.

#### R4.3: ThirdParty Caution Note

Each `ThirdParty` skill element MUST include a `<note>` child element containing a caution
message. The note MUST communicate that the skill is from a third-party source and its
instructions have not been reviewed by Corvus maintainers.

#### R4.4: ThirdParty Preamble

When **any** `ThirdParty` skills are present in the rendered skills list, the system MUST include
a preamble before the `<available_skills>` block. The preamble MUST note that some skills are
from third-party sources and have not been reviewed by Corvus maintainers.

When no `ThirdParty` skills are present, the preamble MUST NOT be included.

##### Scenario: Mixed trust tiers rendered in correct order

- GIVEN three skills are loaded: `git-expert` (Official), `my-workflow` (Local), `community-tool` (
  ThirdParty)
- WHEN the skills section is rendered for the agent prompt
- THEN `git-expert` MUST appear before `my-workflow`
- AND `my-workflow` MUST appear before `community-tool`
- AND `git-expert`'s `<skill>` element MUST have `trust="official"`
- AND `my-workflow`'s `<skill>` element MUST have `trust="local"`
- AND `community-tool`'s `<skill>` element MUST have `trust="third-party"`

##### Scenario: ThirdParty skill includes caution note

- GIVEN a skill `community-tool` with `trust` of `ThirdParty`
- WHEN the skill is rendered
- THEN the `<skill>` element MUST contain a `<note>` child element
- AND the note MUST contain text about the skill being from a third-party source

##### Scenario: Preamble included when ThirdParty skills present

- GIVEN the loaded skills include at least one `ThirdParty` skill
- WHEN the `<available_skills>` block is rendered
- THEN a preamble MUST precede the skill list
- AND the preamble MUST mention that some skills have not been reviewed by Corvus maintainers

##### Scenario: No preamble when only Official and Local skills

- GIVEN all loaded skills are `Official` or `Local` (no `ThirdParty`)
- WHEN the `<available_skills>` block is rendered
- THEN no third-party preamble SHALL be included

---

### R5: allowed-tools Parsing

#### R5.1: Frontmatter Parsing

The system MUST parse the `allowed-tools` field from SKILL.md YAML frontmatter. The field MUST
be a YAML list of tool name strings.

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

If the `allowed-tools` field is absent, it MUST be treated as `None` (not as an empty list).
An explicit empty list (`allowed-tools: []`) MUST be treated as "no tools allowed".

#### R5.2: Enforcement by Trust Tier

Tool access MUST be enforced according to the following matrix:

| Trust Tier | `allowed-tools` declared | Behavior                                    |
|------------|--------------------------|---------------------------------------------|
| Official   | Any (present or absent)  | All tools allowed regardless of declaration |
| Local      | Any (present or absent)  | All tools allowed regardless of declaration |
| ThirdParty | Present (non-empty list) | Only declared tools exposed to agent        |
| ThirdParty | Absent or empty          | Instruction-only — no tools exposed         |

#### R5.3: Parse Failure Safe Default

If `allowed-tools` in the SKILL.md frontmatter cannot be parsed (malformed YAML, wrong type),
the system MUST treat it as absent. For `ThirdParty` skills, this results in instruction-only
mode (no tools). The system SHOULD log a warning about the parse failure.

##### Scenario: ThirdParty skill with declared allowed-tools

- GIVEN a ThirdParty skill with SKILL.md frontmatter containing `allowed-tools: [Read, Grep, Glob]`
- AND the skill also defines tools `Read`, `Grep`, `Glob`, and `Bash` in its tool set
- WHEN the skill is activated and tools are built for the agent
- THEN only `Read`, `Grep`, and `Glob` MUST be exposed to the agent
- AND `Bash` MUST NOT be exposed

##### Scenario: ThirdParty skill without allowed-tools declaration

- GIVEN a ThirdParty skill whose SKILL.md frontmatter does not contain an `allowed-tools` field
- AND the skill defines tools `Read` and `Bash`
- WHEN the skill is activated
- THEN no tools MUST be exposed to the agent
- AND the skill MUST function as instruction-only

##### Scenario: Official skill ignores allowed-tools restriction

- GIVEN an Official skill with SKILL.md frontmatter containing `allowed-tools: [Read]`
- AND the skill defines tools `Read`, `Grep`, `Glob`, and `Bash`
- WHEN the skill is activated and tools are built for the agent
- THEN all tools (`Read`, `Grep`, `Glob`, `Bash`) MUST be exposed
- AND the `allowed-tools` declaration MUST NOT restrict tool access

##### Scenario: Local skill ignores allowed-tools restriction

- GIVEN a Local skill with SKILL.md frontmatter containing `allowed-tools: [Read]`
- AND the skill defines tools `Read` and `Bash`
- WHEN the skill is activated
- THEN both `Read` and `Bash` MUST be exposed to the agent

##### Scenario: Malformed allowed-tools defaults to no tools for ThirdParty

- GIVEN a ThirdParty skill with SKILL.md frontmatter containing `allowed-tools: "not-a-list"`
- WHEN the frontmatter is parsed
- THEN `allowed-tools` MUST be treated as absent
- AND a warning MUST be logged about the malformed field
- AND the skill MUST be instruction-only (no tools exposed)

---

### R6: Install Flow Trust Gating

#### R6.1: Trust Resolution at Install

The `skills install` command MUST resolve the `SkillTrust` tier from the install source before
proceeding. The derivation MUST follow the same rules as R1.2.

#### R6.2: ThirdParty Trust Gate for Tool-Declaring Skills

When installing a `ThirdParty` skill that declares `allowed-tools` in its SKILL.md frontmatter
(non-empty list), the install MUST require explicit user consent:

- If the `--trust` CLI flag is provided: proceed without prompting.
- If a TTY is available and `--trust` is not provided: prompt the user interactively, displaying
  the list of declared tools and asking for confirmation.
- If no TTY is available and `--trust` is not provided: abort the install with an error message
  explaining that `--trust` is required for non-interactive installation of third-party skills
  with tools.

ThirdParty skills that do NOT declare `allowed-tools` (instruction-only) MUST install without
the trust gate.

#### R6.3: Install Validation

The install command MUST validate the following before completing:

- `SKILL.md` file exists in the skill directory.
- SKILL.md YAML frontmatter parses successfully.
- The `name` field in frontmatter matches the skill directory name.

If any validation fails, the install MUST abort with a descriptive error message and MUST NOT
write a lock entry.

#### R6.4: Lock Entry on Success

On successful install, the system MUST write a lock entry to `{workspace}/skills.lock` containing
at minimum: `trust`, `source`, `content_hash`, and `installed_at`. The `content_hash` MUST be
the SHA-256 hex digest of the SKILL.md file content, prefixed with `"sha256:"`.

#### R6.5: Content Hash Computation

The system MUST compute SHA-256 over the raw byte content of the SKILL.md file. The hash MUST be
stored as a lowercase hex string prefixed with `"sha256:"`.

##### Scenario: Install third-party skill with tools and --trust flag

- GIVEN a user runs `skills install https://github.com/user/shell-helper --trust`
- AND the skill's SKILL.md declares `allowed-tools: [Bash, Read]`
- WHEN the install resolves trust as `ThirdParty`
- THEN the skill MUST be installed without interactive prompting
- AND a lock entry MUST be written with `trust = "third-party"`
- AND the lock entry MUST include `allowed_tools = ["Bash", "Read"]`

##### Scenario: Install third-party skill with tools without --trust (TTY available)

- GIVEN a user runs `skills install https://github.com/user/shell-helper` (no `--trust`)
- AND a TTY is available
- AND the skill's SKILL.md declares `allowed-tools: [Bash]`
- WHEN the install resolves trust as `ThirdParty`
- THEN the system MUST prompt the user for confirmation
- AND the prompt MUST display the list of tools the skill declares
- AND the install MUST proceed only if the user confirms

##### Scenario: Install third-party skill with tools without --trust (no TTY)

- GIVEN a user runs `skills install https://github.com/user/shell-helper` in a non-interactive
  context
- AND no TTY is available
- AND no `--trust` flag is provided
- AND the skill declares `allowed-tools: [Bash]`
- WHEN the install resolves trust as `ThirdParty`
- THEN the install MUST abort
- AND the error message MUST explain that `--trust` is required

##### Scenario: Install instruction-only third-party skill without gate

- GIVEN a user runs `skills install https://github.com/user/writing-tips`
- AND the skill's SKILL.md does NOT declare `allowed-tools`
- WHEN the install resolves trust as `ThirdParty`
- THEN the skill MUST be installed without requiring `--trust` or interactive confirmation
- AND a lock entry MUST be written with `trust = "third-party"`

##### Scenario: Install fails validation — name mismatch

- GIVEN a user runs `skills install https://github.com/user/cool-skill`
- AND the cloned repository's SKILL.md frontmatter contains `name: different-name`
- AND the skill directory is `cool-skill`
- WHEN validation runs
- THEN the install MUST abort with an error indicating the name mismatch
- AND no lock entry SHALL be written

##### Scenario: Install fails validation — missing SKILL.md

- GIVEN a user runs `skills install https://github.com/user/not-a-skill`
- AND the cloned repository does not contain a SKILL.md file
- WHEN validation runs
- THEN the install MUST abort with an error indicating SKILL.md is missing
- AND no lock entry SHALL be written

##### Scenario: Content hash computed and stored on install

- GIVEN a skill is being installed and SKILL.md contains 1024 bytes of content
- WHEN the install completes successfully
- THEN the lock entry `content_hash` field MUST contain `"sha256:<64-char-hex-digest>"`
- AND the digest MUST be the SHA-256 hash of the exact byte content of SKILL.md
