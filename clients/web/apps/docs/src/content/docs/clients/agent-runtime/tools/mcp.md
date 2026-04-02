---
title: Model Context Protocol (MCP)
description: Guide for integrating and using MCP tools in the Corvus Agent Runtime.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

# Model Context Protocol (MCP)

The Model Context Protocol (MCP) is an open standard that enables agents to connect to external tools, data sources, and services. Corvus provides a first-class MCP runtime that integrates these external capabilities directly into the agent's toolbelt.

## Configuration

MCP servers are configured in `config.toml` under the `[mcp]` section. Each server requires a unique name and a command to launch it.

```toml
[mcp]
enabled = true

[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_PERSONAL_ACCESS_TOKEN = "your_token_here" }

[[mcp.servers]]
name = "postgres"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

## Namespacing

To prevent name collisions with built-in tools, all MCP tools are automatically prefixed with the server name:

`mcp.<server_name>.<tool_name>`

*Example:* If the `github` server provides a `create_issue` tool, the agent will see it as `mcp.github.create_issue`.

## Security & Approval

MCP tools are treated as **Action-Bearing (Risk-bearing)** by default.

- **Approval:** In Supervised mode, any call to an MCP tool will trigger an approval request.
- **Timeouts:** Every MCP call has a default timeout (30s) to prevent hanging the agent loop.
- **Output Limits:** Responses are capped (default 64 KB) to protect context window space.

## Discovery

Corvus discovers MCP tools at startup. If a server fails to start, the runtime will log the error but continue to operate with other healthy servers. You can verify discovered tools using:

```bash
corvus doctor
```

## Supported Capability Types

The Corvus MCP implementation currently supports:
- **Tools:** Executable functions (e.g., query database, send email).
- **Resources:** (Planned) Read-only data sources.
- **Prompts:** (Planned) Pre-defined prompt templates.
