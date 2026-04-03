# Glossary Governance

> Term lifecycle process for the Corvus Product Glossary.
> Canonical glossary: [`terms.json`](./terms.json) |
> Human-readable reference: [`README.md`](./README.md)

## Ownership

The canonical glossary is owned by the **architecture team**. Changes require approval from:

1. At least one **architecture team member** (mandatory for all changes)
2. At least one **surface maintainer** from an affected surface (mandatory for term additions and
   modifications that impact existing surfaces)

## Proposing a New Term

1. **Open a change proposal** following the standard SDD process (proposal → spec → design → tasks)
2. **Add the term** to `terms.json` with all required fields:
    - `canonical` — the English canonical form
    - `definition` — what the term means in Corvus context
    - `context` — where and when to use this term
    - `aliases` — acceptable synonyms (if any)
    - `anti_terms` — disallowed synonyms that MUST NOT appear in locale files
    - `locales.es` — canonical Spanish translation (required for all Tier 1 locales)
3. **Add the term** to `README.md` following the existing entry format
4. **Submit a PR** with both file changes and request review from architecture + affected surface
   maintainer(s)
5. **CI validation**: The PR pipeline will validate JSON schema compliance and scan existing locale
   files for conflicts with the new term's anti-terms

## Modifying an Existing Term

1. **Open a change proposal** with justification and impact analysis
2. **Update the entry** in `terms.json` — modify the relevant fields
3. **Update the entry** in `README.md` to match
4. **Impact assessment**: If the modification changes `canonical`, `anti_terms`, or `locales`,
   identify all surfaces that reference the term and include migration steps in the proposal
5. **Submit a PR** with review from architecture + all affected surface maintainers

### Modification Scope

| Field Changed           | Impact                                               | Review Required                            |
|-------------------------|------------------------------------------------------|--------------------------------------------|
| `definition`, `context` | Documentation only                                   | Architecture                               |
| `aliases`               | May affect lint rules                                | Architecture                               |
| `anti_terms`            | CI lint rules change; existing locale files may fail | Architecture + all surface maintainers     |
| `canonical`             | All surfaces must update UI strings                  | Architecture + all surface maintainers     |
| `locales`               | Translated surfaces must update                      | Architecture + affected surface maintainer |

## Deprecating a Term

Terms referenced in active locale files MUST NOT be removed immediately. The deprecation process is:

1. **Mark as deprecated** in `terms.json` by adding a `deprecated` field:

   ```json
   {
     "canonical": "Old Term",
     "deprecated": {
       "since": "1.1.0",
       "replacement": "new_term_key",
       "removal_target": "2.0.0"
     }
   }
   ```

2. **Grace period**: The deprecated term MUST remain in the glossary for at least **2 minor version
   bumps** of the glossary (e.g., deprecated in 1.1.0, removable from 1.3.0 onward)
3. **CI behavior during grace period**: The deprecated term's anti-terms are still enforced, and CI
   SHOULD warn (not fail) when the deprecated canonical form is used
4. **Removal**: After the grace period, the term MAY be removed from `terms.json` and `README.md`
   only if no active locale files reference it
5. **Audit before removal**: Run a full-text search across all locale files to confirm zero
   references before removing

## Anti-Term Enforcement in CI

Anti-terms defined in `terms.json` are enforced by CI on every PR that modifies locale or string
files:

### Enforcement Rules

| Surface Tier          | Enforcement Level | CI Behavior                                                |
|-----------------------|-------------------|------------------------------------------------------------|
| Tier 1 (Full i18n)    | Mandatory         | Build **fails** if anti-terms found in locale files        |
| Tier 2 (Content i18n) | Recommended       | Build **warns** if anti-terms found in content files       |
| Tier 3 (English-only) | Advisory          | Build **warns** if anti-terms found in user-facing strings |

### Scan Scope

The CI anti-term scanner checks:

- `clients/web/packages/shared/locales/**/*.json` — web locale files
- `clients/composeApp/**/values*/strings.xml` — mobile string resources
- `clients/web/apps/docs/src/content/**/*.{md,mdx}` — documentation content

### Exclusions

The following are excluded from anti-term scanning:

- Code comments and documentation within source files
- Technical identifiers (variable names, function names, API paths)
- Quoted references to external systems or third-party terminology
- The glossary files themselves (`terms.json`, `README.md`, `GOVERNANCE.md`)

## Dispute Resolution

If surface teams disagree on terminology:

1. **Discussion**: Open an issue tagged `terminology` for asynchronous discussion
2. **Evidence gathering**: Each side presents usage data, user research, or precedent
3. **Architecture decision**: The architecture team makes the final decision within one sprint
4. **Documentation**: The decision and rationale are recorded in the term's `context` field

## References

- [i18n Governance Specification](../specs/i18n-governance/spec.md) — Locale tiers, parity rules,
  and key naming conventions
- [Design Token Governance](../specs/design-tokens/spec.md) — Visual language naming conventions
- [Client Surfaces Specification](../specs/client-surfaces/spec.md) — Surface registry and
  capability matrix
