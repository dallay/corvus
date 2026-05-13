---
title: Memory Tools
description: Reference for long-term memory persistence and retrieval tools in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-13
appliesTo: main
docType: reference
---

Memory tools allow the agent to persist information across conversations, effectively building a long-term "soul" or knowledge base.

## `memory_store`

Stores a fact, preference, or note in long-term memory.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Sensitive Data Filter:** Automatically rejects content that appears to contain passwords, API keys, or credentials.
- **Categories:**
  - `core`: Permanent facts (e.g., "The user lives in London").
  - `daily`: Temporary notes for the current session.
  - `conversation`: Chat-specific context.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `key` | `string` | **Required.** Unique identifier for the memory (e.g., `user_pref_theme`). |
| `content` | `string` | **Required.** The information to remember. |
| `category` | `string` | Optional category. Default: `core`. |

---

## `memory_recall`

Searches the memory system for relevant information based on a semantic query.

- **Security Tier:** Read-Only (Safe).
- **Plan Mode:** ✅ Safe for Plan Mode (`--plan`).
- **Retrieval:** Uses hybrid search (Vector similarity + Keyword BM25) when supported by the backend.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `query` | `string` | **Required.** Keywords or phrase to search for. |
| `limit` | `integer` | Maximum number of results to return. Default: `5`. |

---

## `memory_forget`

Permanently removes a memory entry by its key.

- **Security Tier:** Action-Bearing (Risk-bearing).

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `key` | `string` | **Required.** The key of the memory to delete. |
