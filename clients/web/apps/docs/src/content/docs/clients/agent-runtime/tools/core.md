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

## `code_search`

Searches workspace files for literal or regex matches and returns both human-readable output and structured match data.

- **Security Tier:** Read-Only (Safe).
- **Execution:** Native runtime tool with optional literal-query index narrowing and mandatory live verification against current file contents.
- **Current limitation:** Regex correctness is supported, but indexed candidate narrowing does **not** support regex in v1. Regex planning falls back with `query_regex_not_supported` to discovery plus live verification.
- **Rollout evidence:** See the dedicated [`code_search` rollout page](code-search.md) for measured shell-vs-native results, fallback labels, and recommendation guidance.

### Parameters

See the dedicated [`code_search` page](code-search.md) for the full parameter contract, benchmark methodology, and rollout guidance.

---

## `Glob`

Claude-style parity tool for workspace-safe file pattern discovery.

- **Security Tier:** Read-Only (Safe).
- **Execution:** Native runtime tool backed by workspace discovery metadata helpers.
- **Contract:** Requires `pattern`; optionally scopes traversal with a workspace-relative `path`.
- **Parity note:** `Glob` is the canonical parity-facing name for this slice and is additive alongside existing native tool names.

---

## `Grep`

Claude-style parity content search backed by the same Corvus search internals used by `code_search`.

- **Security Tier:** Read-Only (Safe).
- **Execution:** Native runtime tool with deterministic workspace-relative outputs.
- **Contract:** Supports `pattern`, optional `path`, optional `glob`, and `output_mode` values `content`, `files_with_matches`, or `count`.
- **Parity note:** `Grep` is canonical for parity-facing documentation, while `code_search` remains available as the retained native contract.

---

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
