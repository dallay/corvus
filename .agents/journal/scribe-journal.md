# Scribe's Documentation Journal 📝

## 2026-02-25 - Initial Assessment - COMPLETE

### Verification
- Ran `./gradlew :web:docsCheck` - Passes with warnings (CSS !important usage)
- Ran `./gradlew :web:docsBuild` - Builds successfully
- Verified directory structure has bilingual parity (en/es)
- Checked symlink `docs/` exists and points to correct location

### Changes
- No changes made yet - initial assessment phase
- Identified documentation structure:
  - 16 English guides + intro + index + 404
  - 16 Spanish guides + intro + index + 404
  - All files appear to have 1:1 parity

### Validation
- ✅ `make docs-web-check` equivalent: `./gradlew :web:docsCheck` passes
- ✅ `make docs-web-build` equivalent: `./gradlew :web:docsBuild` passes
- ⚠️ Warning: CSS lint issues in `custom.css` (60 warnings about !important)
- ⚠️ Warning: Duplicate IDs in architecture docs (Starlight folder/index.md conflict)

### Notes
- Glossary: Need to establish consistent terminology for bilingual docs
- Build system issue: Makefile uses `:docs:websiteCheck` but project is `:web`
  - Should fix Makefile to use correct gradle path OR the web module should register under :docs

---

## 2026-02-25 - CLI Reference Verification - COMPLETE

### Verification
- Compared CLI documentation against `clients/agent-runtime/src/main.rs`
- Verified commands and subcommands match actual implementation

### Discrepancies Found
| Command | Missing in Docs |
|---------|----------------|
| `auth paste-redirect` | NOT documented |
| `auth paste-token` | NOT documented |
| `auth setup-token` | NOT documented |
| `auth refresh` | NOT documented |
| `plugins pin` | NOT documented |
| `plugins revocations sync` | NOT documented |
| `channel doctor` | NOT documented |

### Actions
- [ ] TODO: Update CLI reference docs to include missing subcommands
- [ ] TODO: Sync both English and Spanish versions

---

## 2026-02-25 - Bilingual Parity Check - COMPLETE

### Verification
- Compared file sizes between en/es guides
- Checked architecture.md and architecture/index.md for content parity

### Results
- ✅ All guide files exist in both languages
- ✅ Content is properly translated (architecture.md, architecture/index.md)
- ⚠️ Some English text remains in Spanish docs (diagram names in tables)
  - This is acceptable as diagrams are shared between languages

### Duplicate ID Warning
- Cause: Both `guides/architecture.md` and `guides/architecture/index.md` resolve to same Starlight ID
- Impact: Low - warnings only, no build failures
- Recommendation: Accept as-is or restructure (e.g., rename architecture.md to overview.md)

---

## 2026-03-04 - Codecov Web Configuration - COMPLETE

- Configured Vitest coverage in `clients/web/apps/chat` and `clients/web/apps/dashboard` using `v8` provider.
- Added `test:coverage` script to `package.json` of these apps.
- Updated `.github/codecov.yml` with flags for `rust`, `kotlin`, and `web`.
- Extended the Gradle build in `clients/web/build.gradle.kts` to include an aggregate task `testCoverageAllWebApps`.
- Updated `.github/workflows/pull-request-check.yml` to execute web coverage tests and upload `lcov.info` files to Codecov.
- Standardized `@vitest/coverage-v8` version in `clients/web/pnpm-workspace.yaml` catalog.

- Optimized Gradle web tasks to be configuration-cache compliant by deferring package.json script checks to execution time using `onlyIf`.
- Improved Codecov reliability in CI by using `directory` parameter for web coverage uploads, ensuring all `lcov.info` files are discovered without relying on runner-side glob expansion.

- Pinned C4-PlantUML include to v2.13.0 for documentation stability.
- Localized actor labels in Spanish container diagrams.
- Corrected GPG setup documentation regarding key sizes and CI/CD subkey export commands.
- Added language tags to PGP blocks in Spanish documentation to satisfy linting.

---

## 2026-03-06 - Documentation Review & Sync - COMPLETE

**Verification:**
- Audited `clients/agent-runtime/src/lib.rs` and `main.rs` against CLI docs.
- Discovered missing `update` command in CLI reference.
- Identified generic/outdated content in `intro/introduction.mdx`.
- Verified `make` commands in root `README.md` were incorrect (`docs-web-build` vs `docs-build`).
- Confirmed `guides/structure.md` and `guides/getting-started.md` are accurate.

