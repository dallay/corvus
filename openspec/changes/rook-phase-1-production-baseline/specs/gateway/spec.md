# Delta for gateway

## ADDED Requirements

### Requirement: Effective Rook Configuration Assembly and Export

The system MUST provide a single effective configuration assembly path for Rook runtime startup,
operator diagnostics, and operator-visible config export within the gateway domain.

The effective configuration MUST apply sources in this precedence order:

1. built-in defaults
2. configuration file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

The system MUST support `rook config export` as an operator-visible command that returns the
effective configuration after precedence resolution and validation.

The export output MUST be deterministic for the same effective inputs and MUST be safe for
operator visibility.

The export output MUST include Phase 1 runtime concerns needed by startup and diagnostics,
including at minimum server bind configuration, database path, inbound auth configuration state,
transport configuration, rate-limit configuration, and idempotency configuration.

The system MUST validate the effective configuration before using it for server startup, doctor
checks, or config export. Invalid effective configuration MUST fail closed with a non-success
result and a readable validation error.

The system MUST use the same effective configuration assembly behavior for `serve`, `rook doctor`,
and `rook config export`; these commands MUST NOT diverge in precedence, validation, or redaction
behavior.

#### Scenario: config export reflects precedence across defaults, file, environment, and CLI

- GIVEN built-in defaults define a server bind port
- AND a config file sets a different server bind port
- AND `ROOK_*` environment overrides set a third server bind port
- AND CLI flags set a fourth server bind port
- WHEN an operator runs `rook config export`
- THEN the exported effective configuration MUST report the CLI-provided port
- AND the result MUST reflect the precedence order defaults < file < environment < CLI

#### Scenario: config export uses environment overrides when CLI does not override them

- GIVEN a config file sets a database path
- AND `ROOK_*` environment overrides set a different database path
- AND no CLI flag overrides the database path
- WHEN an operator runs `rook config export`
- THEN the exported effective configuration MUST report the environment-provided database path

#### Scenario: serve and config export share the same effective configuration

- GIVEN a config file and `ROOK_*` environment overrides together define the effective runtime
  configuration
- WHEN the server starts with `serve`
- AND an operator runs `rook config export` under the same inputs
- THEN both code paths MUST resolve the same effective configuration values
- AND neither path MUST apply a different precedence or validation rule

#### Scenario: invalid effective configuration fails closed before startup or export

- GIVEN effective configuration inputs contain an invalid value for a required Phase 1 runtime
  setting
- WHEN the server loads configuration for `serve` or `rook config export`
- THEN configuration validation MUST fail with a readable error
- AND the command MUST return a non-success result
- AND the server MUST NOT start with partially applied configuration

### Requirement: Operator-Visible Config Export Redaction

The system MUST treat `rook config export` as an operator-visible gateway surface and MUST redact
or reduce secret-bearing fields to presence-only state.

Config export MUST NOT expose raw inbound bearer tokens, provider API keys, authorization header
values, cookies, pairing codes, or equivalent secret-bearing material.

When a secret is configured, config export SHOULD indicate presence or enabled state rather than
echoing the raw value.

#### Scenario: config export redacts inbound auth secrets

- GIVEN inbound auth is enabled with a configured non-empty bearer token
- WHEN an operator runs `rook config export`
- THEN the export MAY report that inbound auth is enabled or configured
- AND it MUST NOT expose the raw bearer token value

#### Scenario: config export redacts provider credentials

- GIVEN effective configuration contains one or more provider credentials needed for runtime
- WHEN an operator runs `rook config export`
- THEN the export MAY report credential presence or enabled state
- AND it MUST NOT expose raw provider credential values

### Requirement: Rook Doctor Deterministic Diagnostics

The system MUST provide a `rook doctor` command for operator diagnostics in the gateway domain.

`rook doctor` MUST evaluate deterministic local checks against the same effective configuration
used by runtime startup.

Phase 1 doctor coverage MUST include at minimum:

- effective configuration load and validation
- database path usability and migration/open readiness
- embedded dashboard or admin asset availability required by the process
- inbound auth configuration consistency

The doctor command MUST classify each check as `pass`, `warn`, or `fail` and MUST include for each
check a machine-readable status, a short check name, and a human-readable explanation.

The doctor command MUST return a non-zero exit status when any required check fails.

The first Phase 1 doctor version MUST remain fast and deterministic and MUST NOT require live
upstream provider probing.

#### Scenario: doctor succeeds when all required local checks pass

- GIVEN effective configuration is valid
- AND the configured database path can be opened and any required migrations can run
- AND required embedded assets are available
- AND inbound auth configuration is internally consistent
- WHEN an operator runs `rook doctor`
- THEN the command MUST report all required checks with status `pass` or `warn`
- AND the command MUST return a zero exit status

#### Scenario: doctor fails when configuration is invalid

- GIVEN effective configuration fails validation
- WHEN an operator runs `rook doctor`
- THEN the output MUST include at least one check with status `fail`
- AND that failed check MUST identify configuration as the failing area
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when database path is unusable

- GIVEN effective configuration is otherwise valid
- AND the configured database path cannot be opened or prepared for runtime use
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a database-related check with status `fail`
- AND the explanation MUST indicate that the database path is not usable
- AND the command MUST return a non-zero exit status

