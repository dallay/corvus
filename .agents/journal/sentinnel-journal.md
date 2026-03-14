
## 2025-05-15 - Shell Redirection and URL-encoded Traversal

**Learning:** Shell input redirection operator (`< `) and URL-encoded parent directory markers (`%2e%2e`) were identified as potential security bypasses in the `SecurityPolicy`. While standard traversal components (`..`) and output redirections (`>`) were already blocked, these variants remained open.

**Action:** Explicitly block `<` in `contains_blocked_operators` and `%2e%2e` in `is_path_allowed` to ensure defense-in-depth against command injection and path traversal. Always add regression tests for these specific patterns.
