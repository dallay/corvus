---
title: Core Tools
description: Reference for system command execution and filesystem tools in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

Core tools provide the foundation for agent autonomy, allowing interaction with the host operating system and the local workspace.

## `shell`

Executes an arbitrary shell command within the workspace directory.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Execution:** Runs via the configured [Runtime](../architecture.md#runtime) (Native or Docker).
- **Constraints:**
  - Blocked commands: Defined in `autonomy.forbidden_paths`.
  - Allowed commands: Must be in `autonomy.allowed_commands` if configured.
  - Environment: Only safe functional variables (`PATH`, `HOME`, `USER`, etc.) are passed. API keys and secrets are explicitly redacted/cleared.
  - Timeout: Defaults to 60 seconds.
  - Output limit: Truncated at 1 MB to prevent memory exhaustion.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `command` | `string` | **Required.** The shell command to execute. |
| `approved` | `boolean` | Set `true` to explicitly approve medium/high-risk commands in supervised mode. |

---

## `file_read`

Reads the contents of a file within the workspace.

- **Security Tier:** Read-Only (Safe).
- **Constraints:**
  - Path traversal (e.g., `../../etc/passwd`) is strictly blocked.
  - Symlinks that resolve outside the workspace boundary are rejected.
  - Maximum file size: 10 MB.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `path` | `string` | **Required.** Relative path to the file within the workspace. |

---

## `file_write`

Writes or overwrites a file within the workspace.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Constraints:**
  - Creates parent directories automatically if they don't exist.
  - Refuses to write through symlinks (TOCTOU protection).
  - Subject to the same path-sandboxing rules as `file_read`.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `path` | `string` | **Required.** Relative path to the file within the workspace. |
| `content` | `string` | **Required.** The content to write to the file. |
