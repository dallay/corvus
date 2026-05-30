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

## 2025-05-30 - Hardening Verb Detection and Blocking Execution Redirection

**Learning:**
1. `is_medium_risk_command` was vulnerable to bypass when subcommands were preceded by global flags (e.g., `git -C path commit`). Only checking the first argument was insufficient.
2. `git --exec-path` and `find -execdir`/`-okdir` provided vectors for unauthorized code execution or redirection that were not fully covered by existing blocked lists.
3. `iterative_url_decode` was incurring unnecessary overhead on a hot path by attempting decoding even when no encoding (`%`) was present.

**Action:**
1. ✅ Refactored `is_medium_risk_command` to use `args.iter().any()` for verb detection, ensuring subcommands are flagged regardless of their position relative to flags.
2. ✅ Expanded `is_args_safe` blocked list for `git` (`--exec-path`) and `find` (`-execdir`, `-okdir`).
3. ✅ Added early return to `iterative_url_decode` when input contains no `%` character, reducing allocations and CPU cycles.
