# Gateway API Specification

> **Status:** Active

This specification records client-surface and ingress expectations for HTTP gateway-style entrypoints in Corvus.

## Cerebro MCP HTTP Surface

### Requirement: Cerebro MCP requests are process-bounded

Cerebro MUST protect `POST /mcp` with service-local controls that do not depend on external client identity:

- request body limit: `1 MiB`,
- request timeout: default `30s`, configurable with `request_timeout_secs`,
- in-flight concurrency cap: default `32`, configurable with `max_concurrent_mcp_requests`.

Health, readiness, and metrics endpoints MUST remain outside the MCP concurrency cap so operators can inspect service health during MCP saturation.

#### Scenario: Slow MCP request exceeds timeout

- Given Cerebro is running with `request_timeout_secs = 30`
- When a `POST /mcp` request takes longer than 30 seconds to process
- Then Cerebro returns `408 Request Timeout` or the deployment-equivalent timeout response
- And the request does not continue consuming MCP handler capacity indefinitely

#### Scenario: MCP concurrency is exhausted

- Given Cerebro is running with `max_concurrent_mcp_requests = 32`
- When more than 32 MCP requests are in flight
- Then excess MCP requests are rejected or shed with `503 Service Unavailable` or the deployment-equivalent overload response
- And `/healthz`, `/readyz`, and `/metrics` remain reachable

### Requirement: Exposed deployments rate-limit at ingress

Cerebro does not own request-frequency rate limiting inside the service. Production deployments that expose Cerebro beyond loopback MUST enforce rate limiting at ingress using a trusted client key such as source IP, mTLS identity, or authenticated gateway principal.

Recommended baseline: `60 requests/minute` per trusted client with burst controls, tuned for known automation workloads.
