# Delta for gateway

## ADDED Requirements

### Requirement: Shared Effective Rook Configuration Assembly

The system MUST define a first-class `RookConfig` model as the effective runtime configuration for
Rook within the gateway domain.

The system MUST assemble the effective configuration through one shared resolution path that is used
by both `serve` and `rook config export`.

That shared resolution path MUST apply configuration sources in this precedence order:

1. built-in defaults
2. configuration file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

The shared resolution path MUST validate the final effective configuration before it is used for
server startup or config export.

The system MUST NOT allow `serve` and `rook config export` to diverge in effective-value
resolution, precedence behavior, or validation outcome.

#### Scenario: serve and config export resolve the same effective configuration

- GIVEN the same built-in defaults, config file inputs, `ROOK_*` environment values, and CLI flag
  values
- WHEN Rook resolves configuration for `serve`
- AND Rook resolves configuration for `rook config export`
- THEN both commands MUST produce the same effective configuration values
- AND both commands MUST apply the same precedence order
- AND both commands MUST apply the same validation rules

#### Scenario: CLI overrides all lower-precedence sources

- GIVEN a built-in default bind port
- AND a config file sets a different bind port
- AND a `ROOK_*` environment override sets a third bind port
- AND a CLI flag sets a fourth bind port
- WHEN Rook resolves the effective configuration
- THEN the effective bind port MUST be the CLI-provided value
- AND the result MUST reflect the precedence order defaults < file < environment < CLI

#### Scenario: environment overrides file values when CLI does not override them

- GIVEN a config file sets a database path
- AND a `ROOK_*` environment override sets a different database path
- AND no CLI flag overrides the database path
- WHEN Rook resolves the effective configuration
- THEN the effective database path MUST be the environment-provided value

### Requirement: `ROOK_*` Environment Override Contract

The system MUST support operator-facing configuration overrides through documented `ROOK_*`
environment variables.

Each supported `ROOK_*` environment variable MUST map deterministically onto the corresponding
`RookConfig` field or sub-field it overrides.

Environment override behavior MUST be documented for operators, including the variable naming
scheme and its position in the precedence order.

Verification for this change MUST include automated coverage that demonstrates supported
`ROOK_*` overrides affect the effective configuration as documented.

#### Scenario: documented environment override is applied to effective configuration

- GIVEN a supported `ROOK_*` environment variable is documented for a specific configuration field
- AND the config file provides a different value for that field
- WHEN Rook resolves the effective configuration
- THEN the effective configuration MUST use the environment-provided value for that field

#### Scenario: unsupported environment variable does not create ambiguous configuration

- GIVEN an environment variable outside the documented supported `ROOK_*` override contract
- WHEN Rook resolves the effective configuration
- THEN the system MUST NOT treat that variable as a valid override for an unrelated config field
- AND operator-facing documentation MUST remain the source of truth for supported overrides

### Requirement: Redacted Effective Config Export

The system MUST provide `rook config export` as an operator-visible command that outputs the
validated effective configuration.

`rook config export` MUST render the effective configuration derived from the same shared assembly
path used by `serve`.

Config export MUST protect secret-bearing values using redacted or presence-only semantics.

Config export MUST NOT expose raw inbound bearer tokens, provider API keys, authorization header
values, cookies, or equivalent secret-bearing material.

When config export needs to communicate secret state, it MUST report only configured, enabled,
present, absent, or an equivalent redacted state rather than the raw secret value.

#### Scenario: config export shows effective non-secret values and redacts secrets

- GIVEN effective configuration contains non-secret runtime settings and one or more configured
  secret-bearing values
- WHEN an operator runs `rook config export`
- THEN the output MUST include the effective non-secret values
- AND the output MUST represent secret-bearing fields only with redacted or presence-only state
- AND the output MUST NOT reveal the raw secret values

#### Scenario: config export preserves gateway bind posture reporting without leaking secrets

- GIVEN the effective configuration resolves the gateway bind host and port
- AND inbound auth or provider credentials are also configured
- WHEN an operator runs `rook config export`
- THEN the output MUST report the effective bind target consistently with the gateway domain
- AND the output MUST continue to redact secret-bearing fields

### Requirement: Invalid Configuration Fails Closed With Operator-Facing Messages

The system MUST fail closed when the effective configuration is invalid after applying defaults,
file inputs, environment overrides, and CLI overrides.

Validation failure MUST prevent server startup and MUST prevent successful config export.

Validation failure output MUST be operator-facing and clear enough to identify the invalid
configuration area and the reason the configuration cannot be used.

The system MUST NOT continue with partially applied or partially validated configuration.

#### Scenario: invalid effective configuration blocks startup

- GIVEN effective configuration inputs resolve to an invalid state required for gateway startup
- WHEN Rook loads configuration for `serve`
- THEN configuration validation MUST fail before the server starts
- AND the command MUST return a non-success result
- AND the operator-facing error MUST identify the invalid configuration area

#### Scenario: invalid effective configuration blocks config export

- GIVEN effective configuration inputs resolve to an invalid state
- WHEN an operator runs `rook config export`
- THEN configuration validation MUST fail before export output is produced as a successful result
- AND the command MUST return a non-success result
- AND the operator-facing error MUST clearly explain why the configuration is invalid

### Requirement: Explicit Precedence Verification and Documentation

The system MUST make configuration precedence explicit in operator-facing documentation for this
change.

The documented precedence order MUST be defaults < file < environment < CLI.

Verification for this change MUST include automated tests that assert precedence behavior across at
least defaults, file inputs, `ROOK_*` environment overrides, and CLI flags.

Verification for this change MUST include coverage that confirms config export redaction behavior
and invalid-configuration fail-closed behavior.

#### Scenario: precedence documentation matches implemented behavior

- GIVEN operator-facing documentation for Rook configuration inputs
- WHEN the documentation describes configuration precedence
- THEN it MUST state the order defaults < file < environment < CLI
- AND that documented order MUST match the behavior verified by automated tests

#### Scenario: automated verification catches precedence regressions

- GIVEN automated tests for layered configuration resolution
- WHEN a lower-precedence source would incorrectly override a higher-precedence source
- THEN the relevant precedence verification MUST fail
- AND the failure MUST identify that implemented precedence drifted from the documented contract
