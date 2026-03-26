---
name: rust-performance-runtime
description: >
  Performance and footprint guidance for Corvus Rust runtime. Trigger: When
  optimizing clients/agent-runtime hot paths, startup cost, async execution,
  allocations, dependency choices, release size, or benchmark-sensitive code.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Rust Performance Runtime

Performance guidance for `clients/agent-runtime` aligned with this repo's goals: high performance,
high efficiency, small release binaries, and predictable async behavior. Prefer measured
improvements over cleverness.

## When to Use

- Optimizing hot paths in `src/agent/**`, `src/providers/**`, `src/memory/**`, `src/tools/**`
- Reviewing startup cost, task spawning, allocation patterns, or clone-heavy code
- Changing `Cargo.toml` dependencies or features
- Working on benches under `clients/agent-runtime/benches/**`
- Evaluating release profile, binary size, or optional feature boundaries

## Critical Patterns

### Measure first, then optimize

- Start with the smallest reproducible hotspot.
- Prefer targeted benchmarks or focused timing before changing architecture.
- Do not trade maintainability for tiny theoretical wins.
- Keep the optimization reversible.

### Respect repo-level size and dependency policy

Current runtime choices already optimize for size:
- `tokio` with reduced features
- `reqwest` with `default-features = false`
- `panic = "abort"`
- `opt-level = "z"`, `strip = true`, `lto`
- optional features for heavier capabilities like browser, probe, pdf, landlock

**Rule:** do not add a heavy crate for a convenience helper if the standard library or an existing
crate can handle it.

### Avoid unnecessary allocation and cloning

- Prefer borrowing over cloning.
- Avoid `String` creation if `&str` is enough.
- Reuse buffers in loops when practical.
- Watch for repeated `serde_json::Value` reshaping if a typed struct is cleaner.
- In async code, clone `Arc<T>` intentionally; avoid hidden data duplication.

```rust
fn classify(command: &str) -> bool {
    matches!(command, "run" | "check" | "doctor")
}
```

Better than allocating a normalized `String` unless the normalization is required.

### Async performance rules

- Do not block the Tokio runtime with expensive sync I/O or CPU-heavy work.
- Use `spawn_blocking` only for truly blocking work and keep the closure small.
- Avoid unbounded task spawning in loops.
- Prefer structured orchestration over detached background tasks.
- Apply timeouts and backpressure where external systems may stall.

### Optimize the startup path

`src/main.rs`, config load, provider setup, and command routing should stay lean.

- No expensive discovery during basic command parsing unless required.
- Delay heavy setup until the command actually needs it.
- Optional integrations should remain behind features or lazy construction.

## Decision Table

### What optimization is appropriate?

| Symptom | First move |
|---|---|
| Slow startup | profile init path, defer heavy setup |
| High memory or clone churn | inspect ownership and borrowed APIs |
| Runtime stalls | find blocking work inside async path |
| Large binary | inspect dependency/features before code tricks |
| Slow repeated operation | micro-benchmark the exact function |

## Code Examples

### Keep heavy work off the async core

```rust
let parsed = tokio::task::spawn_blocking(move || parse_large_document(bytes))
    .await
    .map_err(RuntimeError::Join)??;
```

Use this only when the work is truly blocking or CPU-heavy and cannot be cheaply streamed.

### Prefer typed data over repeated dynamic access

```rust
#[derive(Deserialize)]
struct WebhookPayload {
    event: String,
    channel_id: String,
}
```

This is usually faster and safer than repeatedly traversing `serde_json::Value` in hot paths.

## Commands

```bash
# lint for common performance/code quality issues
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings

# run runtime tests
cargo test --manifest-path clients/agent-runtime/Cargo.toml

# run benchmarks
cargo bench --manifest-path clients/agent-runtime/Cargo.toml

# release build aligned with repo profiles
cargo build --manifest-path clients/agent-runtime/Cargo.toml --profile release-fast
```

## Review Checklist

- Was the hotspot measured or at least isolated before optimizing?
- Did the change reduce clones, allocations, or blocking without obscuring intent?
- Did it preserve the repo's small-binary and optional-feature strategy?
- Are async boundaries still structured and safe?
- Is there a benchmark, targeted test, or concrete validation for the improvement?
