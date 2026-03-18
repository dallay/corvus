# Delta for Cerebro

## ADDED Requirements

### Requirement: In-Process TUI Toggle

The Cerebro service MUST provide an in-process TUI that can be enabled or disabled via a feature
flag and via configuration or CLI toggle.

#### Scenario: TUI enabled by flag (happy path)

- GIVEN a Cerebro service configured with the TUI feature flag enabled
- WHEN the service starts with the TUI toggle set to enabled
- THEN the TUI starts in-process
- AND MCP requests remain available

#### Scenario: TUI disabled by configuration (edge case)

- GIVEN a Cerebro service with the TUI feature flag enabled
- WHEN the service starts with the TUI toggle set to disabled
- THEN the TUI does not start
- AND MCP requests remain available

### Requirement: MCP Remains Non-Blocking

When the TUI is enabled, MCP request handling MUST remain non-blocking and MUST NOT depend on the
TUI event loop.

#### Scenario: MCP remains responsive with TUI running (happy path)

- GIVEN the TUI is enabled and running
- WHEN a client sends MCP tool calls
- THEN the MCP responses are processed and returned without waiting on the TUI

#### Scenario: TUI stalls (edge case)

- GIVEN the TUI event loop becomes unresponsive
- WHEN a client sends MCP tool calls
- THEN the MCP responses are still processed
- AND the service does not block on the TUI

### Requirement: TUI View Availability

When the TUI is enabled, it MUST provide the following views: dashboard, memory explorer, session
timeline, and live tool-call stream.

#### Scenario: Views available (happy path)

- GIVEN the TUI is enabled
- WHEN an operator navigates the TUI
- THEN the dashboard, memory explorer, session timeline, and live tool-call stream views are
  available

#### Scenario: View missing (edge case)

- GIVEN the TUI is enabled
- WHEN the operator attempts to open a required view
- THEN the TUI returns a visible error indicating the view is unavailable

### Requirement: TUI Data Redaction

The TUI MUST redact sensitive data from all views using the same classification guidance applied
to MCP operations. Redaction MUST apply to secrets, credentials, and PII before rendering.

#### Scenario: Sensitive fields are redacted (happy path)

- GIVEN a memory record contains secret or PII content
- WHEN the record is displayed in any TUI view
- THEN the sensitive fields are redacted
- AND the redaction is visible in the rendered output

#### Scenario: Unknown data classification (edge case)

- GIVEN a memory record contains fields with unknown sensitivity
- WHEN the record is displayed in the TUI
- THEN the TUI defaults to redacting fields that are not explicitly safe

### Requirement: Graceful TUI Shutdown

The TUI MUST shut down gracefully without interrupting MCP availability and MUST release terminal
control on exit.

#### Scenario: Operator exits TUI (happy path)

- GIVEN the TUI is running
- WHEN the operator requests exit
- THEN the TUI closes cleanly
- AND MCP continues to serve requests

#### Scenario: TUI crashes (edge case)

- GIVEN the TUI process encounters a fatal error
- WHEN the error occurs
- THEN the TUI exits without corrupting terminal state
- AND MCP continues to serve requests

### Requirement: No New Network Endpoints

The optional TUI MUST NOT introduce new network endpoints or listeners beyond the existing MCP
surface.

#### Scenario: TUI enabled without new listeners (happy path)

- GIVEN the TUI is enabled
- WHEN the service starts
- THEN only the existing MCP endpoint is bound

#### Scenario: Unexpected listener detected (edge case)

- GIVEN the TUI is enabled
- WHEN a non-MCP listener is detected at startup
- THEN the service fails startup with a structured error

## MODIFIED Requirements

### Requirement: Optional TUI Surface

The Cerebro distribution MAY include an in-process TUI; when enabled, it MUST provide the
following views: dashboard, memory explorer, session timeline, and live tool-call stream, and it
MUST remain optional and non-blocking for MCP availability.

(Previously: The Cerebro distribution MAY include a TUI; when enabled, it MUST provide the
following views: dashboard, memory explorer, session timeline, live tool-call stream.)

#### Scenario: TUI enabled (happy path)

- GIVEN a Cerebro deployment with the TUI enabled
- WHEN the operator opens the TUI
- THEN the dashboard, memory explorer, session timeline, and live tool-call stream views are
  available
- AND MCP requests remain available

#### Scenario: TUI disabled (edge case)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN the operator attempts to open the TUI
- THEN the service starts without a UI and continues to serve MCP requests
