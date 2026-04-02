---
title: Automation & Utility Tools
description: Reference for Git, Cron, Scheduling, and Notification tools in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

These tools enable the agent to perform repository management, schedule future actions, and notify the user.

## `git_operations`

A structured interface for common Git tasks.

- **Security Tier:** Mixed (Write operations like `commit` are Action-Bearing).
- **Supported Operations:** `status`, `diff`, `log`, `branch`, `commit`, `add`, `checkout`, `stash`.
- **Safety:** Automatically sanitizes arguments to prevent shell injection (blocks `--exec`, `-c`, etc.).

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `operation` | `string` | **Required.** One of the supported git commands. |
| `message` | `string` | Commit message (for `commit`). |
| `paths` | `string` | File paths to stage (for `add`). |

---

## `cron_add` / `schedule`

Tools for managing autonomous, time-based execution.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Capability:** Allows the agent to schedule itself (Agent Job) or a shell script (Shell Job) to run in the future.
- **Schedules:**
  - `cron`: Recurring tasks (e.g., `0 9 * * *`).
  - `at`: One-shot tasks at a specific RFC3339 timestamp.
  - `every`: Fixed intervals in milliseconds.

### `schedule` Actions
`create`, `list`, `get`, `cancel`, `pause`, `resume`.

---

## `pushover`

Sends a push notification to the user's mobile device.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Requirements:** Requires `PUSHOVER_TOKEN` and `PUSHOVER_USER_KEY` in the workspace `.env` file.
- **Usage:** Ideal for notifying the user when a long-running mission is complete or requires manual intervention.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `message` | `string` | **Required.** The notification text. |
| `priority` | `integer` | Priority from -2 (silent) to 2 (emergency). |
| `sound` | `string` | Optional sound override (e.g., `bugle`, `bike`). |
