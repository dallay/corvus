# Delta for gateway

## MODIFIED Requirements

### Requirement: Shared Effective Rook Configuration Assembly

The system MUST define a first-class `RookConfig` model as the effective runtime configuration for
Rook within the gateway domain.

The system MUST assemble the effective configuration through one shared resolution path that is used
by `serve`, `rook doctor`, and `rook config export`.

That shared resolution path MUST apply configuration sources in this precedence order:

1. built-in defaults
2. configuration file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

The shared resolution path MUST validate the final effective configuration before it is used for
server startup, doctor diagnostics, or config export.

`rook doctor` MUST evaluate effective configuration through that same runtime-startup path rather
than through a parallel or reduced validation path.

The system MUST NOT allow `serve`, `rook doctor`, and `rook config export` to diverge in
effective-value resolution, precedence behavior, or validation outcome.

Operator-visible reporting derived from the effective configuration MUST identify the effective bind
target consistently with the gateway domain's loopback-first posture and MUST NOT imply that
loopback binding alone is an authentication mechanism.

(Previously: The shared resolution path was required only for `serve` and `rook config export`, and
operator diagnostics were specified only as using the same effective configuration in general terms.)

#### Scenario: serve and doctor use the same effective configuration validation path

- GIVEN the same built-in defaults, config file inputs, `ROOK_*` environment values, and CLI flag
  values
- WHEN Rook resolves configuration for `serve`
- AND Rook resolves configuration for `rook doctor`
- THEN both commands MUST produce the same effective configuration values
- AND both commands MUST apply the same precedence order
- AND both commands MUST apply the same validation rules

#### Scenario: doctor reports the effective bind target from startup-equivalent configuration

- GIVEN the effective configuration resolves a bind host and port
- WHEN an operator runs `rook doctor`
- THEN the diagnostics output MUST report the same effective bind target that `serve` would use
- AND the output MUST describe that bind target without treating loopback posture as sufficient
  authentication

---

### Requirement: Rook Doctor Deterministic Diagnostics

The system MUST provide a `rook doctor` command for operator diagnostics in the gateway domain.

Default `rook doctor` execution MUST remain deterministic and local-first.

Default `rook doctor` execution MUST evaluate startup-readiness checks against the same effective
configuration and local prerequisites used by runtime startup.

The default doctor command MUST NOT require live upstream provider reachability, remote account
verification, or other network-dependent checks in order to determine overall success.

Doctor coverage MUST include at minimum:

- effective configuration load and validation through the runtime-startup path
- database open and migration readiness sufficient for local service startup
- inbound auth configuration validation when inbound auth is enabled
- embedded dashboard asset availability required for the local admin/dashboard surface

The doctor command MUST classify each check as `pass`, `warn`, or `fail`.

Each reported check MUST include at minimum:

- a stable check name
- a machine-readable status
- a human-readable explanation
- actionable operator guidance when the status is `warn` or `fail`

A `fail` result MUST mean the checked condition prevents or invalidates correct local startup for the
default operational contract.

A `warn` result MUST mean the checked condition is advisory, degraded, or noteworthy but does not by
itself block correct local startup.

A `pass` result MUST mean the checked condition satisfied the local startup expectation being
validated.

The command MUST return a non-zero exit status when one or more required checks report `fail`.

The command MUST return a zero exit status when all required checks report only `pass` or `warn`.

The doctor command MUST keep secrets redacted in status output and MUST NOT expose raw inbound
bearer tokens, provider API keys, or equivalent secret-bearing values.

(Previously: The doctor contract required deterministic local checks with pass/warn/fail output and
non-zero exit on failure, but it did not tightly require startup-equivalent validation paths,
database migration readiness semantics, enabled-only inbound auth validation, or explicit actionable
result semantics.)

#### Scenario: doctor succeeds with passes and warnings only

