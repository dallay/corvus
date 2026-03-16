
## 2025-05-15 - Robust Path Hardening and CI Validation

**Learning:** Literal string checks for encoded patterns like `%2e%2e` are insufficient as they don't account for multi-level encoding or other variations. A more robust approach is to percent-decode the path before validation. Additionally, the lack of Rust validation in CI was identified as a risk for security-sensitive logic.

**Action:**
1. Implemented percent-decoding in `is_path_allowed` using the `urlencoding` crate to catch all variations of encoded traversal markers.
2. Enabled Rust validation (fmt, clippy, tests) in the PR workflow by passing `-PenableRustTasks=true` to Gradle.
3. Added comprehensive regression tests for raw and encoded traversal patterns.
