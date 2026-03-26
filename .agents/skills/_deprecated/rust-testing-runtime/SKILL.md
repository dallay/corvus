---
name: rust-testing-runtime
description: >
  Testing guidance for Corvus Rust runtime. Trigger: When adding or fixing Rust
  behavior in clients/agent-runtime, especially regression tests, integration
  tests, async tests, trait wiring validation, or benchmark-backed changes.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Rust Testing Runtime

Testing guidance for `clients/agent-runtime` with a repo-first approach: write the smallest test
that proves behavior, prefer regression tests for bugs, keep async/runtime tests deterministic, and
validate the real integration boundary when the risk crosses modules.

## When to Use

- Fixing bugs in `clients/agent-runtime/src/**`
- Adding new Rust behavior that changes runtime contracts
- Writing regression tests for security, gateway, tool, provider, or memory flows
- Creating integration tests in `clients/agent-runtime/tests/*.rs`
- Verifying trait wiring and factory registration in providers/channels/tools/memory
- Adding or updating benchmarks in `clients/agent-runtime/benches/**`

## Critical Patterns

### TDD default: red -> green -> refactor

- Bug fix: add a failing regression test first when practical.
- New feature: add the smallest focused test that proves the expected behavior.
- Refactor: preserve behavior with existing tests before changing structure.

### Put the test where the risk lives

| Change type | Best test location |
|---|---|
| Pure logic in one module | local `#[cfg(test)]` unit test |
| Factory registration / module wiring | local module test or focused integration test |
| Cross-module contract | `clients/agent-runtime/tests/*.rs` |
| CLI / runtime entry behavior | integration test |
| Security boundary / webhook / policy behavior | regression-focused integration test |
| Hot-path optimization | benchmark in `benches/` plus correctness test |

### Keep tests deterministic

- Avoid real network calls unless the test is explicitly integration/e2e and isolated.
- Prefer temp dirs, fakes, local fixtures, and test helpers over ambient machine state.
- Avoid time-based flakiness; inject clocks or use controlled timeouts where possible.
- In async tests, ensure all spawned work is awaited or bounded.
- Do not rely on test order.

### Assert behavior, not implementation trivia

Good tests prove:
- returned result and error shape
- side effects at module boundaries
- policy/security decisions
- wiring/registration behavior

Bad tests lock in:
- private helper internals
- incidental log strings
- exact formatting unless formatting is the contract

### Regression tests must name the failure clearly

Prefer descriptive names that explain the broken behavior.

```rust
#[tokio::test]
async fn webhook_rejects_request_with_invalid_signature() {
    // arrange
    // act
    // assert
}
```

## Async Testing Patterns

- Use `#[tokio::test]` for async runtime behavior.
- Keep setup lean; avoid spinning full subsystems if a focused fake works.
- Bound waiting with explicit timeouts for async coordination.
- Prefer channels/fakes over sleeping.

```rust
#[tokio::test]
async fn provider_returns_error_when_profile_is_missing() {
    let result = load_provider_profile("missing-profile").await;

    assert!(matches!(result, Err(ProviderError::ProfileNotFound { .. })));
}
```

## Benchmark Guidance

Use a benchmark only when performance matters and correctness is already covered by tests.

- Benchmark one hotspot at a time.
- Keep input representative, not synthetic nonsense.
- Do not use benchmarks as the only validation for behavior changes.

## Commands

```bash
# full runtime tests
cargo test --manifest-path clients/agent-runtime/Cargo.toml

# one specific test by name
cargo test --manifest-path clients/agent-runtime/Cargo.toml webhook_rejects_request_with_invalid_signature

# one module-style test path
cargo test --manifest-path clients/agent-runtime/Cargo.toml module_name::tests::test_name

# benchmarks
cargo bench --manifest-path clients/agent-runtime/Cargo.toml
```

## Review Checklist

- Does a failing test exist first for the bug or changed behavior?
- Has the test been located at the right boundary level?
- Are tests deterministic and free from unnecessary sleeps or external dependencies?
- Do they assert contract-level behavior instead of internals?
- For performance work, do both a correctness test and meaningful benchmark exist?
