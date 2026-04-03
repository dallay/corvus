# Surface Contract: web/apps/chat

## Metadata

- **Role**: End-user (Web)
- **Transport**: HTTP Gateway
- **Location**: `clients/web/apps/chat/`
- **Status**: Scaffold (stub implementation)
- **Spec**: [Canonical matrix](../spec.md)
- **Migration**: M2 (Web Chat Stub → Full Implementation)

## Role Definition

Primary web-based chat interface for end users accessing Corvus via browser. The chat surface
provides conversational interaction with the agent through the HTTP Gateway API.

## Mandatory Capabilities

### Chat Composition

- [ ] Text input field with send/cancel controls
- [ ] Message submission via gateway `/chat/send`
- [ ] Streaming response display (WebSocket or SSE)
- [ ] Sync response display fallback
- [ ] Message history rendering (user/assistant bubbles)

### Session Management

- [ ] Session creation via gateway
- [ ] Session resumption with `X-Session-Id`
- [ ] Session termination
- [ ] UUID-based session IDs

### Tool Approval

- [ ] Inline tool call display
- [ ] Approve control
- [ ] Deny control
- [ ] Approval status feedback

### Gateway Integration

- [ ] Pairing code exchange (`POST /pair`)
- [ ] Bearer token authentication
- [ ] Health check connectivity (`GET /health`)
- [ ] URL safety validation (HTTPS enforcement)

## Optional Capabilities

### Memory Display

- [ ] Short-term memory context display
- [ ] Session-scoped memory indicators

### Long-term Memory

- [ ] Cerebro memory query integration
- [ ] Memory tool results in conversation

### MCP Tool Visibility

- [ ] Tool call metadata display
- [ ] Tool execution progress indicators

## Out-of-Scope

| Capability                    | Reason                         |
|-------------------------------|--------------------------------|
| Direct runtime process access | Browser sandboxing prevents    |
| Local filesystem access       | Browser sandboxing prevents    |
| Native notification dispatch  | Browser API only, not full OS  |
| Runtime configuration editing | Dashboard surface handles this |
| Admin/operator controls       | Dashboard surface handles this |

## Current Status

**Gap**: The chat surface currently uses a local stub (`buildLocalAssistantReply`) that generates
fake responses. The `useGateway.ts` composable is empty.

**Required for completion**:

1. Implement `useGateway.ts` composable
2. Wire chat to gateway `/chat/send` endpoint
3. Add WebSocket streaming support
4. Implement session management

See: [Migration M2](../migrations.md#m2-web-chat-stub--full-implementation)

## Transport Rule

Web chat **MUST** use HTTP Gateway only. Process bridges and CLI invocation are prohibited.

## Security Notes

- HTTPS-only for non-localhost (configurable for development)
- Bearer token storage (in-memory, sessionStorage, or secure storage)
- Pairing code never persisted
- Webhook secret validation

## UI Framework

- Vue 3 + TypeScript
- Tailwind CSS
- shadcn-vue-style components
- No shared code with composeApp (separate implementations by design)

## i18n Requirements

**Locale Tier**: Tier 1 — Full
**Supported Locales**: en, es
**Parity Requirement**: Mandatory — CI-enforced
**Glossary Compliance**: Mandatory — CI-enforced

### String Externalization

- The surface MUST use `@corvus/locales` as its translation source
- All UI strings MUST use `t("key")` calls — no hardcoded user-facing strings in Vue templates
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention
- Recovery messages MUST use canonical recovery patterns from the i18n governance spec
- All product terms MUST match the canonical glossary

### Parity Testing

- The surface MUST pass the `parity.spec.ts` test for key parity across locales
- All translation keys MUST be present in both `en.json` and `es.json`
- The CI parity check MUST pass on every PR that touches locale files

### Design Tokens

- The surface MUST use `--corvus-*` CSS custom properties from the canonical token catalog
- The surface MUST support light and dark themes via token switching
- Glass morphism elements MUST use canonical glass tokens
- No hardcoded color values MUST exist outside the token definition file

### Scenarios

#### Scenario: Chat surface passes i18n audit

- GIVEN the `web/apps/chat` surface in production
- WHEN the i18n compliance audit runs
- THEN all translation keys MUST be present in both `en.json` and `es.json`
- AND no hardcoded user-facing strings MUST exist in Vue templates
- AND all product terms MUST match the canonical glossary
- AND the CI parity check MUST pass

#### Scenario: Chat onboarding uses canonical "pair" term

- GIVEN the chat surface renders the onboarding trust step
- WHEN the step text is displayed
- THEN the text MUST use "pair" (en) or "emparejar" (es)
- AND the text MUST NOT use "link", "connect", or any disallowed synonym

#### Scenario: Chat surface uses canonical tokens

- GIVEN the chat surface's CSS
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `--corvus-*` custom properties
- AND no hardcoded color values MUST exist outside the token definition file

### References

- [i18n Governance Specification](../../i18n-governance/spec.md)
- [Design Token Governance](../../design-tokens/spec.md)
- [Canonical Glossary](../../../glossary/README.md)

## Change History

| Version | Date       | Changes                                               |
|---------|------------|-------------------------------------------------------|
| 1.1.0   | 2026-03-24 | Added i18n Requirements section (Tier 1 — Full, #278) |
