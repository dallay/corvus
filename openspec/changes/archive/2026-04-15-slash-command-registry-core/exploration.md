# Exploration — Slash Command Registry Core

## Executive Summary

Current slash-session handling is centralized only at the pre-execution seam, not at a true command registry. `clients/agent-runtime/src/session_commands/` contains a strict parser (`parser.rs`), a thin hard-coded "registry" with an unused supported-command list plus match-based dispatch (`registry.rs`), and a memory-backed service (`service.rs`) for `/tldr`, `/compact`, `/suspend`, and `/resume`.

The main integration seam is `pre_execution::evaluate_ingress(...)`, which is invoked independently by CLI, gateway HTTP early-response and SSE paths, webhook dispatcher, and channel ingress. So command interception is behaviorally centralized, but entrypoint wiring is duplicated.

Existing registry patterns elsewhere are stronger than session commands: `capabilities/registry.rs` is a real validated registry with register/get/iter semantics, while channel/integration registries use static entry tables. Session commands do not yet have that level of abstraction.

## Key Findings

- Session command dispatch is still hard-coded.
- Entry-point interception is common in concept but duplicated in wiring.
- Caller identity semantics differ by surface and must be preserved.
- Slash-session behavior is tightly coupled to memory backend capabilities and should not be mixed into registry-core concerns.

## Recommended Next Step

Proceed to proposal for a central slash command registry that owns command metadata, parse/lookup/dispatch contracts, and entrypoint integration rules while preserving the current early-short-circuit seam in `pre_execution::evaluate_ingress(...)`.

## Risks

- Registry refactor can break transport parity if one ingress path bypasses the seam.
- Existing `session_commands/registry.rs` is not truly extensible.
- `/resume` authorization semantics vary by surface.
- Backend capability rules must remain outside registry-core.
