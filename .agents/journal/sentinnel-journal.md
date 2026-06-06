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

## 2025-05-24 - Unified URL Decoding across SecurityPolicy entry points

**Learning:** URL encoding was being used to bypass shell operator checks and risk classification in `is_command_allowed` and `command_risk_level`. While `is_path_allowed` was already decoding inputs, other entry points were vulnerable to encoded redirection (`%3e`), background (`%26`), and subshell (`%24%28`) operators.
**Action:** Centralized iterative URL decoding into a helper function and applied it consistently at the start of `is_path_allowed`, `is_command_allowed`, and `command_risk_level`. This ensures that all security filters operate on normalized input, preventing obfuscation-based bypasses.

## 2025-05-28 - Hardening SecurityPolicy against Quote-based Bypasses

**Learning:** `SecurityPolicy` was vulnerable to path validation bypass using nested or partial quotes (e.g., `"/etc"/passwd`, `""/etc/passwd""`). The previous `strip_matching_quotes` only removed outer quotes, leaving internal quotes to disrupt prefix-based forbidden path and absolute path checks.
**Action:** Implemented `strip_all_quotes` to normalize command arguments and paths by removing all single and double quotes. In `is_path_allowed`, `iterative_url_decode` is still applied before quote stripping so encoded quotes are exposed before later processing.

## 2025-06-01 - Optimizing SecurityPolicy and Fixing Case-Sensitive Bypasses

**Learning:** Mandatory global lowercasing of command arguments for security validation caused both false positives (e.g., blocking `git -C` because it was treated as `git -c`) and security gaps (e.g., missing joined flags like `git -cconfig=value`). It also introduced unnecessary $O(N)$ string allocations for every command execution.
**Action:** Transitioned to passing case-preserving arguments to `is_args_safe`. Implemented case-insensitive checks for subcommands and case-sensitive prefix matching for sensitive flags (like `git -c`). This improves performance by avoiding allocations in the common path and increases security by correctly identifying complex flag forms.
