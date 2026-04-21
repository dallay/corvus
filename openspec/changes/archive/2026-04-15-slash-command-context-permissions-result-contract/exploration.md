# Exploration — Slash Command Context, Permissions, and Result Contract

## Executive Summary

Current slash commands are centralized in `clients/agent-runtime/src/session_commands/{registry,service,types}.rs` and routed through `pre_execution::evaluate_ingress(...)` from CLI, gateway, webhook dispatcher, and channels.

Today the command context is only `{ session_id, caller_token_hash }`, descriptor metadata is string-tag based (`capability_tags`, `permission_tags`, `backend_tags`), and the result/error contract is not first-class across transports because `SessionCommandError` is flattened in pre-execution into a generic `SessionCommandResult` plus `success: bool`.

Transport-specific identity currently enters in the gateway via bearer-token SHA-256 hashes, in channels via `sha256("{channel}:{sender}")`, and in CLI as `None`.

## Key Findings

- `CommandContext` is too narrow for a shared command execution contract.
- Descriptor requirement metadata is descriptive but not typed enough for downstream guarantees.
- Result and error handling are lossy across the ingress seam.
- Transport response envelopes already diverge across CLI, gateway, channels, and streaming.
- `/resume <target>` authorization appears under-enforced because the service requires a caller hash but does not compare it to target-session ownership.

## Recommended Next Step

Proceed to proposal for a typed slash command execution contract that standardizes command context, typed requirement metadata, and non-lossy result/error outcomes while preserving per-surface identity semantics and keeping backend/auth policy out of the registry core.

## Risks

- Tightening permission and result contracts can expose gaps in current `/resume` ownership enforcement.
- Over-design here could accidentally absorb transport integration work that belongs to #541.
- Result-contract changes will touch the shared ingress seam and can regress HTTP/SSE/channel parity if done carelessly.
