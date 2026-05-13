---
title: Web Tools
description: Reference for web browsing, search, and HTTP request tools in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-13
appliesTo: main
docType: reference
---

# Web Tools

Web tools enable agents to retrieve information from the internet and interact with external APIs. All web tools enforce a strict **Domain Allowlist** policy.

## `web_search_tool`

Performs a web search to find current information, news, or research topics.

- **Security Tier:** Read-Only (Safe).
- **Plan Mode:** ✅ Safe for Plan Mode (`--plan`).
- **Providers:**
  - `duckduckgo` (Default): Free, no API key required.
  - `brave`: Requires `web_search.brave_api_key`.
- **Results:** Returns ranked titles, URLs, and snippets.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `query` | `string` | **Required.** The search query. Be specific for better results. |

---

## `browser`

Full browser automation for interacting with complex web applications. Supports multiple backends including Playwright-based `agent-browser` and OS-level `computer_use`.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Backends:**
  - `agent_browser`: Uses the `agent-browser` CLI.
  - `rust_native`: Built-in Rust driver (requires `browser-native` feature).
  - `computer_use`: OS-level mouse/keyboard control via sidecar.
- **Constraints:** Enforces `browser.allowed_domains`.

### Common Actions

| Action | Description |
| :--- | :--- |
| `open` | Navigate to a URL (HTTPS only). |
| `snapshot` | Get an accessibility-tree snapshot with element refs (`@e1`, `@e2`). |
| `click` | Click an element by ref (e.g., `@e5`) or selector. |
| `fill` | Type text into an input field. |
| `screenshot` | Capture a visual of the current page. |

---

## `browser_open`

A lightweight alternative to `browser` that simply opens an approved HTTPS URL in the host's Brave Browser.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Note:** This tool does **not** allow the agent to scrape or see the page content; it is for opening pages for the user's benefit.

---

## `http_request`

Performs structured HTTP requests (REST/JSON) to external APIs.

- **Security Tier:** Action-Bearing (Risk-bearing).
- **Constraints:**
  - Only `http://` and `https://` schemes are allowed.
  - Local/private hosts (SSRF protection) are strictly blocked.
  - Sensitive headers (Authorization, API-Key) are redacted in logs.
  - Redirects are disabled by default for security.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `url` | `string` | **Required.** The full URL to request. |
| `method` | `string` | HTTP method (GET, POST, PUT, DELETE, etc.). Default: `GET`. |
| `headers` | `object` | Optional key-value pairs for headers. |
| `body` | `string` | Optional payload for POST/PUT requests. |

---

## `WebFetch`

Read-only fetch-and-extract parity tool for allowlisted web content.

- **Security Tier:** Read-Only (Safe).
- **Plan Mode:** ✅ Safe for Plan Mode (`--plan`).
- **Execution:** Uses the same outbound URL-policy boundary as `http_request` for host allowlists, private-host blocking, and redirect-denial behavior.
- **Compatibility alias:** `web_fetch`
- **Native relationship:** Uses the same outbound URL policy boundary as `http_request`, but remains a read-only fetch-and-extract surface.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `url` | `string` | **Required.** The full URL to fetch. |
| `prompt` | `string` | **Required.** Guiding instructions for extraction. |
