---
phase: explore
status: complete
date: 2026-03-24
issue: 278
---

# Exploration: Cross-Client i18n Governance (#278)

## Current i18n State

### Web — Chat & Dashboard (Vue + vue-i18n)

**Status: Well-structured, shared locale package exists.**

- Both `@corvus/chat` and `@corvus/dashboard` depend on `@corvus/locales` (workspace package at
  `clients/web/packages/locales/`).
- Both use `vue-i18n` (Composition API, `legacy: false`).
- Both share **identical** `i18n.ts` config: default locale `es`, fallback `en`.
- Shared locale package exports two JSON files: `src/en.json` (252 lines) and `src/es.json` (252
  lines).
- **Parity test exists**: `parity.spec.ts` validates that `en` and `es` have identical key sets and
  matching placeholders. This is a strong foundation.
- All UI strings in chat and dashboard use `t("key")` calls — no hardcoded strings in Vue templates.
- Translation keys are well-structured: `app.*`, `sections.*`, `auth.*`, `onboarding.*`,
  `chatOnboarding.*`, `form.*`, `chat.*`, `errors.*`, `webhook.*`.

**Gap**: The locale package is web-only. No mechanism shares these translations with mobile or CLI.

### Web — Docs (Astro Starlight)

**Status: Native Starlight i18n, separate from `@corvus/locales`.**

- Uses Starlight's built-in i18n: `defaultLocale: "root"` (English), with `es` locale configured.
- Spanish content lives in `src/content/docs/es/` (22+ translated `.md`/`.mdx` files).
- Does **not** use `@corvus/locales` or `vue-i18n` — Starlight manages its own content localization
  via file-based routing (`/es/guides/...`).
- Does **not** depend on the `@corvus/locales` package (only `@corvus/shared`).

**Gap**: Terminology in docs is manually maintained. No shared glossary or term enforcement.

### Web — Marketing (Astro)

**Status: No i18n infrastructure.**

- No locale files, no i18n library, no translated content found.
- Does not depend on `@corvus/locales` (only `@corvus/shared`).
- Static Astro site with English-only content.

**Gap**: Marketing is entirely English-only with no i18n scaffolding.

### Mobile — ComposeApp (Kotlin Multiplatform Compose)

**Status: Compose Resources with 2 locales, minimal coverage.**

- Uses Compose Multiplatform resources: `composeResources/values/strings.xml` (English, 12 strings)
  and `composeResources/values-es/strings.xml` (Spanish, 12 strings).
- Strings cover onboarding steps and button labels only.
- Uses `stringResource(Res.string.*)` pattern — idiomatic for Compose.
- Agent name defined as `AGENT_NAME = "Corvus Agent"` constant in `App.kt` (hardcoded, not from
  resources).

**Gap**: Only 12 strings localized. Chat UI strings, error messages, session management text — all
missing from resources. No parity test equivalent.

### Rust CLI — Agent Runtime

**Status: No i18n infrastructure. All strings hardcoded in Rust source.**

- All user-facing strings are inline `println!()` / `style()` calls (hundreds of instances in
  `wizard.rs` alone — 5000+ lines).
- No message catalog, no locale files, no `fluent`/`gettext`/`rust-i18n` dependency.
- Strings are English-only, with emoji-heavy formatting (`🦀`, `✅`, `❌`, `🔒`, etc.).
- CLI help text, onboarding wizard, gateway startup messages — all hardcoded.

**Gap**: Complete absence of i18n. This is the largest surface by string count with zero
localization.

## Current UX/UI Language & Design Tokens

### Shared Design Tokens

- `clients/web/packages/shared/tokens.css` — **116-line CSS custom property file** with
  comprehensive brand tokens:
    - Fonts: `--corvus-font-heading` (Syne), `--corvus-font-sans` (Manrope), `--corvus-font-mono` (
      JetBrains Mono)
    - Colors: background, text, accent, gradients, glass morphism, borders
    - Spacing: radius (`sm/md/lg/xl`), transitions
    - Aliases: Dashboard/Chat compatibility layer (`--color-bg-primary`, `--color-accent`, etc.)
- Both chat and dashboard import these tokens. Marketing and docs use `@corvus/shared` but
  tokens.css adoption is unclear.

### Shared UI Components

- `clients/web/packages/ui/` — Minimal shared component library: `Button.vue` and `Input.vue` only.
- Components use CSS custom properties from the token system.
- No shared component library for mobile (Compose has its own theming).

### Mobile Theming

