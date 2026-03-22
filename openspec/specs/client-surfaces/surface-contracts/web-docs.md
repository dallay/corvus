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

```
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
