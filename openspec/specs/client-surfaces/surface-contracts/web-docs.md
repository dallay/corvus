# Surface Contract: web/apps/docs

## Metadata

- **Role**: Supporting (Documentation)
- **Transport**: None (static site)
- **Location**: `clients/web/apps/docs/`
- **Status**: Complete
- **Spec**: [Canonical matrix](../spec.md)

## Role Definition

Public-facing documentation site built with Astro Starlight. Provides reference documentation,
guides, and API specifications. Zero runtime interaction by design.

## Mandatory Capabilities

### Documentation Content

- [ ] Architecture overview
- [ ] CLI reference
- [ ] Configuration guide
- [ ] Gateway API documentation
- [ ] MCP protocol documentation
- [ ] Cerebro memory system docs
- [ ] Development guides

### Site Features

- [ ] Search functionality (Starlight built-in)
- [ ] Navigation sidebar
- [ ] Version compatibility indicators
- [ ] Multi-language support (English, Spanish)

### Static Asset Serving

- [ ] Image assets
- [ ] Font loading
- [ ] CSS/JS optimization

## Out-of-Scope

| Capability | Reason |
|-----------|--------|
| Agent chat interaction | Not a chat surface |
| Runtime configuration | Dashboard handles this |
| Memory queries | No runtime access |
| User authentication | Public site |
| Form submissions | Lead capture belongs to marketing |

## Current Status

**Complete**: Documentation site is fully functional with comprehensive content.

## Content Structure

```text
docs/
├── guides/
│   ├── architecture/overview.md
│   ├── cerebro/
│   │   ├── migration.md
│   │   └── mcp-schema/
│   ├── configuration.md
│   └── cli-reference.md
├── clients/
│   └── agent-runtime/
├── intro/
└── es/  (Spanish translations)
```

## Framework

- Astro + Starlight
- Content collections
- MDX support for interactive docs
- Algolia DocSearch (optional)

## No Runtime Access

This surface intentionally has no runtime communication:
- No gateway API calls
- No CLI bridge integration
- No memory system access
- Static content only

## Accessibility

- WCAG 2.1 AA compliance
- Keyboard navigation
- Screen reader support
- High contrast mode

## i18n Requirements

**Locale Tier**: Tier 2 — Content
**Supported Locales**: en, es
**Parity Requirement**: Recommended
**Glossary Compliance**: Recommended

### String Externalization

- The surface MUST support `en` (default) and `es` locales via Starlight file-based routing
  configuration
- Spanish content pages MUST be placed in `src/content/docs/es/`
- Missing Spanish pages SHOULD display the English version with a "not yet translated" indicator
- Missing pages MUST NOT return a 404
- Product terminology in documentation MUST use canonical glossary terms
- The surface SHOULD maintain reasonable content parity (new pages SHOULD have translations within
  one release cycle)

### Parity Testing

- Content parity is recommended but not CI-enforced
- New documentation pages SHOULD have Spanish translations within one release cycle
- A manual review process SHOULD track translation coverage

### Design Tokens

- The surface SHOULD use `--corvus-*` CSS custom properties where applicable
- Starlight's built-in theming MAY be used for light/dark mode
- The surface is not required to implement the full canonical token catalog

### Scenarios

#### Scenario: Docs surface serves translated content

- GIVEN a documentation page exists in both `en` and `es`
- WHEN a user navigates to the Spanish version
- THEN the docs site MUST serve the Spanish content
- AND all product terms in the content MUST match the canonical glossary

#### Scenario: Missing translation falls back gracefully

- GIVEN an English documentation page has no Spanish equivalent
- WHEN a Spanish-locale user navigates to that page
- THEN the docs site MUST display the English content
- AND the page SHOULD include a visible "This page is not yet translated" indicator
- AND the page MUST NOT return a 404

#### Scenario: Docs terminology matches glossary

- GIVEN documentation references the onboarding process
- WHEN the content mentions device trust establishment
- THEN the documentation MUST use "pair" (en) or "emparejar" (es)
- AND MUST NOT use "link", "connect", or other disallowed synonyms

#### Scenario: Docs surface theming

- GIVEN the docs site supports dark mode via Starlight
- WHEN the user toggles the theme
- THEN the theme switch SHOULD use canonical token values where available
- AND the switch MUST NOT break the reading experience

### References

- [i18n Governance Specification](../../i18n-governance/spec.md)
- [Design Token Governance](../../design-tokens/spec.md)
- [Canonical Glossary](../../../glossary/README.md)

## Change History

| Version | Date       | Changes                                                    |
|---------|------------|------------------------------------------------------------|
| 1.1.0   | 2026-03-24 | Added i18n Requirements section (Tier 2 — Content, #278)   |
