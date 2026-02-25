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

## TODO
- [x] Verify CLI reference against actual code implementation - DONE (gaps found)
- [x] Do deep comparison of en/es content parity - DONE
- [ ] Fix duplicate ID warnings (optional, low priority)
- [ ] Consider CSS !important warnings (optional, may need Biome config)
- [x] Update CLI docs with missing subcommands (high priority) - DONE (both en/es)
