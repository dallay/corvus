# Product Contract: Skill Install Lifecycle

## Overview

Every skill in Corvus follows a defined lifecycle: resolution, installation, validation,
registration, loading, and removal. This contract defines each stage, the lockfile as the single
source of truth, and policies for updates, repair, and agent-driven installation.

## Identity

A skill's identity is its **name** — a lowercase alphanumeric string with hyphens, 1-64 characters,
no consecutive hyphens. The name must match both the directory name and the `name` field in SKILL.md
frontmatter.

Names are unique within a workspace. Installing a skill with a name that already exists overwrites
the previous installation.

## Registry: skills.lock

The `skills.lock` file (TOML format) is the single source of truth for all installed skills. It
lives at the workspace root.

Each entry contains:

| Field           | Description                                          |
|-----------------|------------------------------------------------------|
| `trust`         | Trust tier: `Official`, `ThirdParty`, or `Local`     |
| `source`        | Install source: catalog name, git URL, or local path |
| `path`          | Relative path to skill directory within workspace    |
| `ref`           | Git ref (commit SHA) at install time, if applicable  |
| `content_hash`  | SHA-256 hash of skill content at install time        |
| `installed_at`  | ISO 8601 timestamp of installation                   |
| `allowed_tools` | List of tools declared in frontmatter                |

Example entry:

```toml
[skills.my-linter]
trust = "Official"
source = "catalog:my-linter"
path = "skills/my-linter"
ref = "a1b2c3d4"
content_hash = "sha256:e5f6..."
installed_at = "2026-03-20T14:30:00Z"
allowed_tools = ["bash", "read"]
```

## Install Flow

### Source Resolution

The installer resolves the source argument in order:

1. **Bare name** (e.g., `my-skill`) — resolves against the official catalog. If found, installs as
   Official.
2. **URL** (e.g., `https://github.com/user/repo`) — clones the git repository. Installs as
   ThirdParty.
3. **Local path** (e.g., `./path/to/skill`) — symlinks (Unix) or copies the directory. Installs as
   Local.

### Validation Pipeline

After fetching the skill content:

1. **Structure check**: `SKILL.md` must exist
2. **Frontmatter parse**: YAML frontmatter parsed if present (parse errors are warnings)
3. **Name validation**: name must pass `[a-z0-9-]` rules (error on install, warning on load)
4. **Name consistency**: directory name must match frontmatter `name` field
5. **Trust gate**: ThirdParty skills with `allowed-tools` require `--trust` or interactive consent
6. **Scanner check**: content scanned for injection patterns; high scores block install unless
   `--trust`
7. **Lockfile write**: entry added to `skills.lock` with computed content hash

## Update Flow

```bash
corvus skills update [name]
```

- **Official skills**: fetches latest catalog entry, compares `content_hash`. If changed,
  re-downloads and updates lockfile.
- **ThirdParty skills**: re-fetches from source URL, re-validates, updates lockfile. Does not
  re-trigger trust gate (future hardening opportunity).
- **Local skills**: skipped — local skills are managed by the user directly.
- If `name` is omitted, all non-local skills are updated.

## Remove Flow

```bash
corvus skills remove <name>
```

1. Validates the skill exists in the workspace
2. Path safety check: confirms the target is within the skills directory
3. Removes the skill directory (or symlink)
4. Removes the corresponding `skills.lock` entry

## Search and Discovery

### Catalog search

```bash
corvus skills search <query>
```

Case-insensitive substring match against the official catalog's name, description, and tags fields.
Results show install status.

### Third-party discovery

```bash
corvus skills discover [query]
```

Display-only GitHub search for skill repositories. Results are shown but not installed — the user
must explicitly run `corvus skills install <url>` to install. All discovered skills are ThirdParty.

## Repair

```bash
corvus skills lock repair
```

Reconciles the lockfile against the filesystem:

- **Missing entries**: skills on disk without a lockfile entry are added (trust derived from source)
- **Orphaned entries**: lockfile entries without a corresponding directory are removed
- **Hash mismatch**: content hashes are recomputed and updated

## Agent-Driven Install

Agents (LLMs operating within Corvus) may need to install skills to complete tasks. The policy:

- **Official catalog skills**: allowed. Agents may install Official skills by name without user
  confirmation.
- **ThirdParty skills**: never allowed automatically. ThirdParty installs always require human
  approval via `--trust` or interactive consent.
- **Config**: `skills.agent_auto_install` (default: `false`). When set to `true`, agents may install
  Official skills without prompting. When `false`, all agent-initiated installs require user
  confirmation.

This policy ensures agents can seamlessly access curated skills while preventing unauthorized
execution of untrusted code.

## Limitations & Future Work

- **Structured CLI output is deferred.** All commands currently use human-readable text output. A
  `--json` flag for `list`, `search`, and `list --catalog` would enable scripting and CI
  integration. This is planned as a follow-up.
- **No `skills info <name>` command.** Detailed metadata inspection (full frontmatter, lockfile
  entry, trust chain) is not exposed as a dedicated command. `skills list` and lockfile inspection
  cover most needs.
- **No `skills verify` command.** On-demand integrity verification is partially covered by
  `lock repair`. A dedicated verify command is a future consideration.
