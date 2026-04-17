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
1. ✅ Implemented iterative URL decoding (3 levels) in `is_path_allowed` and block residual `%` and `\`.
2. ✅ Enforced exact command matching in `is_segment_valid` to block `./cmd` bypasses.
3. ✅ Validated path-like command arguments against path traversal and forbidden path rules.
4. ✅ Expanded `contains_blocked_operators` to include backslash; glob chars blocked in `is_segment_valid`.

## 2025-05-20 - Flag-based Path Validation Hardening

**Learning:** The `SecurityPolicy` was vulnerable to path validation bypass when absolute paths or traversal sequences were passed within command flags (e.g., `grep --file=/etc/passwd` or `git -C/etc status`). The argument validation loop was only checking standalone arguments and not extracting values from flag assignments.
**Action:** Updated the argument validation loop in `is_segment_valid` to extract and validate potential paths from flags (`--key=value` and `-Cvalue`). This ensures that security checks (traversal, workspace bounds, forbidden paths) are applied consistently to all path-like inputs, even when embedded in flags.
