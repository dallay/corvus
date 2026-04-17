# Composing Agents with Corvus

This guide explains how to create, build, and run composed agents using the
Corvus capability-based architecture.  A composed agent is defined entirely in
a TOML manifest — no Rust code required.

---

## Table of Contents

1. [Concepts](#concepts)
2. [Manifest Structure](#manifest-structure)
3. [Manifest Fields Reference](#manifest-fields-reference)
4. [CLI Usage](#cli-usage)
5. [Agent Templates](#agent-templates)
6. [End-to-End Example](#end-to-end-example)
7. [Validation Rules](#validation-rules)
8. [Security Notes](#security-notes)
9. [Capability Constraints](#capability-constraints)
10. [Platform Limitations](#platform-limitations)
11. [Migration from Full Runtime](#migration-from-full-runtime)
12. [Troubleshooting](#troubleshooting)

---

## Concepts

| Concept | Description |
|---------|-------------|
| **Manifest** | TOML file that declares which capabilities an agent needs. |
| **Capability** | A registered provider, channel, tool, memory backend, or sandbox. |
| **Composer** | The `AgentComposer` that parses, validates, and wires an agent from a manifest. |
| **Registry** | The per-capability crate (`corvus-providers`, `corvus-tools`, …) that knows which implementations are available. |

The key guarantee is **behavioral equivalence**: a capability used inside a
composed agent behaves identically to the same capability inside the full
runtime, because both draw from the same registered implementations.

---

## Manifest Structure

```toml
version = "1.0"
name    = "my-agent"
description = "Optional human-readable description."

[providers]
providers   = ["anthropic"]   # required, at least one
default     = "anthropic"     # required, must be in providers list
model       = "claude-haiku-4-5-20251001"  # optional
temperature = 0.7             # optional

[channels]
channels = ["slack"]          # required, at least one
default  = "slack"            # required when channels has >1 entry

[tools]
tools = ["file_read", "shell"] # required (can be empty [])
mode  = "allow"               # optional; "allow" (default) or "deny"

[memory]                      # optional section
backend = "sqlite"
[memory.config]
path = "${CORVUS_DATA_DIR}/memory.db"

[observer]                    # optional section
name = "audit"
[observer.config]
output = "stdout"
format = "json"

[security]                    # optional section
sandbox           = "none"
tool_restrictions = ["file_read"] # must be subset of tools.tools
```

---

## Manifest Fields Reference

### Top-level

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | string | ✅ | Manifest schema version. Currently `"1.0"`. |
| `name` | string | ✅ | Agent identifier. Used in logs and CLI output. |
| `description` | string | — | Human-readable description. |

### `[providers]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `providers` | string[] | ✅ | Enabled provider names. |
| `default` | string | ✅ | Default provider. Must be in `providers`. |
| `model` | string | — | Model identifier passed to the provider. |
| `temperature` | float | — | Sampling temperature (0.0 – 2.0). |

**Known providers:** `anthropic`, `openai`, `openrouter`, `google`, `gemini`,
`azure`, `cerebro`, `ollama`, `xai`.

### `[channels]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `channels` | string[] | ✅ | Enabled channel names. |
| `default` | string | — | Default channel. Required when `channels` has more than one entry. |

**Known channels:** `telegram`, `discord`, `slack`, `webhook`, `stdio`.

### `[tools]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tools` | string[] | ✅ | Enabled tool names. Empty list `[]` is valid. |
| `mode` | string | — | `"allow"` (default) or `"deny"`. |

**Known tools:** `shell`, `file_read`, `file_write`, `browser`, `http_request`,
`memory_recall`, `memory_store`, `memory_forget`, `web_search_tool`,
`code_search`, `git_operations`, `image_info`, `screenshot`, `browser_open`,
`delegate`, `composio`, `cron_add`, `cron_list`, `cron_remove`, `cron_run`,
`cron_runs`, `cron_update`, `pushover`, `schedule`, `hardware_board_info`,
`hardware_memory_map`, `hardware_memory_read`.

### `[memory]` (optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `backend` | string | ✅ | Memory backend name. |
| `config` | table | — | Backend-specific configuration. |

**Known backends:** `sqlite`, `none`.

### `[observer]` (optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✅ | Observer name (e.g., `"audit"`). |
| `config` | table | — | Observer-specific configuration. |

### `[security]` (optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `sandbox` | string | — | Sandbox backend. |
| `tool_restrictions` | string[] | — | Allowlist of tools the agent may invoke. Must be a subset of `tools.tools`. |

**Known sandboxes:** `wasmi`, `landlock`, `bubblewrap`, `none`.

---

## CLI Usage

### Scaffold a new agent from a template

```bash
corvus agent new --template chat-bot --name my-bot
# Creates: agents/my-bot/agent.toml
```

### Validate a manifest (dry-run)

```bash
corvus agent build agents/my-bot/agent.toml
```

`build` validates the manifest and prints the resolved `CapabilityReport`.
It does **not** start the agent.

### Run a composed agent

```bash
corvus agent run agents/my-bot/agent.toml
```

This validates, composes, and starts the agent.  Press `Ctrl-C` to stop.

### Use the interactive REPL (full runtime, no manifest)

```bash
corvus
```

The REPL is unaffected by composition changes — it uses the full runtime
and reads configuration from environment variables as before.

---

## Agent Templates

Pre-built templates live in `agents/templates/`.

| Template | Channels | Tools | Memory | Sandbox | Use-case |
|----------|----------|-------|--------|---------|----------|
| `minimal` | stdio | none | none | none | Spike / quick test |
| `chat-bot` | slack | none | sqlite | none | Conversational bot |
| `support-agent` | slack, telegram, webhook | http, memory, search | sqlite | none | Helpdesk |
| `code-assistant` | stdio | file, shell, git, search | sqlite | none | Local coding aid |
| `research-agent` | webhook, stdio | http, browser, search, memory | sqlite | none | Autonomous research |
| `ops-agent` | slack, webhook, stdio | full toolset | sqlite | bubblewrap | Infrastructure automation |

Scaffold from any template:

```bash
corvus agent new --template <name> --name <my-agent>
```

---

## End-to-End Example

### Goal

Create a Slack greeting bot using the `chat-bot` template.

### Step 1: Scaffold

```bash
corvus agent new --template chat-bot --name greeter
# Created: agents/greeter/agent.toml
```

### Step 2: Customize

Edit `agents/greeter/agent.toml`:

```toml
version = "1.0"
name    = "greeter"
description = "Greets users in the #general Slack channel."

[providers]
providers   = ["anthropic"]
default     = "anthropic"
model       = "claude-haiku-4-5-20251001"
temperature = 0.9

[channels]
channels = ["slack"]
default  = "slack"

[tools]
tools = []
mode  = "allow"

[memory]
backend = "sqlite"

[memory.config]
path = "${CORVUS_DATA_DIR}/greeter-memory.db"
```

### Step 3: Validate

```bash
corvus agent build agents/greeter/agent.toml
```

Expected output:

```
Manifest valid.
Capability report:
  providers : ["anthropic"]
  channels  : ["slack"]
  tools     : []
  memory    : sqlite
```

### Step 4: Run

```bash
export ANTHROPIC_API_KEY="sk-..."
export SLACK_BOT_TOKEN="xoxb-..."
corvus agent run agents/greeter/agent.toml
```

---

## Validation Rules

The composer enforces these rules before wiring any capability:

| Rule | Description |
|------|-------------|
| R1 | `providers.providers` must not be empty. |
| R2 | `channels.channels` must not be empty. |
| R3 | `providers.default` must be present in `providers.providers`. |
| R4 | Every provider, channel, tool, memory backend, and sandbox must be a known registered name. |
| R5 | `security.tool_restrictions` must be a subset of `tools.tools`. |
| R6 | `channels.default` must be set when `channels.channels` has more than one entry. |
| R7 | Inline secrets (values containing `sk-`, `xoxb-`, `Bearer `, etc.) are rejected; use environment variable references instead. |

`corvus agent build` (validate-only) reports all violations and exits non-zero
on any error.

---

## Security Notes

- **Never put secrets in manifests.** Use environment variable references
  (e.g., `${ANTHROPIC_API_KEY}`).  Rule R7 will reject manifests that contain
  known secret patterns.
- **`tool_restrictions`** is your first line of defense.  Even if a tool is
  enabled, restricting it here limits what the agent can actually call.
- **Sandboxes** (`landlock`, `bubblewrap`, `wasmi`) provide OS-level
  isolation.  `none` means no isolation — acceptable for local stdio agents,
  not for agents exposed over the network.
- **`shell` and `file_write`** are high-privilege tools.  Enable them only
  when strictly necessary, and always pair them with a sandbox and
  `tool_restrictions`.
- The `ops-agent` template ships with `bubblewrap` — review and tighten
  `tool_restrictions` before deploying to production.

---

## Capability Constraints

Some combinations have known constraints:

| Constraint | Detail |
|------------|--------|
| `browser` / `screenshot` require a display | Headless mode is enabled by default, but some environments need `DISPLAY` or `XVFB`. |
| `landlock` sandbox is Linux-only | Falls back to `none` on macOS/Windows with a warning. |
| `bubblewrap` sandbox requires root or unprivileged user namespaces | Check `sysctl kernel.unprivileged_userns_clone`. |
| `wasmi` sandbox only applies to tools compiled as WASM modules | Native tools run unsandboxed even when `wasmi` is set. |
| `memory_recall` / `memory_store` require `[memory]` section | Validator emits a warning if memory tools are enabled without a memory backend. |

---

## Platform Limitations

| Platform | Limitation |
|----------|------------|
| macOS | `landlock` and `bubblewrap` sandboxes not supported; use `none`. |
| Windows | `landlock` and `bubblewrap` sandboxes not supported; use `none`. |
| Linux | All sandboxes supported. `bubblewrap` preferred for server workloads. |
| All | `wasmi` sandbox only covers WASM-compiled tools. |

---

## Migration from Full Runtime

If you currently run the Corvus REPL or a custom Rust binary using the full
runtime, here is how to migrate to a composed agent:

### Before (full runtime, env-var config)

```bash
ANTHROPIC_API_KEY="sk-..." corvus
```

The REPL loads all registered providers, channels, and tools.

### After (composed agent, explicit manifest)

1. Create a manifest that lists the capabilities you actually use.
2. Validate: `corvus agent build my-agent.toml`
3. Run: `corvus agent run my-agent.toml`

**Backward compatibility:** The full runtime REPL (`corvus` with no
sub-command) is unchanged.  Composed agents are additive — you opt in by
using `corvus agent build/run`.

### Migration checklist

- [ ] Identify providers, channels, and tools your agent actively uses.
- [ ] Create a manifest (start from the closest template).
- [ ] Set `tool_restrictions` to exactly the tools your agent needs.
- [ ] Choose a sandbox appropriate for your deployment environment.
- [ ] Validate with `corvus agent build`.
- [ ] Test with `corvus agent run` in a staging environment.
- [ ] Remove hard-coded env-var wiring from your deployment scripts.

---

## Troubleshooting

### `unknown provider capability 'foo'`

`foo` is not in the known provider registry.  Check the spelling against the
**Known providers** list in [Manifest Fields Reference](#manifest-fields-reference).

### `default provider 'foo' must be enabled`

`providers.default` must appear in `providers.providers`.

### `default channel must be specified when multiple channels are configured`

Add `default = "<channel-name>"` under `[channels]`.

### `tool restrictions [...] must be subset of enabled tools [...]`

Every entry in `security.tool_restrictions` must also appear in `tools.tools`.

### `inline secret not allowed in '...'`

Move the secret value to an environment variable and reference it as
`${MY_SECRET}` in the manifest.

### Agent starts but tools fail at runtime

Most tool failures are caused by missing environment variables (API keys,
tokens, paths).  Check the tool's documentation for required environment
variables and ensure they are set before running `corvus agent run`.
