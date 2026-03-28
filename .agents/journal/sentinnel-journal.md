## 2025-05-15 - Hardening Path and Command Validation

**Learning:** `SecurityPolicy` checks for path traversal and shell operators were bypassable using percent-encoding (e.g., `%2e%2e` for `..`) and missing common redirection operators like `<`.
**Action:** Always decode percent-encoded strings before performing security validation on paths. Explicitly block all shell redirection operators (`<`, `>`) to prevent unauthorized file access or exfiltration.