- ComposeApp uses glass morphism styling and dark/light theme support (per contract).
- No shared token file between web and mobile — separate implementations.

## Terminology Audit

| Concept              | Web Locales (en.json)                                                                    | Compose strings.xml                                      | Rust CLI                                                   | Consistency                                           |
|----------------------|------------------------------------------------------------------------------------------|----------------------------------------------------------|------------------------------------------------------------|-------------------------------------------------------|
| **Agent name**       | `"Corvus Agent"` (in `App.vue` constant + `chat.disclaimer`)                             | `AGENT_NAME = "Corvus Agent"` (hardcoded in `App.kt`)    | `"corvus agent"` (CLI command context)                     | Mostly consistent but not from shared source          |
| **Session**          | `"session"` consistently (`newSession`, `startSession`, `resumeSession`, `sessionReady`) | `"session"` in onboarding (`resume_session`)             | `"session"` in config schema                               | Consistent                                            |
| **Conversation**     | Not used in UI strings                                                                   | Not used                                                 | `MemoryCategory::Conversation` (internal, not user-facing) | OK — internal only                                    |
| **Chat**             | `"chat"` for the action/surface (`newChat`, `chat.welcome`)                              | `"chat"` in onboarding descriptions                      | `"Chat: corvus agent -m"` in wizard                        | Consistent                                            |
| **Surface**          | `"surface"` in onboarding text (`"this chat surface"`, `"Trust this surface"`)           | `"surface"` in onboarding (`"link this mobile surface"`) | Not used in user-facing text                               | Consistent where used                                 |
| **Pairing**          | `"pair"` / `"pairing code"` consistently                                                 | `"linking"` / `"link this app"` (different term!)        | `"pairing"` in gateway                                     | **INCONSISTENT**: web says "pair", mobile says "link" |
| **Runtime**          | `"runtime"` consistently                                                                 | `"runtime"` consistently                                 | `"runtime"` consistently                                   | Consistent                                            |
| **Gateway**          | `"gateway"` consistently                                                                 | Not used (mobile uses bridge)                            | `"gateway"` consistently                                   | Consistent (N/A for mobile)                           |
| **Onboarding steps** | 4 steps: runtime → trust → connect → ready                                               | 4 steps: runtime → link → connect → start/resume         | CLI: full wizard (different UX)                            | Structure matches, but "trust" vs "link" differs      |
| **Tool**             | No "tool" in locale strings (UI not built yet)                                           | Not present                                              | Internal `tool` references only                            | N/A — too early                                       |
| **Recovery states**  | 7 normalized states per surface                                                          | Not localized yet                                        | Not applicable (CLI has direct access)                     | Web-only for now                                      |

**Key inconsistency**: Mobile uses "link" where web uses "pair/trust" for the same onboarding
concept.

## Surface Contract Review

All 7 surface contracts were reviewed for i18n/localization mentions:

| Contract               | i18n Mentioned? | Language/Locale Mentioned?                                               | Key Notes                                  |
|------------------------|-----------------|--------------------------------------------------------------------------|--------------------------------------------|
| `web-chat.md`          | No              | No                                                                       | —                                          |
| `web-dashboard.md`     | No              | No                                                                       | —                                          |
| `composeapp-mobile.md` | No              | No                                                                       | —                                          |
| `composeapp-shared.md` | No              | No                                                                       | —                                          |
| `agent-runtime-cli.md` | No              | No                                                                       | —                                          |
| `web-docs.md`          | No              | Yes — "Multi-language support (English, Spanish)" listed as site feature | Only contract acknowledging multi-language |
| `web-marketing.md`     | No              | No                                                                       | —                                          |

**Finding**: No contract mentions i18n governance, shared glossary, or cross-surface language
consistency. The docs contract is the only one acknowledging multi-language support. The canonical
capability matrix (`spec.md` v1.1.0) has no i18n requirements.

## Tech Stack i18n Options

### Vue/Astro Web Stack

| Option                                       | Fit                | Notes                                                                           |
|----------------------------------------------|--------------------|---------------------------------------------------------------------------------|
| **vue-i18n** (already in use)                | Excellent          | Already adopted by chat + dashboard. Composition API mode. JSON message format. |
| **Starlight built-in i18n** (already in use) | Excellent for docs | File-based routing. Cannot share with vue-i18n directly.                        |
| **@formatjs/intl**                           | Alternative        | ICU MessageFormat. More powerful pluralization. Migration cost.                 |
| **Paraglide.js**                             | Alternative        | Compiled i18n, type-safe. Smaller bundles. Would require migration.             |

