# Delta for Slash Command Registry

## MODIFIED Requirements

### Requirement: Slash Session Discoverability Family Registration

The system MUST register `/session` as the only canonical slash command entry for the session discoverability family in this slice.

The `/session` descriptor MUST allow empty raw arguments so `/session` can remain the family help or usage hub without a required-text contract. The registry MUST treat `/session status` as the canonical `/session` command plus raw arguments equal to `status`, and it MUST treat `/session inspect` as the canonical `/session` command plus raw arguments equal to `inspect`. The registry MUST NOT register `/session status` or `/session inspect` as separate canonical commands or aliases.

(Previously: The `/session` descriptor allowed empty raw arguments for root help or usage, and the registry treated `/session status` as the canonical `/session` command plus raw arguments equal to `status`. The spec did not recognize `/session inspect` as a supported family form.)

#### Scenario: Root help resolves through the canonical family command

- GIVEN the built-in slash command registry includes the session discoverability family
- WHEN a runtime ingress parses `/session` with no trailing text
- THEN the registry MUST resolve the invocation to the canonical `/session` descriptor
- AND the invocation delivered to the handler MUST preserve empty raw arguments.

#### Scenario: Status resolves as raw args of `/session`

- GIVEN the built-in slash command registry includes the session discoverability family
- WHEN a runtime ingress parses `/session status`
- THEN the registry MUST resolve the invocation to the canonical `/session` descriptor
- AND the invocation delivered to the handler MUST preserve raw arguments equal to `status`
- AND the registry MUST NOT require a separate `/session status` registration.

#### Scenario: Inspect resolves as raw args of `/session`

- GIVEN the built-in slash command registry includes the session discoverability family
- WHEN a runtime ingress parses `/session inspect`
- THEN the registry MUST resolve the invocation to the canonical `/session` descriptor
- AND the invocation delivered to the handler MUST preserve raw arguments equal to `inspect`
- AND the registry MUST NOT require a separate `/session inspect` registration.

### Requirement: Centralized Dispatch Through the Pre-Execution Seam

The system MUST route `/resume`, `/suspend`, `/tldr`, `/compact`, and the canonical `/session` discoverability family through the existing registry-backed pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical production short-circuit seam for these commands. CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST submit recognized in-scope session commands to that shared seam. For this slice, recognized `/session` family forms are `/session`, `/session status`, and `/session inspect`; unsupported trailing text after `/session` MUST still be routed as the canonical `/session` invocation so the handler can return discoverability guidance. Production routing MUST NOT depend on transport-local direct-handler branches for those commands.

(Previously: For this slice, recognized `/session` family forms were `/session` and `/session status`; unsupported trailing text after `/session` was still routed as the canonical `/session` invocation so the handler could return discoverability guidance.)

#### Scenario: Supported `/session` family forms use the shared seam

- GIVEN CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatcher execution, and channel-backed ingress each receive `/session`, `/session status`, or `/session inspect`
- WHEN the runtime classifies that recognized command input
- THEN each supported ingress surface MUST route the invocation through `pre_execution::evaluate_ingress(...)`
- AND the handled dispatch MUST come from the registry-backed ingress path rather than a transport-local branch.

#### Scenario: Unsupported `/session` subcommand stays inside the family handler boundary

- GIVEN a supported ingress surface receives `/session archive`
- WHEN the runtime evaluates ingress
- THEN the registry MUST still resolve the canonical `/session` command
- AND the invocation MUST be delivered with raw arguments equal to `archive`
- AND the transport MUST NOT reclassify it as unknown-command fallthrough before the `/session` handler evaluates it.

Note: `/session list` is now a supported subcommand and is handled by the separate list slice with caller-scoped visibility.