**Changes:**
- **CLI Reference (en/es):**
  - Added full `update` command section.
  - Refined `peripheral flash` to include `-p/--port`.
- **Introduction (en/es):**
  - Replaced generic placeholder text with detailed project info from root README.
  - Fixed image asset path by creating `clients/web/apps/docs/src/assets/` and copying `corvus.png`.
- **Root README:**
  - Corrected documentation build commands to `make docs-build` and `make docs-dev`.

**Validation:**
- ✅ `make docs-check`: Passed.
- ✅ `make docs-build`: Passed (after fixing image path).

**Notes:**
- Architecture guides are split into `architecture.md` (Overview) and `architecture/overview.md` (Diagrams Index). This is functional but slightly confusing in structure.
- `architecture/overview.md` is orphaned from the sidebar but reachable via links.

---

## 2026-05-22 - CLI Reference Audit & Update - COMPLETE

**Verification:**

- Audited `clients/agent-runtime/src/main.rs` and `lib.rs` to identify undocumented CLI features.
- Found missing items: `peripheral setup-uno-q`, `migrate openclaw` (with `--dry-run` and `--source`), `hardware info --chip`, and `models refresh --provider`.

**Changes:**

- Updated `en/guides/cli-reference.md` and `es/guides/cli-reference.md` to include the missing commands and flags.
- Ensured 1:1 parity between English and Spanish versions.
- Refined formatting for better readability of technical specs.

**Validation:**
- ✅ `./gradlew :web:docsCheck`: Biome linting passed.
- ✅ `./gradlew :web:docsBuild`: Documentation site built successfully with no broken links.
- ✅ Visual Verification: Used Playwright to capture and inspect screenshots of the rendered pages (`cli_docs_en.png`, `cli_docs_es.png`). Layout and translations verified.

**Notes:**
- Glossary: "Dry run" consistently translated as "Simulación" in Spanish docs.
- Parity: Maintained strict alignment of section IDs and heading levels for the language switcher.

---

## TODO
- [x] Verify CLI reference against actual code implementation - DONE (gaps found)
- [x] Do deep comparison of en/es content parity - DONE
- [ ] Fix duplicate ID warnings (optional, low priority)
- [ ] Consider CSS !important warnings (optional, may need Biome config)
- [x] Update CLI docs with missing subcommands (high priority) - DONE (both en/es)
- [x] Audit and document missing CLI flags/commands from agent-runtime - DONE

---

## 2026-03-08 - 404 Status Page Fixes - COMPLETE

**Verification:**
- Analyzed Ahrefs report identifying 404 errors for diagram assets (.mmd) and relative links.
- Verified that Starlight serves the `public/` directory at the root, making assets available at predictable absolute paths.
- Confirmed that sibling links like `./structure/` in subdirectories like `/guides/getting-started/` were incorrectly resolving to `/guides/getting-started/structure/` instead of `/guides/structure/`.

**Changes:**
- **Asset Migration:**
  - Moved all Mermaid (`.mmd`) and PlantUML (`.puml`) diagrams from `src/content/docs/[en|es]/guides/architecture/diagrams/` to `public/guides/architecture/diagrams/`.
  - Removed the now-empty diagram directories from the source content to prevent build warnings and confusion.
- **Link Corrections:**
  - **Getting Started (en/es):** Fixed sibling links (`structure`, `features`, `development`) by changing `./` to `../`.
  - **Release Process (en/es):** Fixed the `gpg-setup` link by changing `./` to `../`.
  - **Architecture (en/es):**
    - Updated diagram table links to use absolute paths (e.g., `/guides/architecture/diagrams/...`).
    - Fixed the Architecture Overview link to point to the correct route `./overview/` instead of the file `./architecture/overview.md`.
  - **Architecture Overview (en/es):** Updated diagram file references to use the new absolute paths in `public/`.

**Validation:**
- ✅ `./gradlew :web:docsCheck`: Passed.
- ✅ `./gradlew :web:docsBuild`: Passed.
- ✅ Visual & Link Verification: Used Playwright to verify that links in the rendered documentation point to the correct resolved URLs (e.g., `../structure/` resolves correctly and diagram links use the absolute path).

**Notes:**
- **Best Practice:** Always use absolute paths starting with `/` for assets in the `public/` directory to ensure they resolve correctly from any nested route.
- **Bilingual Parity:** Maintained strict 1:1 parity between English and Spanish documentation changes.
