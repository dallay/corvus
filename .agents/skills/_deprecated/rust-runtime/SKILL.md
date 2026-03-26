---
name: rust-runtime
description: >
  Rust implementation guidance for Corvus agent-runtime. Trigger: When creating,
  modifying, or reviewing Rust code under clients/agent-runtime, especially trait
  implementations, module wiring, CLI/runtime behavior, or integration boundaries.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Rust Runtime

Repo-specific Rust guidance for `clients/agent-runtime` with Rust Book fundamentals applied to
Corvus architecture: ownership, explicit error handling, small modules, trait-driven extension,
focused tests, and safe defaults.

## When to Use

- Creating or modifying Rust code in `clients/agent-runtime/src/**`
- Adding a new provider, channel, tool, memory backend, observer, or runtime adapter
- Changing CLI routing in `src/main.rs` or exported behavior in `src/lib.rs`
- Wiring a new implementation into a factory `mod.rs`
- Reviewing Rust changes for maintainability, correctness, or architecture fit

## Critical Patterns

### Start with the runtime architecture

Treat Corvus as a trait-driven runtime, not a pile of handlers.

| Capability | Primary contract | Typical implementation area |
|---|---|---|
| Model providers | `src/providers/traits.rs` | `src/providers/*.rs` |
| Channels | `src/channels/traits.rs` | `src/channels/*.rs` |
| Tools | `src/tools/traits.rs` | `src/tools/*.rs` |
| Memory | `src/memory/traits.rs` | `src/memory/*.rs` |
| Observability | `src/observability/traits.rs` | `src/observability/*.rs` |
| Runtime adapters | `src/runtime/traits.rs` | `src/runtime/*.rs` |

**Rule:** prefer implementing an existing trait and registering it in the relevant `mod.rs`
before inventing a new abstraction.

### Follow Rust Book error handling, not panic-driven flow

- Use `Result<T, E>` for fallible runtime behavior.
- Prefer domain errors with `thiserror` for library/runtime boundaries.
- Use `anyhow` mainly for top-level orchestration, CLI plumbing, or test helpers.
- In production paths: no `unwrap()`, no `expect()`, no hidden panics.
- Convert external errors into actionable messages with context.

```rust
pub async fn load_profile(path: &Path) -> Result<Profile, ConfigError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    toml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}
```

### Keep ownership and data flow simple

- Borrow when possible, clone only when the lifetime boundary requires it.
- Pass `&str`, `&Path`, `&[T]` at boundaries unless ownership is needed.
- Prefer `Arc<T>` only for real shared ownership across async tasks.
- Avoid premature interior mutability; if you need mutation, justify it.
- Keep structs small and focused; split giant config/runtime structs by concern.

### Module discipline

- One concern per file/module.
- Put extension wiring in `mod.rs`, not scattered across call sites.
- Preserve public contracts unless the change is intentional and documented.
- Do not mix refactors with feature work unless required to make the patch safe.

### TDD by default

- Bug fix: add a regression test first when practical.
- New behavior: add a focused unit or integration test before broad refactors.
- Keep tests close to the risk:
  - unit tests near module code with `#[cfg(test)]`
  - cross-module/integration checks in `clients/agent-runtime/tests/*.rs`

## Decision Table

### Where should this Rust change go?

| Change type | Best location |
|---|---|
| New provider/channel/tool/memory backend | matching `src/<area>/` module + trait impl |
| New factory registration | matching `src/<area>/mod.rs` |
| Shared helper only used by one module tree | local module helper |
| Cross-cutting helper with broad reuse | small focused module, not `util.rs` by default |
| End-to-end contract validation | `clients/agent-runtime/tests/*.rs` |

## Code Examples

### Adding a trait implementation cleanly

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(&self, input: ToolInput) -> Result<ToolResult, ToolError>;
}

pub struct UrlSafetyTool {
    policy: Arc<UrlPolicy>,
}

#[async_trait]
impl Tool for UrlSafetyTool {
    async fn execute(&self, input: ToolInput) -> Result<ToolResult, ToolError> {
        let url = parse_url(&input)?;
        self.policy.validate(&url)?;
        Ok(ToolResult::ok())
    }
}
```

### Register in one place

```rust
pub fn register_tools(registry: &mut ToolRegistry) {
    registry.register("url_safety", Arc::new(UrlSafetyTool::new()));
}
```

## Commands

```bash
# format check
cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check

# lint with warnings denied
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings

# full test suite for runtime
cargo test --manifest-path clients/agent-runtime/Cargo.toml

# targeted test
cargo test --manifest-path clients/agent-runtime/Cargo.toml module_name::tests::test_name
```

## Review Checklist

- Does the change fit an existing trait or extension point?
- Are errors explicit and panic-free in production paths?
- Is ownership minimal and cloning justified?
- Is registration centralized in the correct `mod.rs`?
- Did the smallest relevant test actually run?
