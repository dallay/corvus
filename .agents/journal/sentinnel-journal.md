# 2025-05-15 - Hardening Path and Command Validation

**Learning:** `SecurityPolicy` checks for path traversal and shell operators were bypassable using percent-encoding (e.g., `%2e%2e` for `..`) and missing common redirection operators like `<`.
**Action:** Always decode percent-encoded strings before performing security validation on paths. Explicitly block all shell redirection operators (`<`, `>`) to prevent unauthorized file access or exfiltration.

## 2025-05-16 - Comprehensive SecurityPolicy Hardening

**Learning:** The `SecurityPolicy` had several gaps:
1. `is_path_allowed` was vulnerable to multi-layer URL encoding bypasses (e.g., `%252e%252e`).
2. `is_segment_valid` allowed path-based command execution (e.g., `./ls`), enabling workspace binary shadowing.
3. Command arguments were not validated as paths, allowing `cat /etc/passwd` if `cat` was in the allowlist.
4. Shell metacharacters like glob symbols (`*`, `?`) and backslashes (`\`) were not blocked, enabling potential obfuscation and escaping.

**Action:**
1. Implement iterative URL decoding (3 levels) in `is_path_allowed` and block residual `%` and `\`.
2. Enforce exact command matching in `is_segment_valid` to block `./cmd` bypasses.
3. Validate all command arguments against path traversal and forbidden path rules.
4. Expand `contains_blocked_operators` to include glob and escape characters.
