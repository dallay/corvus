# Delta for Agent Runtime Providers

## ADDED Requirements

### Requirement: Provider Account Pool Configuration

The system MUST support configuring a provider account pool under reliability settings, where each
pool entry includes the target provider identifier and its credentials, plus optional metadata
such as api_url and weight.

#### Scenario: Configure a multi-account pool

- GIVEN a reliability configuration with multiple account entries for the same provider
- WHEN the runtime loads and validates the configuration
- THEN the system MUST accept the pool configuration
- AND the system MUST make the pool available for provider selection.

#### Scenario: Reject malformed pool entries

- GIVEN a reliability configuration with a pool entry missing required provider or credential
  fields
- WHEN the runtime validates the configuration
- THEN the system MUST reject the configuration with a validation error.

### Requirement: Pool Selection and Per-Request Credentials

The system MUST select a pool account for each request using the configured strategy. The default
strategy MUST be round-robin when multiple accounts are present. The system MUST apply the selected
account credentials to the provider instance used for that request.

#### Scenario: Round-robin selection across accounts

- GIVEN a provider pool with two valid accounts and the default selection strategy
- WHEN two consecutive requests are processed
- THEN the system MUST select different accounts in round-robin order
- AND the provider for each request MUST use the selected account credentials.

#### Scenario: Single account pool behaves deterministically

- GIVEN a provider pool with a single valid account
- WHEN multiple requests are processed
- THEN the system MUST always select that account
- AND the provider credentials MUST match that account for every request.

### Requirement: Account-Aware Provider Reuse

The system MUST cache or reuse provider instances in a way that preserves account boundaries, so
credentials from one account MUST NOT be used for another account's requests.

#### Scenario: Provider instances stay bound to accounts

- GIVEN a provider pool with two accounts
- WHEN a request selects account A and a later request selects account B
- THEN the system MUST NOT reuse an account A provider instance for account B
- AND each request MUST execute with the credentials of its selected account.

### Requirement: Backward Compatibility Without Pool

The system MUST preserve current reliability behavior when no account pool is configured, using
the existing single-account provider settings.

#### Scenario: Pool omitted from configuration

- GIVEN a reliability configuration without any account pool
- WHEN a request is processed
- THEN the system MUST use the existing provider configuration without pooling
- AND behavior MUST match the previous single-account reliability flow.

### Requirement: Secret Handling for Pooled Credentials

The system MUST encrypt pooled credentials at rest and MUST redact them in logs, diagnostics, and
admin-config responses.

#### Scenario: Redacted admin read of pooled credentials

- GIVEN pooled credentials stored in the configuration
- WHEN the admin config API returns the reliability configuration
- THEN the system MUST redact credential values
- AND the response MUST NOT expose decrypted secrets.

### Requirement: Admin Config Exposure Controls

The admin HTTP configuration interface MAY expose read/patch access to the provider account pool
only when explicitly enabled. When disabled, the admin interface MUST reject pool read/patch
attempts.

#### Scenario: Admin exposure disabled

- GIVEN admin config exposure for provider pools is disabled
- WHEN a client requests or patches pool settings via the admin API
- THEN the system MUST reject the request
- AND the system MUST NOT return pool configuration details.

#### Scenario: Admin exposure enabled with validation

- GIVEN admin config exposure for provider pools is enabled
- WHEN a client submits a pool patch with invalid entries
- THEN the system MUST reject the patch with a validation error
- AND the system MUST leave existing pool configuration unchanged.
