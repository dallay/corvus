---
name: rust
description: >
  General Rust engineering guidance grounded in Rust Book best practices.
  Trigger: When creating, modifying, or reviewing Rust code, especially around
  ownership, error handling, traits, modules, testing, async boundaries, or
  performance-sensitive logic.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Rust

General-purpose Rust guidance centered on correctness, clarity, safety, and maintainability.
Use this as the base skill before layering repo-specific rules.

## When to Use

- Any Rust implementation or review task
- Ownership/borrowing design questions
- `Result`/error handling decisions
- Trait and module design
- Async boundary decisions
- Test strategy and performance-sensitive changes

## Critical Patterns

### Prefer explicit ownership over accidental cloning

- Borrow first: `&str`, `&Path`, `&[T]`, `&T`
- Move ownership only when the callee truly needs it
- Clone only at real lifetime or concurrency boundaries
- Reach for `Arc<T>` only when shared ownership is required

See [references/ownership-and-borrowing.md](references/ownership-and-borrowing.md).

### Use `Result` for fallible behavior

- Model expected failures with `Result<T, E>`
- Prefer `thiserror` for domain errors in libraries/services
- Add context at orchestration boundaries
- Avoid `unwrap()` / `expect()` in production paths unless failure is truly impossible

See [references/error-handling.md](references/error-handling.md).

### Design with traits and small modules

- Keep modules focused on one concern
- Use traits for replaceable behavior and testability
- Hide internals behind small public APIs
- Prefer composition over giant god-structs and giant enums

### Test the contract, not the implementation trivia

- Add regression tests for bugs
- Keep unit tests close to the module
- Use integration tests when behavior crosses module boundaries
- Benchmark only after correctness is covered

See [references/testing-and-validation.md](references/testing-and-validation.md).

## Decision Table

| Problem | Prefer |
|---|---|
| Need read-only access | borrow |
| Need domain-specific failure | typed error enum |
| Swappable behavior | trait + small impl |
| Bug fix | failing regression test first |
| Hot path change | measure before optimizing |

## Code Examples

```rust
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
```

## Commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Resources

- **References**: see [`references/`](references/)
- **Assets**: see [`assets/`](assets/) for starter templates
