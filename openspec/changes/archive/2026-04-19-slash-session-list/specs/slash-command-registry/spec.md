# Delta for slash-command-registry

## MODIFIED Requirements

### Requirement: Slash Session Discoverability Family Registration

The system MUST register `/session` as the only canonical slash command entry for the session discoverability family in this slice.

The `/session` descriptor MUST allow empty raw arguments so `/session` can remain the family help or usage hub without a required-text contract. The registry MUST treat `/session status` as the canonical `/session` command plus raw arguments equal to `status`, it MUST treat `/session inspect` as the canonical `/session` command plus raw arguments equal to `inspect`, and it MUST treat `/session list` as the canonical `/session` command plus raw arguments equal to `list`. The registry MUST NOT register `/session status`, `/session inspect`, or `/session list` as separate canonical commands or aliases.

(Previously: The registry recognized `/session status` and `/session inspect` as canonical `/session` raw-args forms, but it did not include `/session list` in the recognized family forms.)

#### Scenario: List resolves as raw args of `/session`

- GIVEN the built-in slash command registry includes the session discoverability family
- WHEN a runtime ingress parses `/session list`
- THEN the registry MUST resolve the invocation to the canonical `/session` descriptor
- AND the invocation delivered to the handler MUST preserve raw arguments equal to `list`
- AND the registry MUST NOT require a separate `/session list` registration.

#### Scenario: List is not registered as a standalone canonical command

- GIVEN the runtime exposes registry metadata for built-in slash commands
- WHEN another component inspects the registered canonical commands for the session discoverability family
- THEN the registry MUST expose `/session` as the only canonical family command
- AND `/session list` MUST remain discoverable only as a raw-args form of `/session`.

### Requirement: Centralized Dispatch Through the Pre-Execution Seam

The system MUST route `/resume`, `/suspend`, `/tldr`, `/compact`, and the canonical `/session` discoverability family through the existing registry-backed pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical production short-circuit seam for these commands. CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST submit recognized in-scope session commands to that shared seam. For this slice, recognized `/session` family forms are `/session`, `/session status`, `/session inspect`, and `/session list`; unsupported trailing text after `/session` MUST still be routed as the canonical `/session` invocation so the handler can return discoverability guidance. Production routing MUST NOT depend on transport-local direct-handler branches for those commands.

(Previously: Recognized `/session` family forms were limited to `/session`, `/session status`, and `/session inspect`.)

#### Scenario: Supported `/session` family forms use the shared seam

- GIVEN CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatcher execution, and channel-backed ingress each receive `/session`, `/session status`, `/session inspect`, or `/session list`
- WHEN the runtime classifies that recognized command input
- THEN each supported ingress surface MUST route the invocation through `pre_execution::evaluate_ingress(...)`
- AND the handled dispatch MUST come from the registry-backed ingress path rather than a transport-local branch.

#### Scenario: Unsupported `/session` subcommand stays inside the family handler boundary

- GIVEN a supported ingress surface receives `/session archive`
- WHEN the runtime evaluates ingress
- THEN the registry MUST still resolve the canonical `/session` command
- AND the invocation MUST be delivered with raw arguments equal to `archive`
- AND the transport MUST NOT reclassify it as unknown-command fallthrough before the `/session` handler evaluates it.
