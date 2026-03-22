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

| Capability | Reason |
|-----------|--------|
| Direct runtime process access | Gateway API only |
| Runtime binary modification | Configuration only |
| Chat message composition | Chat surface handles this |
| Mobile-specific features | ComposeApp handles this |

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
