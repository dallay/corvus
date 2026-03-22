# Surface Contract: web/apps/marketing

## Metadata

- **Role**: Supporting (Marketing)
- **Transport**: None (static site)
- **Location**: `clients/web/apps/marketing/`
- **Status**: Partial
- **Spec**: [Canonical matrix](../spec.md)

## Role Definition

Public marketing and landing page site built with Astro. Provides product information, installation
guidance, and lead capture. Zero runtime interaction by design.

## Mandatory Capabilities

### Marketing Content
- [ ] Landing page with hero section
- [ ] Feature highlights
- [ ] Pricing information (if applicable)
- [ ] Comparison tables (vs competitors)
- [ ] Testimonials/social proof
- [ ] FAQ section

### Installation
- [ ] Installation script (`/install.sh`)
- [ ] Platform-specific instructions (macOS, Linux, Docker)
- [ ] Quick start guide

### Navigation
- [ ] Header navigation
- [ ] Footer with links
- [ ] CTA buttons (Install, Docs, GitHub)

## Optional Capabilities

### Lead Capture

- [ ] Contact forms
- [ ] Email signup
- [ ] Demo request

**Lead Capture Controls** (required for implementation):

| Control | Requirement |
|---------|-------------|
| Input validation | Sanitize all form fields; reject malformed input |
| Field boundaries | Max length: email (254 chars), name (100 chars), message (2000 chars) |
| Storage/retention | Store in approved CRM only; retain max 2 years or per consent |
| Consent/opt-in | Explicit opt-in checkbox required; pre-checked disallowed |
| Opt-out handling | Unsubscribe link in all emails; honor removal within 48 hours |
| Abuse protection | Rate limiting (max 5 submissions per IP/hour); CAPTCHA on forms |
| Spam filtering | Implement SPF/DKIM/DMARC; block known spam domains |

### Analytics
- [ ] Page view tracking
- [ ] Conversion tracking
- [ ] User behavior analytics

## Out-of-Scope

| Capability | Reason |
|-----------|--------|
| Agent chat interaction | Not a chat surface |
| Runtime configuration | Dashboard handles this |
| Memory queries | No runtime access |
| User authentication | Public marketing site |
| Product documentation | Docs surface handles this |

## Current Status

**Partial**: Landing page exists with basic content. Some sections may need completion.

## Content Structure

```
marketing/
├── pages/
│   ├── index.astro          # Landing page
│   └── install.astro        # Installation guide
├── layouts/
│   └── MarketingLayout.astro
├── components/
│   └── analytics.astro
└── public/
    ├── install.sh           # curl installer
    └── images/
```

## Framework

- Astro
- Static site generation
- Tailwind CSS (if applicable)
- No client-side framework required

## Domain-Aware URL Resolution

The marketing site supports domain-aware URL resolution for multi-environment deployments:
- Production domain resolution
- Local development fallback
- Marketing URL configuration via environment

## No Runtime Access

This surface intentionally has no runtime communication:
- No gateway API calls
- No CLI bridge integration
- Static HTML/CSS/JS only

## Security Notes

- Lead capture forms process minimal user data (see Lead Capture Controls above)
- CSP headers for analytics scripts
- HTTPS enforced in production
- No sensitive data exposure
- Privacy policy and terms of service required for lead capture compliance

## Accessibility

- WCAG 2.1 AA compliance
- Keyboard navigation
- Screen reader support
- Performance optimization (Core Web Vitals)
