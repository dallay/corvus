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

| Capability | Reason |
|-----------|--------|
| Direct runtime process access | Browser sandboxing prevents |
| Local filesystem access | Browser sandboxing prevents |
| Native notification dispatch | Browser API only, not full OS |
| Runtime configuration editing | Dashboard surface handles this |
| Admin/operator controls | Dashboard surface handles this |

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
