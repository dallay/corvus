# Design: Skills Product Contracts

## Overview

This change is primarily documentation. The 4 product contracts formalize behavior that is already
implemented in `clients/agent-runtime/src/skills/`. No architectural changes are required.

## Architecture Decisions

### 1. Contracts Are Additive Documentation

The contracts document existing code behavior — they do not prescribe new implementations. The
source of truth for current behavior remains the code itself; these contracts define the **product
intent** and **user-facing guarantees**.

### 2. No Code Changes in This Change

All implementation work was completed in prior commits (`682fbeed`, `ef20eb39`). This change closes
the documentation gap.

### 3. Deferred Items

| Item                                           | Rationale                                                      | Future Scope                                                      |
|------------------------------------------------|----------------------------------------------------------------|-------------------------------------------------------------------|
| `--json` flag for CLI commands                 | Useful for scripting/CI but not blocking                       | Follow-up: add to `list`, `search`, `list --catalog`              |
| `skills.agent_auto_install` config key         | Policy defined in contract; config key implementation deferred | Follow-up: add config key with `"catalog-only"` / `"none"` values |
| Source allowlisting (`skills.allowed_sources`) | `--trust` per-install is sufficient for current needs          | Follow-up: when CI automation demand materializes                 |

## Contract Structure

Each contract follows a consistent format:

- **Overview** — what the contract covers
- **User Experience** — commands, flows, examples
- **Format / Layout** — file structures, schemas
- **Trust & Security** — trust tier, gating, enforcement
- **Limitations & Future Work** — deferred items with rationale

## Verification Approach

Cross-check each contract against the implementation files identified in the exploration:

- `mod.rs`, `trust.rs`, `validation.rs`, `scanner.rs`, `sandbox.rs`, `lockfile.rs`, `catalog.rs`,
  `frontmatter.rs`
