# Product Contract: Official Catalog Model

## Overview

The official catalog is a curated collection of skills maintained by the Corvus project. Catalog
skills carry the **Official** trust tier — they are reviewed, scanned, and distributed through a
controlled pipeline. The catalog is the default source for bare-name skill installs and the
recommended way to distribute high-quality skills to all Corvus users.

## Repository

The catalog lives at **`dallay/corvus-skills`** on GitHub. This repository contains:

- Individual skill directories (each with a `SKILL.md`)
- A generated `index.toml` that serves as the catalog index
- Contribution guidelines and review process documentation

## Index Format

The catalog index is a TOML file with the following structure:

```toml
[meta]
version = 1
generated_at = "2026-03-20T14:30:00Z"
commit = "a1b2c3d4e5f6"

[skills.kotlin-expert]
name = "kotlin-expert"
description = "Advanced Kotlin patterns and conventions"
version = "1.0.0"
path = "skills/kotlin-expert"
content_hash = "sha256:a1b2c3..."
author = "corvus-team"
tags = ["kotlin", "patterns"]

[skills.docker-expert]
name = "docker-expert"
description = "Docker containerization best practices"
version = "1.0.0"
path = "skills/docker-expert"
content_hash = "sha256:d4e5f6..."
author = "corvus-team"
tags = ["docker", "devops"]
```

**Entry fields:**

| Field          | Description                                 |
|----------------|---------------------------------------------|
| `name`         | Skill identifier (matches directory name)   |
| `description`  | One-line description                        |
| `version`      | Informational version string                |
| `path`         | Relative path within the catalog repository |
| `content_hash` | SHA-256 hash of skill content               |
| `author`       | Skill author or team                        |
| `tags`         | List of tags for search and categorization  |

The `[meta]` section includes the schema version (must be `1`), generation timestamp, and source
commit.

## Distribution: Embedded + Cache

### Embedded index

A snapshot of `index.toml` is compiled into the Corvus binary at build time using `build.rs` and
`include_str!`. This ensures every Corvus installation has a baseline catalog available even without
network access.

### Cache

On first use (or when the cache expires), Corvus fetches the latest `index.toml` from the catalog
repository and stores it at:

```
{workspace}/.catalog-cache/index.toml
```

The cache has a **24-hour TTL** (configurable via `skills.catalog_cache_ttl_hours`). After expiry,
the runtime attempts a fresh fetch with a 3-second timeout.

### Fallback chain

Resolution order on catalog access:

1. **Cache** — if present and within TTL
2. **Network fetch** — if cache is stale or missing (3-second timeout)
3. **Embedded index** — if both cache and network are unavailable

This ensures catalog operations always succeed, even offline.

## Installation

```bash
corvus skills install kotlin-expert
```

Bare-name installs resolve against the catalog:

1. Name looked up in the catalog index
2. If found: skill content fetched from the catalog repository at the specified path and commit
3. Trust tier set to **Official**
4. No trust gate required — Official skills are pre-reviewed
5. Lockfile entry written with `trust = "Official"` and `source = "catalog:<name>"`

If the name is not found in the catalog, the installer shows a helpful error with suggestions based
on fuzzy matching.

## Trust & Security

- Official skills receive the **Official** trust tier
- No trust gate is required for installation (the review process is the gate)
- No scanner check is required on install (scanner score = 0 is a review requirement)
- Official skills are not sandboxed — they have full access like Local skills
- Integrity verification runs on every load (hash compared to lockfile entry)

## Review Standard

Every skill submitted to the catalog must pass a 5-point review:

1. **Valid SKILL.md** — file exists with complete YAML frontmatter (`name` and `description`
   required)
2. **Name validation** — name matches `[a-z0-9-]`, 1-64 characters, no consecutive hyphens
3. **Scanner clean** — injection scanner score of 0 (no patterns detected)
4. **Tools declared** — if the skill uses tools, `allowed-tools` must be explicitly listed in
   frontmatter
5. **Maintainer review** — at least one project maintainer reviews the skill instructions via PR

Submissions that fail any check are rejected with feedback. The review process is PR-based in the
`dallay/corvus-skills` repository.

## User Experience

### Browse the catalog

```bash
corvus skills list --catalog
```

Shows all official skills with name, description, version, and tags. Installed skills are marked
with `[installed]`.

### Search the catalog

```bash
corvus skills search kotlin
```

Case-insensitive substring match against name, description, and tags. Official catalog results
appear first, before any third-party discovery results.

### Prompt rendering

Official skills are sorted first in the agent prompt, before Local and ThirdParty skills. They carry
no caution note (unlike ThirdParty skills).

## Versioning

Catalog skills use **content-hash versioning**. When a skill's content changes, its `content_hash`
in the index changes. The `version` field is informational only — there is no semantic versioning
enforcement in the current phase.

`corvus skills update` detects new versions by comparing the installed `content_hash` against the
catalog's current hash.

## Limitations & Future Work

- **No CI pipeline for index regeneration.** The `index.toml` is currently generated manually. A
  GitHub Action in `dallay/corvus-skills` to auto-regenerate the index on push is planned.
- **No contribution guide yet.** The `dallay/corvus-skills` repository needs a `CONTRIBUTING.md`
  documenting the submission workflow, required fields, and testing expectations.
- **No deprecation policy.** When a skill is removed from the catalog, there is no defined behavior
  for users who have it installed. The lockfile entry persists, and the skill continues to work but
  cannot be updated.
- **No semantic versioning.** Content-hash versioning is sufficient for the current phase. SemVer
  may be introduced when the catalog grows and breaking changes become a concern.
