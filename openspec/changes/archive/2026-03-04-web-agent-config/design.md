# Design: Web Agent Config

## Technical Approach

To enable comprehensive configuration of the agent runtime via the web dashboard, we will modularize
the monolithic `App.vue` into logically grouped Vue 3 components (e.g., General Settings, Security,
External Services, Logging). On the backend, we will expand `AdminConfigView` and
`AdminConfigUpdateRequest` in `clients/agent-runtime/src/gateway/admin.rs` to cover all nested
fields from `config.toml`, ensuring strict deserialization and secure handling of credentials (using
an "unchanged" | "replace" | "clear" strategy for secrets) before persisting them via the existing
configuration save mechanism.

## Architecture Decisions

### Decision: State Management in the Frontend

**Choice**: Use Vue 3's native Composition API (reactive/ref) with a centralized composable
pattern (e.g., `useConfigStore.ts`), rather than introducing Pinia.
**Alternatives considered**: Pinia (adds unnecessary boilerplate for a relatively flat, form-heavy
prototype). Prop-drilling (creates brittle and overly coupled components).
**Rationale**: The configuration state is primarily form data fetched once and synced on save. A
simple composable provides enough reactivity for nested configuration components to read and update
their specific slices of the configuration without adding external dependencies.

### Decision: Secret/Credential Handling Strategy

**Choice**: Use a specific `SecretMode` enum ("unchanged", "replace", "clear") alongside optional
string values in the payload for credentials (like API keys).
**Alternatives considered**: Sending raw passwords (insecure), sending masked passwords and
attempting to diff them (fragile and prone to accidental overwrites).
**Rationale**: This prevents the backend from ever exposing raw secrets to the frontend in
`GET /web/admin/config`. The frontend can explicitly dictate the intent (e.g., keep the existing API
key, replace it with a new one, or clear it out completely), ensuring secure serialization and
avoiding accidental deletion of keys during partial updates.

## Data Flow

    [Vue Dashboard] ── GET /web/admin/config ──→ [Axum Admin Gateway] ──→ [Config Loader] ──→ config.toml
          │                                              │
      (Edits made in modular Vue components)             │ (Strips secrets from payload)
          │                                              │
    [Vue Dashboard] ── PUT /web/admin/config ──→ [Axum Admin Gateway] ──→ [Config Validation] ──→ config.toml
    (Payload includes SecretModes for keys)       (Applies updates securely)

## File Changes

