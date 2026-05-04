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

### Requirement: Cerebro restart and storage recovery behavior is explicit

Production deployments of Cerebro MUST validate restart and storage recovery behavior for the supported single-node, local-first storage posture. The supported durable posture is embedded or disk-backed node-local storage; `remote_surreal`, shared remote persistence, and HA multi-node durability remain unsupported in this build.

#### Scenario: Clean restart preserves embedded storage records

- Given Cerebro is running with `storage_mode = "embedded_surreal"` and a durable `surreal.storage_path` or `storage_path`
- And a memory write has completed successfully
- When Cerebro is stopped cleanly and started again with the same storage path
- Then `/readyz` returns success after startup
- And the committed memory remains available through storage-backed MCP tools

#### Scenario: Crash-equivalent restart preserves committed embedded storage records

- Given Cerebro has completed memory writes in embedded storage
- When the process exits unexpectedly after those writes have completed
- And Cerebro starts again with the same storage path
- Then committed records remain available
- And writes that had not completed before the crash are not guaranteed to persist

#### Scenario: Storage readiness degradation removes the instance from service

- Given Cerebro is running but the storage readiness check fails
- When an operator or orchestrator probes `/readyz`
- Then Cerebro returns `503 Service Unavailable` with a storage-unavailable response
- And `/healthz` remains available for process liveness checks
- And operators remove the instance from traffic until storage readiness recovers

#### Scenario: Storage initialization fallback is treated as degraded durability

- Given embedded storage cannot initialize
- When no storage fallback is configured
- Then Cerebro startup fails clearly rather than serving with an unknown storage state
- When `storage_fallback = "in_memory"` is configured
- Then Cerebro may start on the fallback backend
- But operators MUST treat the instance as durability-degraded and recover or restore the durable storage path before returning it to normal production service