#### Scenario: doctor does not depend on live upstream network health

- GIVEN effective configuration is valid for local startup
- AND external upstream AI providers are unreachable
- WHEN an operator runs the default Phase 1 `rook doctor`
- THEN the command MUST still complete using deterministic local checks
- AND upstream reachability MUST NOT be required for overall success in this phase

### Requirement: Readiness and Liveness Health Endpoints

The system MUST expose distinct liveness and readiness health semantics for the admin surface.

The system MUST expose a liveness endpoint and a readiness endpoint under the `/api/health/*`
namespace.

For Phase 1, the liveness endpoint MUST report whether the Rook process is running and capable of
serving the event loop. Liveness MUST NOT depend on database reachability, provider reachability,
or account-level routing state.

For Phase 1, the readiness endpoint MUST report whether critical local dependencies required to
serve traffic are available.

Readiness MUST evaluate at minimum:

- effective configuration validation success
- database open or initialization success required for serving
- router availability
- embedded assets or other local runtime resources required by the process

Readiness MUST NOT require all upstream AI providers to be reachable in Phase 1.

Both health endpoints MUST return structured JSON responses with stable semantics suitable for
orchestration.

#### Scenario: liveness is healthy while process is running

- GIVEN the Rook process is running
- WHEN a client requests the liveness endpoint
- THEN the response status MUST be successful
- AND the JSON body MUST indicate a live state
- AND the result MUST NOT depend on database or upstream provider reachability

#### Scenario: readiness is healthy after valid startup

- GIVEN the Rook process has completed startup with valid effective configuration
- AND the required database and local runtime resources are available
- WHEN a client requests the readiness endpoint
- THEN the response status MUST be successful
- AND the JSON body MUST indicate a ready state

#### Scenario: readiness fails when a critical local dependency is unavailable

- GIVEN the Rook process cannot satisfy a critical local serving dependency such as configuration
  validation or database initialization
- WHEN a client requests the readiness endpoint
- THEN the response status MUST be non-success
- AND the JSON body MUST indicate not-ready state
- AND the response MUST identify at least one failing readiness dependency

#### Scenario: readiness does not fail solely because upstream providers are unreachable

- GIVEN the Rook process has valid local startup state
- AND one or more upstream AI providers are unreachable
- WHEN a client requests the readiness endpoint
- THEN readiness MUST continue to report ready for Phase 1
- AND upstream provider reachability MUST NOT be required by this requirement

### Requirement: Existing Base Health Endpoint Compatibility

The existing `GET /api/health` admin route MUST remain available for compatibility during Phase 1.

If distinct readiness and liveness routes are added, `GET /api/health` MUST continue to return a
successful lightweight health response or a documented compatibility view, and it MUST NOT be
removed by this change.

#### Scenario: existing base health endpoint remains available after readiness/liveness are added

- GIVEN Phase 1 readiness and liveness endpoints are implemented
- WHEN a client requests `GET /api/health`
- THEN the route MUST still exist
- AND the response MUST remain successful for a healthy running process

### Requirement: Baseline Metrics Exposure for Gateway Operations

The system MUST expose a production metrics surface for the gateway domain in Phase 1.

The metrics surface MUST be reachable through one explicit operator-facing endpoint suitable for
scraping or inspection.

The Phase 1 metrics baseline MUST include at minimum:

- total requests partitioned by route surface, endpoint, or status class
- request duration metrics for core gateway and admin request paths
- rate-limit rejection counts
- idempotency replay, conflict, and pass counts
- upstream request outcome counts

The metrics surface SHOULD be scrape-friendly for operators.

Instrumentation MUST be attached through stable middleware, transport hooks, or gateway helper
boundaries where available; it MUST NOT require one-off per-handler duplication to satisfy the
baseline contract.

The metrics surface MUST be observable without requiring operator access to application logs.

#### Scenario: metrics endpoint is available for operators

- GIVEN a running Rook server with Phase 1 observability enabled
- WHEN an operator requests the metrics endpoint
- THEN the server MUST return a successful metrics response
- AND the response MUST include metric families for the Phase 1 baseline

#### Scenario: request metrics increment for core routed traffic

- GIVEN a running Rook server
- WHEN a client successfully calls a core route such as `/api/*`, `/v1/models`, or
  `/v1/chat/completions`
- THEN the metrics surface MUST reflect an incremented request count
- AND the emitted metrics MUST include latency or duration data for the request class

#### Scenario: rate-limit and idempotency outcomes are observable in metrics

- GIVEN a request is rejected by rate limiting
- AND another request exercises idempotency replay or conflict behavior
- WHEN an operator inspects the metrics surface
- THEN the metrics MUST include a counter increment for the rate-limit rejection
- AND the metrics MUST include the corresponding idempotency outcome increments

#### Scenario: upstream outcomes are observable without reading logs

- GIVEN the gateway performs upstream requests that result in success and failure outcomes
- WHEN an operator inspects the metrics surface
- THEN the metrics MUST expose upstream outcome counts by result type
- AND the operator MUST NOT need to infer these counts exclusively from logs
