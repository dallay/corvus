---
name: rust-security-runtime
description: >
  Security-first Rust guidance for Corvus high-risk runtime surfaces. Trigger:
  When modifying clients/agent-runtime security, gateway, auth, tool execution,
  sandboxing, policy enforcement, secrets handling, or external trust boundaries.
license: Apache-2.0
metadata:
  author: generic-author
  version: "1.0"
---

# Rust Security Runtime

Specialized guidance for the high-risk Rust surfaces in Corvus. Applies Rust best practices from
the Book, but with stricter repo rules: fail closed, validate at boundaries, never leak secrets,
and never widen permissions without an explicit policy reason.

## When to Use

- Editing `clients/agent-runtime/src/security/**`
- Editing `clients/agent-runtime/src/gateway/**`
- Editing `clients/agent-runtime/src/auth/**`
- Editing tool execution in `clients/agent-runtime/src/tools/**`
- Touching webhook verification, pairing, secret storage, sandboxing, or policy checks
- Reviewing code that crosses filesystem, network, process, or credential boundaries

## Critical Patterns

### Fail closed, not open

If a policy check, auth check, sandbox requirement, or signature verification is uncertain,
**deny the action**.

- Missing config for a security-critical feature should produce an explicit error.
- Unsupported sandbox/policy conditions should stop execution, not silently continue.
- If verification data is malformed, treat it as untrusted input and reject it.

### Validate all external input at the boundary

Validate before the data reaches business logic.

| Boundary | Typical risk | Expected defense |
|---|---|---|
| HTTP/webhook input | spoofing, malformed payloads | parse + verify signature + size limits |
| Tool input | command/path injection | strict schema + allowlists + normalization |
| File paths | traversal / symlink escape | canonicalization + policy check |
| Secrets/tokens | accidental disclosure | redaction + zeroization + no debug dumps |
| Child process / shell | privilege escalation | explicit approvals + sandbox + minimal args |

### No secret leakage, ever

- Never log API keys, bearer tokens, pairing codes, webhook secrets, or raw credential material.
- Redact sensitive values before logging errors or request context.
- Avoid `Debug` output on structs carrying secrets unless fields are intentionally redacted.
- Prefer constant-time comparisons where secret/token equality matters.
- Zero sensitive plaintext buffers when practical.

### Rust error handling for security paths

- Return explicit typed errors for auth/policy/sandbox failures.
- Preserve enough context for operators without exposing sensitive payloads.
- Avoid generic `invalid request` if it hides useful operator action; avoid detailed attacker hints.
- No `unwrap()` / `expect()` on request headers, secret material, env vars, or policy state.

```rust
pub fn verify_signature(header: &str, body: &[u8], secret: &SecretKey) -> Result<(), AuthError> {
    let provided = parse_signature_header(header).map_err(AuthError::MalformedSignature)?;
    let expected = compute_signature(body, secret);

    if subtle::ConstantTimeEq::ct_eq(provided.as_ref(), expected.as_ref()).into() {
        Ok(())
    } else {
        Err(AuthError::SignatureMismatch)
    }
}
```

### Keep permission expansion explicit

Any change that broadens access to:
- filesystem paths
- network egress
- tool execution
- sandbox capabilities
- webhook trust sources

must be deliberate, documented, and tested.

## Security Decision Table

### What should happen on failure?

| Situation | Correct default |
|---|---|
| Signature missing or malformed | reject request |
| Approval/policy status unavailable | deny action |
| Path cannot be normalized safely | reject path |
| Sandbox unavailable for required command | fail with explicit error |
| Secret store read/decrypt fails | stop and return sanitized error |
| Tool input exceeds schema/allowlist | reject input |

## Code Examples

### Sanitize before logging

```rust
match load_secret("OPENAI_API_KEY") {
    Ok(_) => tracing::debug!("secret loaded successfully"),
    Err(error) => tracing::warn!(error = %error, "failed to load configured secret"),
}
```

### Guard path access

```rust
pub fn validate_allowed_path(path: &Path, root: &Path) -> Result<PathBuf, SecurityError> {
    let canonical_root = root.canonicalize().map_err(SecurityError::PathResolutionFailed)?;
    let canonical = path.canonicalize().map_err(SecurityError::PathResolutionFailed)?;

    if canonical.starts_with(&canonical_root) {
        Ok(canonical)
    } else {
        Err(SecurityError::PathOutsideAllowedRoot)
    }
}
```

## Commands

```bash
# lint and tests for security-sensitive runtime code
cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path clients/agent-runtime/Cargo.toml

# focused security regression example
cargo test --manifest-path clients/agent-runtime/Cargo.toml whatsapp_webhook_security
```

## Review Checklist

- Did the change fail closed on every uncertain security branch?
- Are all boundary inputs validated before use?
- Could any log, error, or debug output leak secrets?
- Did the change widen any permission or trust boundary?
- Is there a regression test for the security behavior?
