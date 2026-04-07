# Design: Productize Model Routing (Phase 1)

## Technical Approach

Additive-only changes across four independent deliverables: operator documentation, doctor check hardening, warning log improvements, and a formal openspec specification. No runtime behavior changes. Each deliverable can be implemented, tested, and reverted independently.

The approach maps directly to the proposal's four scope items. The spec provides the behavioral contract, the docs make it discoverable, the doctor checks validate configuration integrity, and the warning logs surface silent failure modes.

## Architecture Decisions

### Decision: Documentation Location

**Choice**: New file at `clients/web/apps/docs/src/content/docs/guides/model-routing.md` (English) with a corresponding `es/guides/model-routing.md` (Spanish) following the existing i18n mirror pattern.

**Alternatives considered**:
- Extend existing `configuration.md` — rejected because routing is a full feature guide, not a config reference entry. The existing `configuration.md` covers Gradle/build config, not runtime TOML config.
- Create under a new `guides/providers/` subdirectory — rejected because no such subdirectory convention exists. All current guides are flat files under `guides/`.
- Create under `guides/architecture/` — rejected because this is an operator guide, not an architecture document.

**Rationale**: The existing docs structure uses flat files under `guides/` with consistent frontmatter (`title`, `description`, `owner`, `status`, `lastReviewed`, `appliesTo`, `docType`). A new file follows this convention exactly. The `es/` mirror ensures bilingual parity consistent with every other guide.

### Decision: Doctor Checks as Warnings Only

**Choice**: All new doctor checks emit `Severity::Warn`, never `Severity::Error`.

**Alternatives considered**:
- Emit `Severity::Error` for orphaned classification hints — rejected because existing configs with orphaned hints currently work (they silently fall back to default). Making them errors would break existing setups.
- Add a `--strict` flag to doctor — rejected as out of scope (Phase 2+ concern).

**Rationale**: The proposal explicitly states "all as warnings, not errors." Existing configs that have classification rules referencing non-existent hints currently work via silent fallback. Promoting these to errors would be a breaking change requiring a major version bump. Warnings surface the issue without disrupting operators.

### Decision: New Doctor Checks in Existing Function Structure

**Choice**: Add a new `check_classification_integrity` function called from `check_config_semantics`, following the existing pattern of small focused check functions (e.g., `check_model_routes`, `check_fallback_providers`).

**Alternatives considered**:
- Extend the existing `check_model_routes` function — rejected because classification checks are a separate concern. The existing function validates individual route fields; the new checks validate cross-cutting classification ↔ route relationships.
- Create a separate top-level check category (e.g., `[classification]` in output) — rejected because these are config semantic checks and belong under the existing `[config]` category for consistency.

**Rationale**: The existing `check_config_semantics` function dispatches to small focused functions. Adding `check_classification_integrity` follows this exact pattern: called from `check_config_semantics`, receives `config`, `cat`, and `items`, pushes `DiagItem::warn` entries.

### Decision: Warning Logs Use Structured `tracing::warn!` Fields

**Choice**: Use `tracing::warn!` with named structured fields (e.g., `hint = hint`, `provider = name`) consistent with existing tracing usage in `router.rs` and `providers/mod.rs`.

**Alternatives considered**:
- Use `tracing::error!` — rejected because these are recoverable situations with defined fallback behavior, not failures.
- Use `eprintln!` — rejected because the codebase consistently uses `tracing` for all runtime logging.

**Rationale**: Both `router.rs` and `providers/mod.rs` already use `tracing::warn!` with structured fields for similar situations (e.g., line 54 of router.rs: `tracing::warn!(hint = hint, provider = route.provider_name, ...)`). The new warnings follow this exact pattern for consistency and structured log query support.

### Decision: Spec Location Following Existing Convention

**Choice**: `openspec/specs/model-routing/spec.md` — a new domain directory under the existing `openspec/specs/` tree.

**Alternatives considered**:
- Nest under `openspec/specs/agent-runtime-providers/` — rejected because routing is a distinct domain from provider management. Existing specs each have their own top-level domain directory.
- Use `openspec/specs/routing/` — rejected because `model-routing` is more specific and matches the feature name.

**Rationale**: The existing 32 spec directories each represent a distinct behavioral domain (e.g., `audio-input`, `channel-image-ingestion`, `provider-vision-gating`). `model-routing` follows this naming pattern. The spec format follows the established convention: header with Domain/Status/Issue/Date, Overview, Definitions, Requirements with REQ-N numbering, and Given/When/Then scenarios.

## Data Flow

No runtime data flow changes. The existing flow remains:

```
User Message
    │
    ▼
classify(config, message) ──→ Option<hint>
    │                              │
    │  (if hint found in routes)   │
    ▼                              ▼
"hint:{hint}" as model param    default model
    │                              │
    └──────────┬───────────────────┘
               ▼
    RouterProvider.resolve(model)
               │
    ┌──────────┼──────────────┐
    ▼          ▼              ▼
 Matched    Unknown hint   Non-hint
  Route     (fallback +    (default
            NEW warning)   provider)
    │          │              │
    ▼          ▼              ▼
  Routed    Default        Default
 Provider   Provider       Provider
```

