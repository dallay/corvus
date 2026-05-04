# Design: Slash Tools Listing

## Technical Approach

Implement `/tools` as one additional registry-backed slash command that reads from a narrow, precomputed tool snapshot instead of reaching into live runtime wiring or config mutation paths. The existing `pre_execution::evaluate_ingress(...) -> registry -> SessionCommandService` flow stays intact; this slice only adds a read-only metadata input and a command handler that formats the effective runtime tool inventory.

This directly follows the proposal and the existing slash-command registry spec: `/tools` remains transport-neutral at dispatch time, uses the shared handled-ingress seam, and preserves outward transport adaptation outside the core command contract.

## Architecture Decisions

### Decision: Pass a read-only tool snapshot through the existing slash service boundary

**Choice**: Extend slash execution inputs to include a compact effective-tool snapshot that is computed outside the command handler and passed into `SessionCommandService` / `evaluate_ingress(...)`.
**Alternatives considered**: Passing mutable config/service handles; giving slash handlers direct access to `Vec<Box<dyn Tool>>`; rebuilding runtime tools inside the handler.
**Rationale**: `/tools` only needs runtime metadata. A snapshot keeps the change small, avoids dyn-tool coupling inside slash handlers, and explicitly prevents this slice from becoming a mutation/config-management surface.

### Decision: Keep the slash-facing snapshot smaller than the full capability descriptor model

**Choice**: Use a slash-specific entry shape with only the fields `/tools` needs for output and tests: canonical tool name, description, source kind, and optional source/server label.
**Alternatives considered**: Passing the full `CapabilityRegistry`; passing raw `ToolSpec` values.
**Rationale**: `CapabilityRegistry` carries more structure than this command needs, while raw `ToolSpec` still exposes unused parameter-schema details. A compact snapshot is easier to format, easier to fixture in tests, and keeps the command boundary intentionally read-only.

### Decision: Preserve transport adaptation by adding structured success data, not new envelopes

**Choice**: Add a machine-readable success payload for tool listings while continuing to populate the existing human-readable `message` field consumed by CLI, gateway, webhook, and channels.
**Alternatives considered**: Only returning formatted text; introducing a new transport-specific response shape now.
**Rationale**: The slash-command spec requires a non-lossy internal outcome. Structured success data keeps future callers flexible without forcing transport churn in this slice.

## Data Flow

```text
startup/runtime wiring
  └─> effective tool composition (bootstrap / existing tools registry)
        └─> compact slash tool snapshot
              └─> pre_execution::evaluate_ingress(..., tool_snapshot, ...)
                    └─> SessionCommandService
                          └─> /tools handler
                                ├─> SessionCommandSuccess.message
                                └─> SessionCommandSuccess.data::ToolListing
```

Sequence for a handled `/tools` request:

```text
CLI / Gateway / Webhook / Channel
  -> build CommandContext
  -> pass precomputed tool snapshot into evaluate_ingress(...)
  -> registry resolves /tools
  -> ToolsHandler calls service.handle_tools()
  -> service sorts + formats effective tools
  -> shared adapter maps success/failure to existing transport wrappers
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/registry.rs` | Modify | Register `/tools` descriptor and add a small handler that delegates to `SessionCommandService::handle_tools()`. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modify | Accept a read-only tool snapshot, add `handle_tools()`, and centralize listing formatting/empty-state behavior. |
| `clients/agent-runtime/src/session_commands/types.rs` | Modify | Add slash tool snapshot/output structs and a `SessionCommandSuccessData::ToolListing` variant for machine-readable outcomes. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Thread the tool snapshot through `evaluate_ingress(...)` while preserving the existing handled-ingress classification flow. |
| `clients/agent-runtime/src/main.rs` | Modify | Pass the CLI-effective tool snapshot into handled-ingress evaluation for slash interception. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Store/pass a gateway-effective tool snapshot so `/tools` reflects the runtime-composed tool set seen by gateway ingress. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modify | Pass the same read-only snapshot through webhook handled-ingress evaluation. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Reuse the existing channel tool registry to build/pass the slash tool snapshot for channel ingress. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify (small helper) | Expose a minimal helper for deriving the effective slash tool snapshot from already-composed runtime tools where a caller does not already hold one. |

## Interfaces / Contracts

```rust
pub struct SessionCommandToolEntry {
    pub name: String,
    pub description: String,
    pub source_kind: SessionCommandToolSourceKind,
    pub source_label: Option<String>,
}

pub enum SessionCommandToolSourceKind {
    Native,
    McpTool,
    McpResource,
    McpPrompt,
}

pub enum SessionCommandSuccessData {
    None,
    Resumed { resumed_session_id: String },
    ResumableSessions { sessions: Vec<ResumableSessionEntry> },
    ToolListing { tools: Vec<SessionCommandToolEntry> },
}
```

`/tools` output expectations for this slice:

- Success MUST list the effective currently available tools for the running runtime/profile, not raw configured intent.
- Entries SHOULD be sorted deterministically by canonical tool name.
- Native tools SHOULD render with name + description only.
- MCP-derived entries SHOULD include enough source labeling to explain origin (for example server name and whether the entry is a tool/resource/prompt).
- Empty listings SHOULD return a successful handled command with an explicit empty-state message rather than a failure.
- `/tools` MUST remain argument-free; trailing args continue to fail via the existing `SlashCommandArgumentShape::None` validation path.

Representative human-readable message shape:

```text
Available tools (4):
- file_read — Read the contents of a file in the workspace
- file_write — Write contents to a file in the workspace
- mcp.docs.search — Search docs [mcp tool: docs]
- shell — Execute a shell command in the workspace directory
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `/tools` registry metadata and argument validation | Add registry tests for descriptor presence and invalid trailing args. |
| Unit | Tool snapshot formatting and empty-state behavior | Add `SessionCommandService` tests using fixed snapshot fixtures for native + MCP entries. |
| Integration | Shared ingress handling across transports | Extend handled-ingress tests/call-site tests so `/tools` is intercepted through `evaluate_ingress(...)` like existing slash commands. |
| Integration | Effective availability semantics | Use bootstrap/channel fixtures to verify profile-filtered and MCP-prefixed entries are what reach the snapshot. |
| E2E | Not required for this slice | Existing transport wrappers already consume `success.message`; focused runtime tests are sufficient here. |

## Migration / Rollout

No migration required.

Rollout is intentionally narrow:
- add `/tools` only;
- do not introduce `/tool enable`, `/tool disable`, `/mcp add`, `/mcp remove`, `/model`, `/provider`, or `/temperature` in this change;
- do not introduce config persistence, runtime refresh, or generic command-family plumbing beyond what `/tools` needs.

## Rollback

Rollback is a straight code revert:
- remove the `/tools` registry binding and handler;
- remove the read-only tool snapshot plumbing from slash ingress;
- remove the `ToolListing` success payload and related tests.

Because this slice adds no persisted state and no mutation behavior, rollback does not require config migration or data cleanup.

## Open Questions

- [ ] None for this slice; dynamic mutation/refresh semantics are intentionally deferred with the out-of-scope command families.
