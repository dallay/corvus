# Delta for Gateway

## ADDED Requirements

### Requirement: Linux Release Binary Startup Smoke Validation

The release workflow for the Linux Cerebro binary MUST execute a startup smoke validation that proves the built release artifact can start the real HTTP and MCP service surface, not merely parse CLI arguments or print help text.

This validation MUST run the produced release binary with a temporary CI-specific configuration that defines explicit loopback binding, an explicit non-embedded storage mode suitable for CI, and a deterministic inbound bearer token for MCP authentication.

The smoke validation MUST start the service as a background process, poll for startup within a bounded timeout, capture service logs for diagnostics, and terminate the process before the workflow step exits on both success and failure.

The smoke validation scope for this change MUST apply at least to the Linux release build path.

#### Scenario: Linux release artifact starts real service surface

- GIVEN the Linux release workflow has built a Cerebro release binary
- AND the workflow prepares a temporary configuration with explicit loopback binding, explicit CI-safe storage mode, and a known bearer token
- WHEN the workflow launches the built binary for the smoke validation
- THEN the binary MUST start the real HTTP and MCP service surface
- AND the workflow MUST treat startup as successful only after the service responds within the configured timeout
- AND the workflow MUST terminate the background process before the workflow step exits

#### Scenario: Startup failure surfaces diagnostics and cleanup

- GIVEN the Linux release workflow has launched the Cerebro release binary for smoke validation
- WHEN the service fails to become reachable within the configured timeout or exits prematurely
- THEN the workflow MUST fail the smoke validation
- AND the workflow MUST emit captured service logs for diagnosis
- AND the workflow MUST attempt to terminate any remaining background process before the step exits

### Requirement: Release Smoke Health and Readiness Probes

The Linux release startup smoke validation MUST verify that the temporary Cerebro service exposes basic operational probes after startup.

The smoke validation MUST assert all of the following against the started release binary:

- `GET /healthz` returns HTTP `200`
- `GET /readyz` returns the readiness outcome expected for the temporary CI configuration

The readiness expectation for the temporary configuration MUST be evaluated against the service state established by the explicit CI config rather than default local-deployment assumptions.

#### Scenario: Health and readiness probes pass for CI startup

- GIVEN the Linux release smoke validation has started the Cerebro release binary with the temporary CI configuration
- WHEN the workflow probes `GET /healthz` and `GET /readyz`
- THEN `GET /healthz` MUST return HTTP `200`
- AND `GET /readyz` MUST return the readiness outcome expected for that temporary configuration

#### Scenario: Readiness mismatch fails smoke validation

- GIVEN the Linux release smoke validation has started the Cerebro release binary
- WHEN `GET /readyz` does not match the expected readiness outcome for the temporary CI configuration
- THEN the workflow MUST fail the smoke validation
- AND the failure MUST be reported even if `GET /healthz` succeeded

### Requirement: Release Smoke MCP Authentication Contract

The Linux release startup smoke validation MUST verify Cerebro's MCP authentication posture against the real started service.

The smoke validation MUST assert both of the following:

- an unauthenticated `POST /mcp` request is rejected
- an authenticated `POST /mcp` request using `Authorization: Bearer <configured-token>` succeeds for a minimal valid probe

For this change, the minimal valid authenticated probe SHOULD use a low-flake discovery request such as `tools/list`.

The authenticated success check MUST validate a minimal valid JSON-RPC response contract rather than deeper tool semantics.

#### Scenario: Unauthenticated MCP request is rejected

- GIVEN the Linux release smoke validation has started the Cerebro release binary with MCP bearer authentication configured
- WHEN the workflow sends `POST /mcp` without an authorization header
- THEN the service MUST reject the request
- AND the workflow MUST fail if the unauthenticated request is accepted

#### Scenario: Authenticated MCP discovery request succeeds

- GIVEN the Linux release smoke validation has started the Cerebro release binary with bearer authentication configured
- AND the workflow has the configured bearer token for the temporary CI configuration
- WHEN the workflow sends an authenticated `POST /mcp` request for a minimal valid JSON-RPC probe such as `tools/list`
- THEN the service MUST return a successful minimal valid JSON-RPC response
- AND the workflow MUST fail if the authenticated request is rejected or the response is not minimally valid
