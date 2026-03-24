# Surface Contract: agent-runtime (CLI)

## Metadata

- **Role**: Operator/Admin
- **Transport**: Direct (CLI subprocess)
- **Location**: `clients/agent-runtime/`
- **Status**: Complete
- **Spec**: [Canonical matrix](../spec.md)

## Role Definition

The CLI surface provides direct runtime access for operators and developers managing Corvus deployments.
It exposes the full capability surface of the runtime without gateway transport constraints.

## Mandatory Capabilities

### Core Agent Operations
- [ ] Full agent loop execution (`corvus agent`)
- [ ] Interactive chat mode
- [ ] Single-message mode (`-m "prompt"`)
- [ ] Session management (create, resume, end)

### Runtime Management
- [ ] Gateway startup and configuration
- [ ] Daemon mode for long-running operations
- [ ] Channel management (Telegram, Discord, Slack, WhatsApp, etc.)
- [ ] Heartbeat and periodic task scheduling
- [ ] Service lifecycle (install, start, stop, restart, uninstall)

### Configuration
- [ ] Runtime configuration editing
- [ ] Provider and model selection
- [ ] Memory backend configuration (SQLite, Markdown, Cerebro)
- [ ] Security policy configuration (autonomy levels, workspace scoping)
- [ ] Tunnel provider configuration

### Observability
- [ ] System status reporting
- [ ] Health diagnostics (`doctor`)
- [ ] Channel health checks
- [ ] Prometheus metrics endpoint
- [ ] Audit log viewing

### Developer Tools
- [ ] Integration registry management
- [ ] Skills loader management
- [ ] Onboarding wizard (`onboard`)
- [ ] Migration tools (OpenClaw compatibility)
- [ ] Hardware and peripheral detection

## Out-of-Scope

- [ ] Web UI rendering (handled by client surfaces)
- [ ] HTTP Gateway API (separate surface: dashboard)
- [ ] Mobile-specific features (handled by composeApp)

## Runtime-Only Boundary

The CLI has access to runtime-only capabilities that are excluded from all client surfaces:

| Capability | Access | Notes |
|-----------|--------|-------|
| Raw tool registry | Yes | Direct execution |
| Direct DB access | Yes | SQLite backend |
| Config hot-reload | Yes | `onboard --channels-only` |
| Credential vault | Yes | Encrypted storage |
| Audit log modification | Yes | Append operations |

## Transport Notes

- Direct process execution (no HTTP overhead)
- Full environment access (filesystem, network per policy)
- TTY interaction for interactive prompts
- Subprocess spawning for specialized sessions (code-specialist)

## Related Specifications

- [Agent Loop](../../agent-loop/spec.md) — Loop execution semantics
- [MCP Runtime](../../mcp-runtime/spec.md) — Tool registry
- [Dashboard](../../dashboard/spec.md) — Gateway API parity

## i18n Requirements

**Locale Tier**: Tier 3 — English-only
**Supported Locales**: en
**Parity Requirement**: None
**Glossary Compliance**: Recommended

### String Externalization

- The CLI MAY remain English-only for all user-facing strings
- The CLI SHOULD use canonical glossary terms: "pair" (not "link"), "session" (not
  "conversation"), "tool" (not "action")
- The CLI SHOULD NOT introduce new product terms without updating the canonical glossary
- No i18n infrastructure (key files, locale bundles) is required
- The CLI is exempt from key naming convention enforcement

### Parity Testing

- The CLI is exempt from parity testing — single locale only
- The CLI MAY be promoted to Tier 1/2 in a future change if operator localization is needed

### Design Tokens

- No design token requirements — CLI is a terminal application

### Scenarios

#### Scenario: CLI uses canonical terms in output

- GIVEN the CLI's onboarding wizard output
- WHEN the wizard references the device trust step
- THEN the output SHOULD use "pair" or "pairing" (not "link" or "connect")
- AND the terminology audit SHOULD warn (but not fail) on non-canonical terms

#### Scenario: CLI remains English-only

- GIVEN the CLI surface is classified as Tier 3
- WHEN a locale support review occurs
- THEN the CLI MAY remain English-only without failing any governance check
- AND the CLI MAY be promoted to Tier 1/2 in a future change if operator localization is needed

### References

- [i18n Governance Specification](../../i18n-governance/spec.md)
- [Canonical Glossary](../../../glossary/README.md)

## Change History

| Version | Date       | Changes                                                       |
|---------|------------|---------------------------------------------------------------|
| 1.1.0   | 2026-03-24 | Added i18n Requirements section (Tier 3 — English-only, #278) |