| File                                                                         | Action | Description                                                                                                                                                                                                       |
|------------------------------------------------------------------------------|--------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/web/apps/dashboard/src/App.vue`                                     | Modify | Strip out monolithic form layout; become a router/layout shell that imports modular configuration components.                                                                                                     |
| `clients/web/apps/dashboard/src/composables/useConfig.ts`                    | Create | Centralized composable for fetching, storing, and updating the configuration state.                                                                                                                               |
| `clients/web/apps/dashboard/src/components/config/GeneralSettings.vue`       | Create | Vue component for default provider, model, temperature, and memory backend settings.                                                                                                                              |
| `clients/web/apps/dashboard/src/components/config/SecuritySettings.vue`      | Create | Vue component for runtime kinds, autonomy levels, and gateway authentication settings.                                                                                                                            |
| `clients/web/apps/dashboard/src/components/config/ObservabilitySettings.vue` | Create | Vue component for logging, metrics, and telemetry configuration.                                                                                                                                                  |
| `clients/agent-runtime/src/gateway/admin.rs`                                 | Modify | Expand `AdminConfigView` and `AdminConfigUpdateRequest` structs to encompass all `config.toml` sections (Observability, Runtime, Autonomy, Gateway, Scheduler). Handle `SecretMode` securely in the PUT endpoint. |
| `clients/agent-runtime/src/config/mod.rs`                                    | Modify | Enhance validation logic to ensure that updates originating from `AdminConfigUpdateRequest` conform to the strict schema before invoking `save()`.                                                                |

## Interfaces / Contracts

## Config Coverage Matrix

| `config.toml` Area                                                  | AdminConfigView                                                                              | AdminConfigUpdateRequest                      | Editability           | Notes                                               |
|---------------------------------------------------------------------|----------------------------------------------------------------------------------------------|-----------------------------------------------|-----------------------|-----------------------------------------------------|
| `default_provider`                                                  | `default_provider`                                                                           | `default_provider`                            | Editable              | Trimmed; empty clears value.                        |
| `default_model`                                                     | `default_model`                                                                              | `default_model`                               | Editable              | Trimmed; empty clears value.                        |
| `api_url`                                                           | `api_url`                                                                                    | `api_url`                                     | Editable              | Trimmed; empty clears value.                        |
| `default_temperature`                                               | `default_temperature`                                                                        | `default_temperature`                         | Editable              | Range validated `[0.0, 2.0]`.                       |
| `api_key`                                                           | `provider.has_api_key`                                                                       | `provider.api_key` (`SecretMode`)             | Editable, secret-safe | Redacted in view; supports unchanged/replace/clear. |
| `memory.backend`                                                    | `memory_backend`, `memory.backend`                                                           | `memory_backend`, `memory.backend`            | Editable              | Validated against allowed backends.                 |
| `memory.surreal.url`                                                | `memory.surreal.url`                                                                         | `memory.surreal.url`                          | Editable              | Optional string normalization.                      |
| `memory.surreal.namespace`                                          | `memory.surreal.namespace`                                                                   | `memory.surreal.namespace`                    | Editable              | Optional string normalization.                      |
| `memory.surreal.database`                                           | `memory.surreal.database`                                                                    | `memory.surreal.database`                     | Editable              | Optional string normalization.                      |
| `memory.surreal.allow_http_loopback`                                | `memory.surreal.allow_http_loopback`                                                         | `memory.surreal.allow_http_loopback`          | Editable              | Boolean patch.                                      |
| `memory.surreal.username/password/token`                            | `memory.surreal.has_*` flags                                                                 | `memory.surreal.*` (`SecretMode`)             | Editable, secret-safe | Values redacted in view.                            |
| `observability.backend`                                             | `observability.backend`                                                                      | `observability.backend`                       | Editable              | Validated enum.                                     |
| `observability.otel_*`                                              | `observability.otel_endpoint`, `observability.otel_service_name`                             | matching fields                               | Editable              | Optional string normalization.                      |
| `runtime.kind`                                                      | `runtime.kind`                                                                               | `runtime.kind`                                | Editable              | Validated enum (`native`, `docker`).                |
| `autonomy.*` primary limits                                         | `autonomy.level/workspace_only/max_actions_per_hour/max_cost_per_day_cents`                  | matching fields                               | Editable              | Type/range checked.                                 |
| `autonomy` policy flags/lists                                       | `require_approval_for_medium_risk`, `block_high_risk_commands`, `auto_approve`, `always_ask` | matching fields                               | Editable              | Full patch support.                                 |
| `identity.format`                                                   | `identity.format`                                                                            | `identity.format`                             | Editable              | Validated enum (`openclaw`, `aieos`).               |
| `identity.aieos_path`                                               | `identity.aieos_path`                                                                        | `identity.aieos_path`                         | Editable              | Optional normalization.                             |
| `identity.aieos_inline`                                             | `identity.has_aieos_inline`                                                                  | —                                             | Hidden/non-editable   | Presence only; raw content never returned.          |
| `scheduler.*`                                                       | `scheduler.enabled/max_tasks/max_concurrent`                                                 | matching fields                               | Editable              | `max_* >= 1` validation.                            |
| `gateway.*` security/runtime limits                                 | `gateway.*` (incl. token count)                                                              | matching fields                               | Editable              | Deterministic field-level validation messages.      |
| `gateway.paired_tokens`                                             | `gateway.paired_tokens_count`                                                                | —                                             | Hidden/non-editable   | Count only; tokens never exposed.                   |
| `channels.cli`                                                      | `channels.cli`                                                                               | `channels.cli`                                | Editable              | Boolean patch support.                              |
| `channels.webhook.port`                                             | `channels.webhook.port`                                                                      | `channels.webhook.port`                       | Editable              | Port validation.                                    |
| `channels.webhook.secret`                                           | `channels.webhook.has_secret`                                                                | `channels.webhook.secret` (`SecretMode`)      | Editable, secret-safe | Redacted intent model.                              |
| `channels.webhook.enabled`                                          | `channels.webhook.enabled`                                                                   | `channels.webhook.enabled`                    | Editable              | Creates/removes webhook block safely.               |
| `composio.enabled/entity_id`                                        | `composio.enabled/entity_id`                                                                 | matching fields                               | Editable              | `entity_id` non-empty validation.                   |
| `composio.api_key`                                                  | `composio.has_api_key`                                                                       | `composio.api_key` (`SecretMode`)             | Editable, secret-safe | Redacted in view.                                   |
| `web_search.enabled/provider/max_results/timeout_secs`              | matching fields                                                                              | matching fields                               | Editable              | Enum/range validation.                              |
| `web_search.brave_api_key`                                          | `web_search.has_brave_api_key`                                                               | `web_search.brave_api_key` (`SecretMode`)     | Editable, secret-safe | Redacted in view.                                   |
| `browser.computer_use.api_key`                                      | `browser.has_computer_use_api_key`                                                           | `browser.computer_use_api_key` (`SecretMode`) | Editable, secret-safe | Redacted in view.                                   |
| Unrelated sections (`agent`, `mission`, `mcp`, `peripherals`, etc.) | —                                                                                            | —                                             | Non-editable          | Explicitly out of admin surface for this change.    |

**Frontend Types (TypeScript)**

```typescript
type SecretMode = "unchanged" | "replace" | "clear";