Changes are purely additive:
- **Doctor**: validates config BEFORE runtime (offline check)
- **Warning logs**: annotate existing fallback paths AT runtime (no behavior change)
- **Docs/Spec**: exist outside the runtime entirely

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/web/apps/docs/src/content/docs/guides/model-routing.md` | Create | Operator guide: model routing and query classification config, TOML examples, hint flow diagram, troubleshooting |
| `clients/web/apps/docs/src/content/docs/es/guides/model-routing.md` | Create | Spanish translation of the operator guide (mirror of English) |
| `clients/agent-runtime/src/doctor/mod.rs` | Modify | Add `check_classification_integrity` function with 4 new warning checks |
| `clients/agent-runtime/src/providers/router.rs` | Modify | Enhance existing `tracing::warn!` on line 83 to include the fallback model name |
| `clients/agent-runtime/src/providers/mod.rs` | Modify | Add route-impact warning when non-primary routed provider fails init (~line 752) |
| `openspec/specs/model-routing/spec.md` | Create | Formal routing behavior specification with Given/When/Then scenarios |

## Interfaces / Contracts

### New Doctor Check Function

```rust
// In doctor/mod.rs — called from check_config_semantics

fn check_classification_integrity(
    config: &Config,
    cat: &'static str,
    items: &mut Vec<DiagItem>,
) {
    let classification = &config.query_classification;
    let route_hints: std::collections::HashSet<&str> = config
        .model_routes
        .iter()
        .map(|r| r.hint.as_str())
        .collect();

    // Check 1: classification enabled with zero rules
    if classification.enabled && classification.rules.is_empty() {
        items.push(DiagItem::warn(
            cat,
            "query_classification.enabled = true but no rules configured",
        ));
    }

    // Check 2: classification enabled with zero model_routes
    if classification.enabled && config.model_routes.is_empty() {
        items.push(DiagItem::warn(
            cat,
            "query_classification.enabled = true but no model_routes configured",
        ));
    }

    // Check 3: orphaned classification rule hints
    for rule in &classification.rules {
        if !rule.hint.is_empty() && !route_hints.contains(rule.hint.as_str()) {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "classification rule hint \"{}\" does not match any model_routes entry",
                    rule.hint
                ),
            ));
        }
    }

    // Check 4: never-matching rules (empty keywords AND empty patterns)
    for rule in &classification.rules {
        if rule.keywords.is_empty() && rule.patterns.is_empty() {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "classification rule for hint \"{}\" has no keywords or patterns (will never match)",
                    rule.hint
                ),
            ));
        }
    }
}
```

### Enhanced Warning in router.rs (resolve method)

```rust
// In RouterProvider::resolve, replacing line 83-86
// Current:
//   tracing::warn!(hint = hint, "Unknown route hint, falling back to default provider");

// Enhanced:
tracing::warn!(
    hint = hint,
    fallback_model = model,
    "Unknown route hint, falling back to default provider with raw model string"
);
```

### Route-Impact Warning in providers/mod.rs (create_routed_provider)

```rust
// In create_routed_provider, around line 748-755, after the existing warn
// Current:
//   tracing::warn!(provider = name.as_str(),
//       "Ignoring routed provider that failed to initialize");

// Enhanced:
let affected_hints: Vec<&str> = model_routes
    .iter()
    .filter(|r| &r.provider == name)
    .map(|r| r.hint.as_str())
    .collect();
tracing::warn!(
    provider = name.as_str(),
    affected_routes = ?affected_hints,
    "Ignoring routed provider that failed to initialize — routes using this provider will fail at request time"
);
```

### Documentation Frontmatter

```yaml
---
title: Model Routing & Query Classification
description: Configure multi-model routing with task hints and automatic query classification in the Corvus agent runtime.
owner: team-platform
status: canonical
lastReviewed: 2026-04-07
appliesTo: main
docType: guide
---
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `check_classification_integrity` — orphaned hint warning | Build `Config` with classification rule referencing non-existent hint, assert `Severity::Warn` with expected message |
| Unit | `check_classification_integrity` — enabled with no rules | Build `Config` with `enabled=true`, empty rules, assert warning |
| Unit | `check_classification_integrity` — enabled with no routes | Build `Config` with `enabled=true`, rules present, no model_routes, assert warning |
| Unit | `check_classification_integrity` — never-matching rule | Build `Config` with rule having empty keywords AND empty patterns, assert warning |
| Unit | `check_classification_integrity` — valid config produces no warnings | Build `Config` with matching hints/routes, assert no warn items |
| Unit | Enhanced router warning — unknown hint includes fallback model | Existing `unknown_hint_falls_back_to_default` test already validates behavior; new test or log assertion optional |
| Build | Docs site builds successfully with new guide | `make docs-build` passes |
| Lint | Rust code passes clippy | `cargo clippy --all-targets -- -D warnings` |

All tests follow the existing `doctor/mod.rs` test pattern: construct a `Config`, call the check function, find items by message substring, assert severity. See lines 741-855 of `doctor/mod.rs` for reference examples.

## Migration / Rollout

No migration required. All changes are additive:

- **Docs**: Available immediately on next docs site deploy
- **Doctor checks**: Active on next `corvus doctor` run after binary update
- **Warning logs**: Active on next daemon start after binary update
- **Spec**: Static file, no runtime dependency

No feature flags needed. No phased rollout needed. Each deliverable is independently deployable and revertible.

## Open Questions

- [ ] None — all decisions are straightforward given the additive-only constraint and existing patterns in the codebase.