- GIVEN effective configuration is valid through the runtime-startup validation path
- AND startup-equivalent database open and migration readiness succeeds
- AND required embedded dashboard assets are available
- AND inbound auth is either disabled or enabled with valid configuration
- AND one or more advisory conditions produce `warn` results only
- WHEN an operator runs `rook doctor`
- THEN every required blocking check MUST report `pass` or `warn`
- AND the command MUST return a zero exit status

#### Scenario: doctor fails when startup-equivalent configuration validation fails

- GIVEN effective configuration inputs resolve to a configuration that runtime startup would reject
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a configuration-related check with status `fail`
- AND that check MUST explain what configuration area is invalid
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when database readiness would block startup

- GIVEN effective configuration is otherwise valid
- AND the configured database cannot be opened, initialized, or migrated to the state required for
  local startup
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a database-related check with status `fail`
- AND the explanation MUST identify the database readiness problem in operator-actionable terms
- AND the command MUST return a non-zero exit status

#### Scenario: doctor validates inbound auth only when enabled

- GIVEN inbound auth enforcement is disabled in the effective configuration
- WHEN an operator runs `rook doctor`
- THEN the inbound auth diagnostic MUST NOT fail solely because no inbound bearer token is configured
- AND the command MAY report the auth check as `pass` or `warn` according to the disabled state

#### Scenario: doctor fails enabled inbound auth that startup would reject

- GIVEN inbound auth enforcement is enabled in the effective configuration
- AND the inbound bearer credential is missing, empty, or otherwise invalid for startup
- WHEN an operator runs `rook doctor`
- THEN the output MUST include an inbound-auth-related check with status `fail`
- AND the explanation MUST state that inbound auth is enabled but not correctly configured
- AND the output MUST NOT reveal the raw bearer token value
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when required dashboard assets are unavailable

- GIVEN effective configuration is otherwise valid
- AND the required embedded dashboard assets for the local admin/dashboard surface are unavailable
- WHEN an operator runs `rook doctor`
- THEN the output MUST include an asset-related check with status `fail`
- AND the explanation MUST identify that the dashboard/admin surface would be broken locally
- AND the command MUST return a non-zero exit status

#### Scenario: default doctor remains local and deterministic when remote providers are unreachable

- GIVEN effective configuration is valid for local startup
- AND one or more configured upstream providers are unreachable over the network
- WHEN an operator runs the default `rook doctor`
- THEN the command MUST complete using only deterministic local checks
- AND upstream reachability MUST NOT be required for a successful overall result

## ADDED Requirements

### Requirement: Optional Advisory Upstream Probe Mode

The system MAY provide an explicitly opt-in `rook doctor` mode that probes configured upstream
providers or other remote dependencies.

If such a mode is provided, it MUST be disabled by default.

Any remote or upstream probe performed by `rook doctor` MUST be clearly identified as advisory and
MUST remain separate from the default deterministic local readiness result.

Remote probe results MUST NOT change a successful default local readiness result into a required
failure solely because an upstream dependency is unreachable, slow, or otherwise unavailable.

Remote probe execution, if provided, SHOULD be bounded by explicit timeouts or equivalent limits so
that the command remains operationally predictable.

Remote probe output MUST communicate that the probe reflects optional connectivity or upstream state
rather than the baseline local startup contract.

#### Scenario: default doctor omits remote probes

- GIVEN Rook is configured with one or more upstream provider accounts
- WHEN an operator runs `rook doctor` without an explicit remote-probe opt-in
- THEN the command MUST NOT perform upstream reachability checks as part of the default run
- AND the overall result MUST be derived only from local deterministic diagnostics

#### Scenario: opt-in remote probe remains advisory

- GIVEN Rook provides an explicit opt-in mode for remote or upstream probing
- AND local deterministic doctor checks all report `pass`
- AND an opt-in upstream probe cannot reach a configured provider
- WHEN an operator runs `rook doctor` with that explicit opt-in enabled
- THEN the output MUST identify the upstream probe result as advisory
- AND the command MUST continue to distinguish the local readiness result from the remote probe
  outcome
- AND the unreachable upstream probe MUST NOT by itself redefine the default local readiness
  contract as failed
