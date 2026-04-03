# Surface Contract: web/apps/dashboard

## Metadata

- **Role**: Operator/Admin (Web)
- **Transport**: HTTP Gateway
- **Location**: `clients/web/apps/dashboard/`
- **Status**: Complete
- **Spec**: [Canonical matrix](../spec.md)

## Role Definition

Admin panel for operators managing Corvus runtime configuration, agent settings, and operational
oversight. All operations flow through the HTTP Gateway API with bearer token authentication.

## Mandatory Capabilities

### Runtime Configuration

- [ ] Provider and model selection
- [ ] Temperature and default settings
- [ ] Memory backend configuration
- [ ] Gateway port and host settings
- [ ] Pairing and token management

### Agent Management

- [ ] Agent creation and configuration
- [ ] Agent deletion
- [ ] Agent behavior settings

### Session Monitoring

- [ ] Active session list
- [ ] Session inspection
- [ ] Session termination

### Memory Administration

- [ ] Cerebro memory viewing
- [ ] Memory management controls
- [ ] Embedding configuration

### MCP Server Configuration

- [ ] MCP server registration
- [ ] MCP server removal
- [ ] MCP tool visibility settings

### Approval Policy Management

- [ ] Autonomy level configuration
- [ ] Risk threshold settings
- [ ] Approval rule definition

### Audit and Observability

- [ ] Audit log viewing
- [ ] Health status display
- [ ] Channel status overview

### Gateway Integration

- [ ] Options catalog fetch (`GET /web/admin/options`)
- [ ] Config read (`GET /web/admin/config`)
- [ ] Config update (`PUT /web/admin/config`)
- [ ] Provider pool management (`GET/PUT /web/admin/provider-pools`)

## Optional Capabilities

- [ ] Quick pair flow with magic link support (`#/quick-pair?pairingCode=...`)
- [ ] Secret management UI (replace/clear modes)
- [ ] Conflict detection for restart-required settings

## Out-of-Scope

| Capability                    | Reason                    |
|-------------------------------|---------------------------|
| Direct runtime process access | Gateway API only          |
| Runtime binary modification   | Configuration only        |
| Chat message composition      | Chat surface handles this |
| Mobile-specific features      | ComposeApp handles this   |

## Current Status

**Complete**: Dashboard implements all mandatory capabilities.

## Transport Rule

Dashboard **MUST** use HTTP Gateway only. Direct runtime access is prohibited.

## Security Notes

- Bearer token required for all admin endpoints
- Pairing flow for token acquisition
- No direct config.toml modification
- Section-based save with loading states

## UI Framework

- Vue 3 + TypeScript
- No explicit UI library (custom components)
- Section-based form layout

## i18n Requirements

**Locale Tier**: Tier 1 — Full
**Supported Locales**: en, es
**Parity Requirement**: Mandatory — CI-enforced
**Glossary Compliance**: Mandatory — CI-enforced

### String Externalization

- The surface MUST use `@corvus/locales` as its translation source
- All UI strings MUST use `t("key")` calls — no hardcoded user-facing strings in Vue templates
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention
- Admin-specific terminology MUST use canonical glossary terms: "runtime" (not "server" or
  "backend"), "gateway" (not "API" or "proxy"), "tool" (not "action" or "function"),
  "session" (not "conversation")

### Parity Testing

- The surface MUST pass the `parity.spec.ts` test for key parity across locales
- All translation keys MUST be present in both `en.json` and `es.json`
- The CI parity check MUST pass on every PR that touches locale files

### Design Tokens

- The surface MUST use `--corvus-*` CSS custom properties from the canonical token catalog
- The surface MUST support light and dark themes via token switching
- No hardcoded color values MUST exist outside the token definition file

### Scenarios

#### Scenario: Dashboard passes i18n audit

- GIVEN the `web/apps/dashboard` surface
- WHEN the i18n compliance audit runs
- THEN all translation keys MUST be present in both `en.json` and `es.json`
- AND no hardcoded user-facing strings MUST exist in Vue templates
- AND all admin terms (runtime, gateway, session, tool) MUST match the canonical glossary

#### Scenario: Dashboard configuration labels use canonical terms

- GIVEN the dashboard renders runtime configuration forms
- WHEN field labels reference Corvus concepts
- THEN labels MUST use "runtime" (not "server" or "backend"), "gateway" (not "API" or "proxy"),
  and "tool" (not "action" or "function")

#### Scenario: Dashboard uses canonical tokens

- GIVEN the dashboard surface's CSS
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `--corvus-*` custom properties

### References

- [i18n Governance Specification](../../i18n-governance/spec.md)
- [Design Token Governance](../../design-tokens/spec.md)
- [Canonical Glossary](../../../glossary/README.md)

## Change History

| Version | Date       | Changes                                               |
|---------|------------|-------------------------------------------------------|
| 1.1.0   | 2026-03-24 | Added i18n Requirements section (Tier 1 — Full, #278) |
