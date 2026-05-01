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

Cerebro does not own request-frequency rate limiting inside the service. Production deployments that expose Cerebro beyond loopback MUST enforce rate limiting at ingress using a trusted client key such as source IP, mTLS identity, or authenticated gateway principal. Source-IP keys are acceptable only when ingress is the sole trusted hop and strips or validates `X-Forwarded-For` and other forwarding headers; otherwise, prefer non-spoofable identities such as mTLS identity or an authenticated gateway principal in production.

Recommended baseline: `60 requests/minute` per trusted client with burst controls, tuned for known automation workloads.

### Requirement: Exposed deployments define production alerts

Production deployments of Cerebro MUST define alerting guidance tied to the service metrics, readiness probes, and structured logs. The guidance MUST cover repeated readiness failures, unusual authentication failures, elevated server-side error rates, storage error spikes, and latency spikes.

Recommended internal production starting thresholds:

- readiness degradation: alert when `increase(cerebro_readiness_failures_total[5m]) >= 3` or `/readyz` probe success rate is below `95%` for 5 minutes;
- auth anomaly: alert when `increase(cerebro_auth_failures_total[10m]) > 20` or failures exceed `5x` the 24-hour baseline;
- server-side error rate: warn above `2%` and page above `5%` for `cerebro_requests_total{status=~"storage_error|internal_error"}` outcomes divided by all MCP requests;
- storage operation spike: alert when `increase(cerebro_storage_errors_total[5m]) >= 5` for one operation;
- latency spike: warn when p95 successful tool latency exceeds `1s`, and page when it exceeds `2s` for 10 minutes, using `histogram_quantile(0.95, sum by (le, tool) (rate(cerebro_tool_latency_seconds_bucket{status="ok"}[10m])))`.

#### Scenario: Operators are alerted before degradation becomes user-visible

- Given Cerebro is running in an internal production deployment
- When readiness failures, auth failures, server-side errors, storage errors, or tool latency exceed the documented thresholds
- Then the deployment's monitoring stack raises the corresponding alert
- And the alert identifies the metric signal and relevant structured logs to inspect
