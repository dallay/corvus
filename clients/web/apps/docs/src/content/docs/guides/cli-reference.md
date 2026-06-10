---
title: CLI Reference
description: Comprehensive guide to the Corvus Agent CLI commands and options.
owner: team-platform
status: canonical
lastReviewed: 2026-06-10
appliesTo: main
docType: reference
---

The Corvus CLI (`corvus`) is the primary interface for managing your agents, hardware, and services.

## Core Commands

### `onboard`

Initialize your workspace and configuration.

- `--interactive`: Run the full interactive wizard (default is quick setup).
- `--channels-only`: Reconfigure channels only.
- `--api-key <KEY>`: API key for quick setup.
- `--provider <NAME>`: Provider name (default: openrouter).
- `--memory <TYPE>`: Memory backend (sqlite, lucid, markdown, none).

When using `--interactive`, the wizard now ends with an optional dashboard step:

- Prompt: `Activate web dashboard now? (optional)`
- Accept path: prints a compact activation guide, attempts optional browser open (non-fatal), and
  reports deterministic `DASH-*` status output with fallback commands.
- Decline path: preserves CLI-only completion and prints a resume-later command block.

**Example:**

```bash
corvus onboard --interactive
```

### `agent`

Start the AI agent loop (or compose from manifest).

- `-m, --message <TEXT>`: Single message mode (don't enter interactive mode).
- `-p, --provider <NAME>`: Provider to use (openrouter, anthropic, openai, openai-codex).
- `--model <MODEL>`: Specific model to use.
- `-t, --temperature <VALUE>`: Temperature (0.0 - 2.0, default: 0.7).
- `--peripheral <BOARD:PATH>`: Attach a peripheral (e.g., `nucleo-f401re:/dev/ttyACM0`).
- `--override-budget`: Allow exactly one over-budget request for this CLI session.
- `--plan`: Run the turn in plan mode (analysis-only tool execution).

**Subcommands:**

- `build --manifest <PATH>`: Build an agent from a manifest TOML file.
  - `--output <DIR>`: Output directory for compiled agent.
- `run --manifest <PATH>`: Run an agent directly from a manifest (boot-time composition).
- `new --template <NAME> --name <NAME>`: Create a new agent from a template.
  - `--output <DIR>`: Output directory (optional).

**Example:**

```bash
corvus agent -m "Hello, how can you help me today?"
```

### `code`

Run a code-specialist session (inspect, plan, edit, verify, report).

- `-m, --message <TEXT>`: Task description or instruction for the code session.
- `-p, --provider <NAME>`: Provider to use (openrouter, anthropic, openai).
- `--model <MODEL>`: Specific model to use.
- `-t, --temperature <VALUE>`: Temperature (0.0 - 2.0, default: 0.7).
- `--override-budget`: Allow exactly one over-budget request for this CLI session.
- `--plan`: Run the session in plan mode (analysis-only tool execution).

**Example:**

```bash
corvus code -m "Fix the bug in the authentication module"
```

### `daemon`

Start the long-running autonomous runtime (gateway + channels + heartbeat + scheduler).

- `-p, --port <PORT>`: Port to listen on.
- `--host <HOST>`: Host to bind to.

**Example:**

```bash
corvus daemon --port 3000
```

### `gateway`

Start only the gateway server (webhooks, websockets).

- `-p, --port <PORT>`: Port to listen on.
- `--host <HOST>`: Host to bind to.

**Example:**

```bash
corvus gateway --port 3001
```

---

## Plan Mode

The `--plan` flag (available for `agent` and `code` commands) enables a high-integrity analysis mode where the agent can explore the environment but cannot perform side-effecting actions.

When Plan Mode is active, Corvus enforces a strict allowlist of "analysis-only" tools. Any attempt to use a tool not on this list will be blocked by the security policy.

### Allowed Tools in Plan Mode

- `Glob` / `glob`
- `Grep` / `grep`
- `WebFetch` / `web_fetch`
- `code_search`
- `file_read`
- `image_info`
- `memory_recall`
- `web_search_tool`

All other tools, including `shell`, `file_write`, `git_operations`, and `delegate`, are strictly disabled.

---

## System & Service

### `status`

Show full system status details.

Includes a `Web dashboard (resume anytime)` section with safe resume commands:

- `corvus gateway`
- `make dev-up` then `./dev/cli.sh up-dashboard` (from Corvus repository root)
- `http://corvus.localhost` + secure proxied `/api/pair` flow
- `corvus --help` for command help

**Example:**

```bash
corvus status
```

### Dashboard activation diagnosis codes

When interactive onboarding dashboard activation is accepted, Corvus may emit one of these stable
codes:

- `DASH-001 GatewayNotRunning`
- `DASH-002 GatewayRunningPairingRequired`
- `DASH-003 GatewayRunningAlreadyPaired`
- `DASH-004 DashboardUiUnavailable`
- `DASH-999 UnknownLocalFailure`

Use this secure manual fallback path when needed:

```bash
corvus status
corvus doctor
corvus gateway
# from Corvus repository root (source checkout):
make dev-up
./dev/cli.sh up-dashboard
```

### `doctor`

Run diagnostics for daemon, scheduler, and channel freshness.

**Example:**

```bash
corvus doctor
```

### `service`

Manage the OS service lifecycle (systemd/launchd).

- `install`: Install the daemon service.
  - `--linger <MODE>`: Linux only: keep service active (Keep, On, Off).
- `start`: Start the service.
- `stop`: Stop the service.
- `restart`: Restart the service.
- `status`: Check service status.
- `uninstall`: Remove the service unit.

**Example:**

```bash
corvus service install --linger on
```

---

## Task Management

### `cron`

Configure and manage scheduled tasks.

- `list`: List all scheduled tasks.
- `add <EXPR> <CMD>`: Add a task using a cron expression.
- `add-at <TIMESTAMP> <CMD>`: Add a one-shot task at a specific RFC3339 time.
- `add-every <MS> <CMD>`: Add a fixed-interval task.
- `once <DELAY> <CMD>`: Add a delayed one-shot task (e.g., "30m", "2h").
- `remove <ID>`: Remove a task.
- `pause <ID>`: Pause a task.
- `resume <ID>`: Resume a task.

**Example:**

```bash
corvus cron add "0 9 * * *" "corvus agent -m 'Daily update'"
```

---

## Providers & Auth

### `providers`

List all supported AI providers.

**Example:**

```bash
corvus providers
```

### `auth`

Manage provider authentication profiles.

- `login --provider <NAME>`: Login with OAuth (e.g., `openai-codex`).
  - `--profile <ID>`: Profile name (default: default).
  - `--device-code`: Use OAuth device-code flow.
- `paste-redirect --provider <NAME>`: Complete OAuth by pasting redirect URL or auth code.
  - `--profile <ID>`: Profile name (default: default).
  - `--input <URL>`: Full redirect URL or raw OAuth code.
- `paste-token --provider <NAME>`: Paste setup token / auth token (for Anthropic subscription auth).
  - `--profile <ID>`: Profile name (default: default).
  - `--token <TOKEN>`: Token value (if omitted, read interactively).
  - `--auth-kind <KIND>`: Auth kind override (`authorization` or `api-key`).
- `setup-token --provider <NAME>`: Alias for `paste-token` (interactive by default).
  - `--profile <ID>`: Profile name (default: default).
- `refresh --provider <NAME>`: Refresh access token using refresh token.
  - `--profile <ID>`: Profile name or profile id.
- `list`: List auth profiles.
- `status`: Show auth status and token expiry.
- `use --provider <NAME> --profile <ID>`: Set active profile.
- `logout --provider <NAME>`: Remove a profile.

**Example:**

```bash
corvus auth list
```

### `models`

Manage provider model catalogs.

- `refresh`: Refresh and cache provider models.
  - `--provider <NAME>`: Provider name (defaults to configured default provider).
  - `--force`: Force live refresh.

**Example:**

```bash
corvus models refresh --provider anthropic
```

---

## Capabilities & Integrations

### `skills`

Manage user-defined capabilities.

- `list`: List installed skills.
- `install <SOURCE>`: Install from a GitHub URL or local path.
- `remove <NAME>`: Remove a skill.

**Example:**

```bash
corvus skills install https://github.com/user/my-skill
```

### `integrations`

Browse available integrations.

- `info <NAME>`: Show details about a specific integration.

**Example:**

```bash
corvus integrations info telegram
```

---

## Communication

### `channel`

Manage communication channels (Telegram, Discord, Slack).

- `list`: List configured channels.
- `start`: Start all configured channels.
- `doctor`: Run health checks for configured channels.
- `add <TYPE> <CONFIG_JSON>`: Add a new channel.
- `remove <NAME>`: Remove a channel.
- `bind-telegram <IDENTITY>`: Bind a Telegram user to the allowlist.

**Example:**

```bash
corvus channel list
```

---

## Hardware & Peripherals

### `hardware`

Discover and introspect USB hardware.

- `discover`: Enumerate USB devices and show known boards.
- `introspect <PATH>`: Details about a device at a specific path.
- `info`: Get chip info via USB (probe-rs).
  - `--chip <CHIP>`: Chip name (e.g., `STM32F401RETx`).

**Example:**

```bash
corvus hardware discover
```

### `peripheral`

Manage hardware peripherals (STM32, RPi, etc.).

- `list`: List configured peripherals.
- `add <BOARD> <PATH>`: Add a peripheral.
- `flash-nucleo`: Flash Corvus firmware to Nucleo-F401RE.
- `flash`: Flash Corvus firmware to Arduino.
  - `-p, --port <PORT>`: Serial port (if omitted, uses first arduino-uno from config).
- `setup-uno-q`: Setup Arduino Uno Q Bridge app (deploy GPIO bridge).
  - `--host <IP>`: Uno Q IP address.

**Example:**

```bash
corvus peripheral add nucleo-f401re /dev/ttyACM0
```

---

## Utilities

### `migrate`

Migrate data from other agent runtimes.

- `openclaw`: Import memory from an OpenClaw workspace.
  - `--source <PATH>`: Optional path to OpenClaw workspace.
  - `--dry-run`: Validate and preview migration without writing data.

**Example:**

```bash
corvus migrate openclaw --source ~/.openclaw/workspace
```

### `update`

Manage runtime updates.

- `status`: Show update status and effective policy.
- `check`: Force an update check.
- `install`: Run update install transaction.
- `auto-enable`: Enable auto-install policy.
- `auto-disable`: Disable auto-install policy.
- `history`: Show update audit history.
- `confirm <NONCE>`: Confirm a one-time update confirmation nonce.

**Example:**

```bash
corvus update check
```

### `cost`

Inspect and manage runtime cost state.

- `summary`: Show the current cost summary (session, daily, monthly).
- `history`: Show aggregated cost history.
  - `--period <PERIOD>`: Aggregation period (session, day, month).
  - `--window <SIZE>`: Number of buckets to include (default: 30).
- `reset`: Reset tracked costs for a specific scope.
  - `--scope <SCOPE>`: Reset scope (session, day, month).
  - `--reason <TEXT>`: Optional reason recorded in cost audit history.

**Example:**

```bash
corvus cost summary
```