**Recommendation**: Keep `vue-i18n` for chat/dashboard, Starlight i18n for docs. Focus on shared
source-of-truth for terminology.

### Compose Multiplatform (Kotlin)

| Option                                 | Fit         | Notes                                                                              |
|----------------------------------------|-------------|------------------------------------------------------------------------------------|
| **Compose Resources** (already in use) | Excellent   | Native `stringResource()` pattern. `values/strings.xml` + `values-es/strings.xml`. |
| **Lyricist**                           | Alternative | Kotlin-first i18n for Compose. More flexible than XML resources.                   |
| **moko-resources**                     | Alternative | KMP resource management. Generates type-safe accessors.                            |

**Recommendation**: Keep Compose Resources (already adopted). Expand string coverage significantly.

### Rust CLI

| Option                         | Fit                   | Notes                                                                     |
|--------------------------------|-----------------------|---------------------------------------------------------------------------|
| **rust-i18n**                  | Good                  | Macro-based. YAML/JSON/TOML locale files. Compile-time checked.           |
| **fluent-rs** (Project Fluent) | Excellent             | Mozilla's i18n system. Powerful pluralization, gender, etc. `.ftl` files. |
| **gettext-rs**                 | Moderate              | Classic `.po`/`.pot` files. Mature ecosystem. Heavier.                    |
| **No i18n (status quo)**       | Acceptable short-term | CLI is operator-facing. English-only is defensible for v1.                |

**Recommendation**: For v1, Rust CLI i18n is lowest priority (operator audience, English-dominant).
If pursued, `fluent-rs` or `rust-i18n` are the idiomatic choices.

## Key Findings & Risks

1. **Strong web foundation exists**: `@corvus/locales` with parity tests is a solid pattern. Chat
   and dashboard are fully localized (en/es) through a shared package.

2. **Mobile is underserved**: Only 12 strings localized in Compose resources. No parity test. The
   `AGENT_NAME` is hardcoded outside resources.

3. **CLI has zero i18n**: Hundreds of hardcoded strings. This is acceptable for an operator tool but
   creates terminology drift risk.

4. **Terminology inconsistency already exists**: Mobile says "link" where web says "pair/trust" for
   the same onboarding step. This contradicts the cross-surface parity intent.

5. **No shared glossary**: Each surface defines terms independently. "Corvus Agent" appears in 3
   places with no single source of truth.

6. **Docs and marketing are islands**: Docs uses Starlight's own i18n (file-based). Marketing has
   none. Neither shares terminology with `@corvus/locales`.

7. **Design tokens are web-only**: `tokens.css` has no mobile equivalent. Token naming is not
   governed across platforms.

8. **No contracts mandate i18n**: None of the 7 surface contracts mention localization requirements.
   The capability matrix has no i18n column.

9. **Two supported locales**: English and Spanish are the only locales across all surfaces. Adding a
   third locale would require changes in 4+ places with no coordination mechanism.

10. **Risk: drift will accelerate**: As chat and mobile surfaces move from scaffold to full
    implementation, string counts will grow rapidly. Without governance now, divergence will
    compound.

## Open Questions

1. **Scope of governance**: Should this spec govern only terminology consistency (glossary), or also
   mandate i18n infrastructure for all surfaces?

2. **CLI i18n priority**: Is Rust CLI localization in scope for this change, or deferred? The string
   count is very large (~500+ user-facing strings).

3. **Glossary format**: Should the shared glossary be a JSON file (machine-readable), a markdown
   doc (human-readable), or both?

4. **Locale expansion**: Is the plan to stay with en/es, or are additional locales expected? This
   affects architecture decisions (e.g., lazy loading, CDN-hosted locales).

5. **Marketing i18n**: Should marketing adopt `@corvus/locales` or remain English-only?

6. **Mobile parity test**: Should mobile have an equivalent to the web `parity.spec.ts`? If so,
   how (Kotlin test reading XML resources)?

7. **"Link" vs "Pair" resolution**: Which term wins for the mobile onboarding trust step? This is a
   product decision, not just i18n.

8. **Design token governance**: Is cross-platform token governance (CSS ↔ Compose theme) in scope,
   or a separate concern?

9. **CI enforcement**: Should CI validate cross-surface terminology consistency (e.g., a shared
   glossary lint)?

10. **Ownership**: Who owns the shared glossary — architecture team, product, or individual surface
    maintainers?
