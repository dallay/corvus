---
name: corvus-rust-runtime
description: >
  Corvus-specific Rust runtime guidance. Trigger: When working in
  clients/agent-runtime on providers, channels, tools, security, memory,
  gateway, runtime adapters, or other trait-driven runtime boundaries.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Corvus Rust Runtime

Repo-specific guidance for `clients/agent-runtime`. Load this together with the base `rust` skill
when changes depend on Corvus architecture, security boundaries, runtime contracts, or validation
commands.

## When to Use

- Any change under `clients/agent-runtime/src/**`
- Adding a provider, channel, tool, memory backend, observer, or runtime adapter
- Touching `security/`, `gateway/`, `auth/`, `tools/`, or `memory/`
- Wiring implementations in a factory `mod.rs`
- Reviewing Rust changes for repo fit, safety, and validation scope

## Critical Patterns

### Think in extension points first

Corvus is trait-driven. Before creating a new abstraction, check the existing contracts.

| Capability | Contract | Usual location |
|---|---|---|
| Provider | `src/providers/traits.rs` | `src/providers/*.rs` |
| Channel | `src/channels/traits.rs` | `src/channels/*.rs` |
| Tool | `src/tools/traits.rs` | `src/tools/*.rs` |
| Memory | `src/memory/traits.rs` | `src/memory/*.rs` |
| Observer | `src/observability/traits.rs` | `src/observability/*.rs` |
| Runtime adapter | `src/runtime/traits.rs` | `src/runtime/*.rs` |

See [references/runtime-architecture.md](references/runtime-architecture.md).

### Security is a default, not an add-on

For `security/`, `gateway/`, `auth/`, and `tools/`:
- fail closed
- validate inputs at the boundary
- never log secrets or raw tokens
- never widen access or policy scope silently

See [references/security-boundaries.md](references/security-boundaries.md).

### Repo-specific engineering constraints

- No `unwrap()` / `expect()` in production paths unless failure is truly impossible
- Prefer minimal dependencies; binary size matters
- Keep patches local and focused
- Avoid blocking work in async paths
- Preserve CLI/API contracts unless intentionally changing them
- Add regression tests first for bugs when practical

### Validation by risk

Run the smallest relevant checks before calling work done.

See [references/validation-commands.md](references/validation-commands.md).

## Decision Table

| Change | Preferred move |
|---|---|
| New integration type already matches a trait | implement trait + register in `mod.rs` |
| Security/policy uncertainty | deny/fail closed |
| Cross-module behavior change | add integration or regression test |
| Heavy new dependency | challenge it before adding |
| Runtime hot path touched | verify async/blocking/clone impact |

## Code Examples

### Register in the right factory

```rust
pub fn register_tools(registry: &mut ToolRegistry) {
    registry.register("my_tool", std::sync::Arc::new(MyTool::new()));
}
```

### Return typed runtime errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("input validation failed: {reason}")]
    InvalidInput { reason: String },
}
```

## Commands

```bash
make rust-fmt
make rust-clippy
make rust-test

cargo test --manifest-path clients/agent-runtime/Cargo.toml
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings
```

## Resources

- **References**: see [`references/`](references/)
- **Assets**: see [`assets/`](assets/) for Corvus starter templates
