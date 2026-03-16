
## 2025-05-15 - Robust Path Hardening and CI Validation

**Learning:** Literal string checks for encoded patterns like `%2e%2e` are insufficient as they don't account for multi-level encoding or other variations. A more robust approach is to percent-decode the path before validation. Additionally, the lack of Rust validation in CI was identified as a risk for security-sensitive logic.

**Action:**
1. Implemented percent-decoding in `is_path_allowed` using the `urlencoding` crate to catch all variations of encoded traversal markers.
2. Enabled Rust validation (fmt, clippy, tests) in the PR workflow by passing `-PenableRustTasks=true` to Gradle.
3. Added comprehensive regression tests for raw and encoded traversal patterns.

## 2025-05-15 - Consistent Percent-Decoding in Path Validation

**Learning:** Partial percent-decoding in path validation (only for traversal detection) is insufficient. If the path is later normalized or used by other layers, encoded absolute paths (e.g., `%2fetc%2fpasswd`) or encoded forbidden prefixes (e.g., `%7e/.ssh`) could bypass the `SecurityPolicy` check.

**Action:**
1. Consistently use the percent-decoded path (`policy_path`) for all security checks within `is_path_allowed`, including tilde expansion, absolute path detection, and forbidden path matching.
2. Added a specific check for decoded absolute paths starting with `/` to enforce `workspace_only` constraints on encoded inputs.
3. Added regression tests for encoded absolute and tilde paths.
