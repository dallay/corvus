---
title: Cerebro MCP Tools Reference
description: >-
  Reference for the 8 callable Cerebro memory tools exposed today,
  plus 5 deferred tools that still return NotImplemented.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: reference
---

# MCP Tools Reference

Cerebro currently exposes 8 callable memory tools via JSON-RPC over HTTP at
`POST /mcp`. All requests use the MCP protocol (JSON-RPC 2.0).

Five additional tool names remain reserved for future implementation and currently return a
structured `NotImplemented` error when called.

## Request Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "<tool_name>",
    "arguments": { ... }
  }
}
```

## Tool Status

| Status        | Meaning                                  |
|---------------|------------------------------------------|
| Implemented   | Available and functional                 |
| Planned       | Defined but returns `NotImplemented`     |

---

## Implemented Tools

### `mem_save`

Save a new memory observation.

**Parameters:**

| Field       | Type   | Required | Description                    |
|-------------|--------|----------|--------------------------------|
| `content`   | String | Yes      | The observation content        |
| `what`      | String | No       | What was observed              |
| `why`       | String | No       | Why it matters                 |
| `where`     | String | No       | Context or source              |
| `scope`     | String | No       | Scope identifier               |
| `topic_key` | String | No       | Topic for organization         |

**Returns:** `memory_id`, `status`

---

### `mem_search`

Search stored memories by query.

**Parameters:**

| Field             | Type   | Required | Description                |
|-------------------|--------|----------|----------------------------|
| `query`           | String | Yes      | Search query               |
| `limit`           | Number | No       | Max results to return      |
| `scope`           | String | No       | Filter by scope            |
| `topic_key`       | String | No       | Filter by topic            |
| `include_deleted` | bool   | No       | Include soft-deleted items |

**Returns:** `results_count`, `truncated`, list of memories

---

### `mem_delete`

Delete a memory observation.

**Parameters:**

| Field         | Type   | Required | Description                  |
|---------------|--------|----------|------------------------------|
| `memory_id`   | String | Yes      | ID of memory to delete       |
| `topic_key`   | String | No       | Topic filter                 |
| `hard_delete` | bool   | No       | Permanently remove (vs soft) |

**Returns:** `memory_id`, `status`, `deleted`

---

### `mem_get_observation`

Retrieve a specific memory by ID.

**Parameters:**

| Field             | Type   | Required | Description                |
|-------------------|--------|----------|----------------------------|
| `memory_id`       | String | Yes      | ID of memory to retrieve   |
| `include_deleted` | bool   | No       | Include if soft-deleted     |

**Returns:** `memory_id`, `status`, full observation data

---

### `mem_update`

Update an existing memory observation.

**Parameters:**

| Field       | Type   | Required | Description                    |
|-------------|--------|----------|--------------------------------|
| `memory_id` | String | Yes      | ID of memory to update         |
| `content`   | String | No       | Updated content                |
| `what`      | String | No       | Updated what                   |
| `why`       | String | No       | Updated why                    |
| `where`     | String | No       | Updated context                |

**Returns:** `memory_id`, `status`

---

### `mem_suggest_topic_key`

Suggest a topic key for organizing memories.

**Parameters:**

| Field   | Type   | Required | Description              |
|---------|--------|----------|--------------------------|
| `scope` | String | No       | Scope to suggest within  |
| `input` | String | No       | Input text for suggestion|

**Returns:** `topic_key`, `candidates_count`

---

### `mem_stats`

Get memory storage statistics.

**Parameters:** None

**Returns:**

| Field               | Type   | Description                    |
|---------------------|--------|--------------------------------|
| `memory_count`      | Number | Total stored memories          |
| `session_count`     | Number | Total sessions                 |
| `prompt_count`      | Number | Total saved prompts            |
| `worker_enabled`    | bool   | Background worker status       |
| `worker_queue_depth`| Number | Pending worker tasks           |

---

### `mem_timeline`

Get a chronological timeline of memories.

**Parameters:**

| Field             | Type   | Required | Description                |
|-------------------|--------|----------|----------------------------|
| `memory_id`       | String | No       | Center timeline on this ID |
| `before`          | String | No       | Entries before this date   |
| `after`           | String | No       | Entries after this date    |
| `include_deleted` | bool   | No       | Include soft-deleted items |

**Returns:** `items_count`, list of timeline entries

---

## Planned Tools

:::caution
These tools are defined in the schema but not yet implemented.
Calling them returns a `NotImplemented` error.
:::

### `mem_save_prompt`

Save a prompt template for reuse.

### `mem_session_start`

Start a new memory session for grouping observations.

### `mem_session_end`

End an active memory session.

### `mem_session_summary`

Generate a summary of a completed session.

### `mem_context`

Retrieve contextual memory for the current conversation.

---

## JSON Schemas

Machine-readable JSON schema definitions for all tools are
available in the repository at
[`guides/cerebro/mcp-schema/`](../guides/cerebro/mcp-schema/).

## Example: Full Request/Response

```bash
curl -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "mem_save",
      "arguments": {
        "content": "User prefers dark mode",
        "topic_key": "preferences",
        "what": "UI preference",
        "why": "Personalization"
      }
    }
  }'
```

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "memory_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "saved"
  }
}
```
