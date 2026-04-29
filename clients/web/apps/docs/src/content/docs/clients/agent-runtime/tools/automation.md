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

## `delegate`

Delegates a subtask to a specialized agent.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Execution Modes:**
  - **OneShot:** A single LLM call to a sub-agent.
  - **Session:** Launches a bounded child agent with a full tool loop (Code Session).
- **Depth Limit:** Enforces a maximum recursion depth to prevent infinite delegation loops.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `agent` | `string` | **Required.** Name of the configured sub-agent (e.g., `researcher`, `coder`). |
| `prompt` | `string` | **Required.** The task or prompt to send to the sub-agent. |
| `context` | `string` | Optional context to prepend to the task. |

---

## `delegate_launch`

Launch a supervised multi-child orchestration run.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Scope:** Process-local only.
- **Returns:** An opaque `handle` and an initial `snapshot`.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `children` | `array` | **Required.** List of child agent launch descriptors. |

Each child descriptor requires `child_id`, `agent_name`, and `prompt`. Optional `execution` metadata can override sandbox modes, models, and transport.

---

## `delegate_inspect`

Return a point-in-time snapshot of an active orchestration run.

- **Security Tier:** Read-Only (Safe).
- **Contract:** Requires the `handle` returned by `delegate_launch`.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `handle` | `string` | **Required.** The opaque handle from `delegate_launch`. |

---

## `delegate_cancel`

Cancel an active supervised orchestration run.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Contract:** Requires the `handle` returned by `delegate_launch`.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `handle` | `string` | **Required.** The opaque handle from `delegate_launch`. |

---

## `composio`

Executes actions on 1000+ managed apps via the Composio platform.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Integrations:** Gmail, Notion, GitHub, Slack, Linear, and more.
- **Requirements:** Requires `COMPOSIO_API_KEY` in the workspace environment.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `action` | `string` | **Required.** Operation to perform: `list`, `execute`, or `connect`. |
| `app` | `string` | App/Toolkit slug (e.g., `gmail`). |
| `tool_slug` | `string` | The specific tool identifier to execute. |
| `params` | `object` | JSON parameters for the action. |

---

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

## `cron_*` / `schedule`

Tools for managing autonomous, time-based execution. Corvus provides both a set of granular `cron_*` tools and a unified `schedule` tool.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Job Types:**
  - **Agent Job:** The agent runs itself with a specific prompt.
  - **Shell Job:** Executes a shell command.
- **Schedules:**
  - `cron`: Recurring tasks (e.g., `0 9 * * *`).
  - `at`: One-shot tasks at a specific RFC3339 timestamp.
  - `every`: Fixed intervals in milliseconds.

### Tools

| Tool | Description |
| :--- | :--- |
| `cron_add` | Create a new scheduled job. |
| `cron_list` | List all configured cron jobs. |
| `cron_remove` | Delete a cron job by ID. |
| `cron_run` | Force-run a job immediately. |
| `cron_runs` | View recent run history for a job (requires `job_id`). |
| `cron_update` | Patch an existing job's schedule or configuration (requires `job_id` and `patch`). |
| `schedule` | Unified tool for `create`, `list`, `get`, `cancel`, `pause`, and `resume`. |

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
