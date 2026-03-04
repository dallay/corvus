# Delta for Agent Configuration API

## ADDED Requirements

### Requirement: Comprehensive Configuration Payload Support

The backend configuration update payload (`AdminConfigUpdateRequest`) MUST support all fields defined in the system's `config.toml`.
The backend MUST securely accept updates to server settings, agent identity details, and LLM provider configurations.

#### Scenario: Full configuration update via API

- GIVEN an authenticated admin client
- WHEN a POST request is sent to the configuration update endpoint with a complete JSON payload representing all `config.toml` fields
- THEN the server validates the payload
- AND successfully updates the running configuration and persists it to `config.toml`
- AND responds with a 200 OK status containing the updated `AdminConfigView`

### Requirement: Secure Gateway Pairing Payload

The backend MUST provide dedicated endpoints and payloads for handling gateway connection pairing.

#### Scenario: Secure pairing token submission

- GIVEN an unpaired agent runtime
- WHEN a valid pairing request with a cryptographic token is received
- THEN the system validates the token against the gateway
- AND securely stores the gateway credentials
- AND establishes a persistent connection to the gateway

## MODIFIED Requirements

### Requirement: Admin Configuration View (Previously: Partial config view)

The `AdminConfigView` payload MUST return a comprehensive representation of the current configuration, matching the expanded scope of the update request, while stripping any sensitive credentials (e.g., raw API keys).

#### Scenario: Fetching current configuration

- GIVEN an authenticated admin client
- WHEN a GET request is made to the configuration endpoint
- THEN the server returns an `AdminConfigView` object containing all public configuration fields from `config.toml`
- AND sensitive fields (like `provider_api_key`) are omitted or masked
