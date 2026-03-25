# Product Contract: Third-Party Source Policy

## Overview

A third-party skill is any skill installed from an `https://` URL or discovered via external sources (e.g., SkillForge, GitHub search). Third-party skills carry the **ThirdParty** trust tier and are subject to consent requirements, scanner checks, integrity enforcement, and sandboxing.

This contract defines what counts as third-party, how consent works, what security gates apply, and how third-party skills appear in the agent prompt.

## Definition

A skill is classified as **ThirdParty** when:

- It is installed from a git URL (`https://github.com/user/repo`)
- It is discovered via `corvus skills discover`
- It is installed from any source that is not the official catalog or a local path

A skill installed from a URL that happens to point to the official catalog repository (`dallay/corvus-skills`) is **still ThirdParty**. Trust tier is determined by install method, not source URL. Only bare-name installs resolved through the catalog produce Official trust.

## Disabled by Default

Third-party skills are not loaded automatically. There is no "open skills" discovery that silently pulls in external content. Every third-party skill requires explicit user action to install.

HTTP-only URLs (non-HTTPS) are rejected outright.

## Consent Model

### The `--trust` flag

When installing a third-party skill that declares `allowed-tools` in its frontmatter, the user must provide explicit consent:

```bash
# Explicit consent via flag
corvus skills install https://github.com/user/repo --trust

# Interactive consent (TTY only)
corvus skills install https://github.com/user/repo
# → "This skill requests access to tools: [bash, read]. Allow? [y/N]"
```

**Rules:**
- If `--trust` is provided, the skill installs without prompting
- If `--trust` is absent and a TTY is available, an interactive prompt asks for consent
- If `--trust` is absent and no TTY is available (CI, scripts), the install **aborts**
- Skills without `allowed-tools` (instruction-only) install without a trust gate

### Scanner Gate

Before installation completes, the skill content is scanned for injection patterns across 5 categories. The scanner produces a numeric score:

- Score at or below the configured threshold: skill installs normally
- Score above threshold: install is **blocked** unless `--trust` is provided
- `--trust` overrides the scanner gate (the user accepts the risk)

The scan threshold is configurable via `skills.scan_threshold` in workspace config.

## Integrity Enforcement

On every load, the runtime computes a content hash of the skill and compares it to the hash stored in `skills.lock`:

- **Hash matches**: skill loads normally with full capabilities
- **Hash mismatch**: skill tools are **disabled**; only instruction content is injected into the prompt

This prevents tampered skills from executing tools while still allowing the user to see what the skill contains. The user can re-install or update the skill to restore full functionality.

## Sandboxing

Third-party skill tools are restricted to operating within the skill's own directory. Path traversal attempts (e.g., `../../../etc/passwd`) are detected and blocked. Symlinks that resolve outside the skill directory are also rejected.

This sandbox applies only to ThirdParty skills. Local and Official skills are not sandboxed.

## Prompt Rendering

When multiple skills are active, third-party skills are:

1. **Sorted last** in the prompt — after Official and Local skills
2. **Annotated** with a caution note indicating they are third-party
3. **Labeled** with a `trust: third-party` attribute visible to the agent

This ensures the agent is aware of the trust level and can weight instructions accordingly.

## Limitations & Future Work

- **Source allowlisting is deferred.** Currently, every third-party install requires `--trust` or interactive consent. A future `skills.allowed_sources` config list would let teams pre-approve trusted organizations (e.g., `github.com/my-org/*`). This is not needed yet — `--trust` per-install combined with lockfile pinning covers the primary use case.
- **No trust revocation command.** To revoke trust from an installed third-party skill, users must `corvus skills remove` and re-install. A dedicated revoke command is a future consideration.
- **No re-consent on update.** When `corvus skills update` changes a third-party skill's `allowed-tools`, the update does not currently re-trigger the trust gate. This is a future hardening opportunity.