interface SecretUpdate {
  mode: SecretMode;
  value?: string;
}

interface AdminConfigUpdateRequest {
  default_provider?: string;
  default_model?: string;
  default_temperature?: number;
  memory_backend?: string;
  // Nested configs mirroring config.toml
  observability?: AdminObservabilityUpdate;
  runtime?: AdminRuntimeUpdate;
  autonomy?: AdminAutonomyUpdate;
  scheduler?: AdminSchedulerUpdate;
  gateway?: AdminGatewayUpdate;
  // Credentials use SecretUpdate to avoid accidental exposure/overwrite
  api_keys?: Record<string, SecretUpdate>;
}
```

**Backend Types (Rust)**

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretMode {
    Unchanged,
    Replace,
    Clear,
}

#[derive(Debug, Deserialize)]
pub struct SecretUpdate {
    pub mode: SecretMode,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminConfigUpdateRequest {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: Option<f64>,
    pub memory_backend: Option<String>,
    pub observability: Option<AdminObservabilityUpdate>,
    pub runtime: Option<AdminRuntimeUpdate>,
    pub autonomy: Option<AdminAutonomyUpdate>,
    pub scheduler: Option<AdminSchedulerUpdate>,
    pub gateway: Option<AdminGatewayUpdate>,
    // Dynamic map of secret updates
    pub api_keys: Option<std::collections::HashMap<String, SecretUpdate>>,
}
```

## Testing Strategy

| Layer       | What to Test                                | Approach                                                                                                                                                                             |
|-------------|---------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Unit        | Frontend config composable (`useConfig.ts`) | Verify state updates, fetch actions, and payload generation for PUT requests using Vitest.                                                                                           |
| Unit        | Backend payload deserialization             | Assert that `AdminConfigUpdateRequest` correctly parses JSON with missing optional fields and `SecretMode` enum values in Rust.                                                      |
| Integration | `GET/PUT /web/admin/config` endpoints       | Spin up a test Axum server with a mock `config.toml`. Ensure GET strips secrets. Ensure PUT safely replaces or clears secrets according to `SecretMode` without corrupting the file. |
| E2E         | Dashboard Form Submission                   | Use Playwright to load the dashboard, modify a setting (e.g., toggle an autonomy level), submit the form, and verify the success toast and subsequent GET response.                  |

## Migration / Rollout

No data migration required. The changes affect the serialization/deserialization boundaries of the
admin UI and the API gateway, resting on the existing `config.toml` structure.

## Open Questions

- [ ] Will the agent runtime require a soft restart/reload signal when the configuration is saved
  via `PUT /web/admin/config`, or do all internal subsystems already hot-reload effectively?
- [ ] Do we need a dedicated validation endpoint (`POST /web/admin/config/validate`) before saving
  to provide immediate form feedback, or is the error response from the PUT request sufficient?
