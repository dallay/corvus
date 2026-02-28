---
name: rust
description: >
  Rust conventions for safe refactoring, testing, and Cargo configuration.
  Trigger: When working with Rust files, Cargo.toml, or Rust module quality improvements.
license: Apache-2.0
allowed-tools: Read, Edit, Write, Glob, Grep, Bash
metadata:
  author: "@yacosta738"
  version: "1.1"
---

# Rust Skill

Conventions for writing safe, maintainable, and testable Rust code in this repository.

## When to Use

- Creating or modifying `**/*.rs` files
- Working in `clients/agent-runtime/**`
- Updating `Cargo.toml` dependencies, features, or toolchain settings
- Refactoring Rust code with regression safety requirements
- Adding or improving Rust unit/integration tests

## Principles

- Security first: validate input, avoid unsafe defaults, minimize exposed surface
- Performance second: prefer clear algorithms, measure before micro-optimizing
- Determinism: tests should not depend on wall-clock timing or network flakiness
- Minimal changes: smallest possible diff that solves the problem safely

## Critical Patterns

### 1. Error Handling Without Panic Paths

Prefer explicit error propagation over `unwrap`/`expect` in production code.

```rust
// ❌ Avoid in production paths
let cfg = read_config(path).unwrap();

// ✅ Prefer explicit propagation
let cfg = read_config(path)?;

// ✅ Add context in app/service layers
let cfg = read_config(path).context("failed to read runtime config")?;
```

Use `thiserror` for domain/library errors and `anyhow` for application boundaries.

### 2. Keep Public APIs Typed and Explicit

```rust
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid relay url: {0}")]
    InvalidRelayUrl(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn load_runtime(path: &std::path::Path) -> Result<RuntimeConfig, RuntimeError> {
    // ...
    Ok(RuntimeConfig::default())
}
```

### 3. Tests First for Behavior Changes

- Add a failing test before fixing a bug or adding behavior
- Cover happy path, boundary conditions, and failure path
- Prefer table-driven tests when permutations are important

```rust
#[test]
fn parses_valid_profile() {
    let input = r#"name=alice"#;
    let profile = parse_profile(input).expect("valid profile should parse");
    assert_eq!(profile.name, "alice");
}
```

### 4. Safer Refactoring Workflow

1. Add/adjust tests to lock behavior
2. Refactor internals with unchanged signatures first
3. Run targeted tests, then full module tests
4. Run lint and format checks

### 5. Cargo Hygiene

- Prefer minimal dependency set and pinned compatible versions
- Use feature flags to isolate optional capabilities
- Keep `Cargo.toml` changes justified and scoped

## Anti-Patterns

- `unwrap()` / `expect()` in production paths
- Silent error swallowing (`let _ = ...`) without intent
- Hidden global mutable state for runtime behavior
- Unbounded task spawning without backpressure controls
- Adding dependencies without clear need

## Verification Commands

```bash
# Fast feedback
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings

# Project-integrated tasks (if available)
./gradlew :agent-runtime:cargoBuild
./gradlew :agent-runtime:cargoCheck
./gradlew :agent-runtime:cargoTest
```

## Common Workflows

### Add a Bug Fix Safely

1. Reproduce with a failing test
2. Apply minimal code fix
3. Validate with targeted test and full Rust test suite
4. Run fmt/clippy and verify no regressions

### Improve Error Model

1. Introduce typed error variants (`thiserror`)
2. Convert call sites to `?` and contextual errors where needed
3. Remove panic paths from production code

## Related Skills

- `rust-async-patterns` for Tokio/concurrency/channel patterns
- `tdd` for strict Red -> Green -> Refactor workflow
