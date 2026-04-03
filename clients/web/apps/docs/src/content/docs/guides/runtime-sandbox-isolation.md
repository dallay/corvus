---
title: Runtime Sandbox Isolation
description: Security model, backend selection, sidecar verification, and audit expectations for runtime sandbox isolation in Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-03
appliesTo: main
docType: guide
---

# Runtime Sandbox Isolation

Corvus uses **two security layers** for user-triggered execution:

1. **Application-layer policy** via `SecurityPolicy`
   - command allowlists
   - path restrictions
   - rate limits
   - risk classification and approval gates
2. **OS-level sandboxing** via the runtime `Sandbox` backends
   - `landlock`
   - `firejail`
   - `bubblewrap`
   - `docker`

Both layers matter. The policy layer decides **whether** an action is allowed. The OS sandbox constrains **what the action can reach even if it is allowed**.

## Sandbox backend selection

Configure the runtime under `security.sandbox`:

```toml
[security.sandbox]
enabled = true
backend = "auto"      # auto | landlock | firejail | bubblewrap | docker | none
require = false        # when true, fail closed if no OS sandbox is available
firejail_args = []
```

### `backend = "auto"`

Corvus tries supported backends in platform order and uses the first available one.

- Linux: `landlock` → `firejail` → `bubblewrap` → `docker`
- macOS: `bubblewrap` → `docker`
- other platforms: `docker`
- if none are available and `require = false`, Corvus falls back to `none`

### `require = true`

When `require = true`, Corvus **fails closed**.

That means:
- explicit unavailable backend → startup error
- `backend = "auto"` with no backend found → startup error
- `backend = "none"` → startup error
- `enabled = false` → startup error

Use this for real user-facing deployments where OS isolation is mandatory.

## Execution contract

### Shell execution

For the `shell` tool, Corvus now applies this sequence:

1. validate command against `SecurityPolicy`
2. sanitize environment variables
3. wrap the command with the selected sandbox backend
4. execute with timeout
5. audit the result with sandbox metadata

This means every allowed shell command runs inside the active sandbox boundary when one is configured.

### Noop sandbox behavior

If no OS-level backend is available and `require = false`, Corvus uses `none` (`NoopSandbox`).

In that mode:
- application-layer policy still applies
- mutating commands emit a warning that OS sandboxing is not active
- audit logs record `sandbox_backend = "none"`

This is allowed for local development, but it is a weaker security posture.

## Computer-use sidecar isolation

Computer-use actions (`mouse_move`, `mouse_click`, `mouse_drag`, `key_type`, `screen_capture`) use a sidecar.

Safe defaults:
- endpoint defaults to loopback: `http://127.0.0.1:8787/v1/actions`
- remote/public endpoints are blocked unless `allow_remote_endpoint = true`
- public remote endpoints must use HTTPS
- allowed domains, window allowlists, and coordinate limits are forwarded as sidecar policy

Corvus performs a **lazy health-check** against the sidecar on first computer-use action using:

- `GET /v1/health`

The sidecar is expected to report isolation details such as:

```json
{
  "status": "healthy",
  "isolation": {
    "type": "container",
    "runtime": "docker",
    "version": "24.0.7"
  }
}
```

Corvus records this as a `SecurityEvent` audit entry.

### When sidecar verification fails

- if `security.sandbox.require = false`: Corvus logs a warning and continues
- if `security.sandbox.require = true`: Corvus rejects the computer-use action because sidecar isolation could not be verified

## Audit expectations

Shell command audit events include:
- command (redacted to strip inline secrets)
- risk level
- approval flag
- success/failure
- `security.sandbox_backend`

Computer-use sidecar verification generates a `SecurityEvent` audit entry describing:
- sidecar health status
- reported isolation type
- reported runtime

## Recommended operator defaults

### Local development

```toml
[security.sandbox]
backend = "auto"
require = false
```

### Hardened workstation or server deployment

```toml
[security.sandbox]
backend = "auto"
require = true
```

### Remote sidecar usage

Only enable this when you control the deployment:

```toml
[browser.computer_use]
endpoint = "https://computer-use.example.com/v1/actions"
allow_remote_endpoint = true
```

If you do this, make sure the sidecar itself runs in an isolated environment and exposes `/v1/health` with truthful isolation metadata.

## What this does not guarantee

This change does **not** add:
- per-user sandbox instances
- per-session containerization
- automatic sidecar process sandboxing by Corvus itself

Operators still own the deployment boundary for the computer-use sidecar.
