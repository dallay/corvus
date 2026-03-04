# Proposal: Web Agent Config

## Intent

Enable comprehensive configuration of the agent runtime via the web dashboard by expanding backend payload support and modularizing the frontend Vue prototype. This will allow users to securely and easily configure all features available in `config.toml` from a graphical interface.

## Scope

### In Scope
- Refactoring the massive single-file Vue prototype (`App.vue`) in `clients/web/apps/dashboard/` into modular components.
- Expanding payload support in `AdminConfigUpdateRequest` and `AdminConfigView` within `clients/agent-runtime/src/gateway/` to match all features from `config.toml`.
- Ensuring tight schema validation on the backend (`clients/agent-runtime/src/config/`) for updates originating from the web client.
- Securely handling credentials using the existing `save()` mechanism (which encrypts credentials).

### Out of Scope
- Adding new configuration features to `config.toml` that do not currently exist.
- Overhauling the core pairing logic (`POST /pair`) beyond expanding its payload support.
- Changes to non-agent components of the system.

## Approach

1. **Frontend**: Break down `App.vue` into logical, feature-specific Vue components (e.g., `NetworkSettings`, `SecuritySettings`, `LLMConfig`). Create a unified state management layer to handle the complex nested object structure of the configuration.
2. **Backend Gateway**: Update the Axum structs (`AdminConfigUpdateRequest`, `AdminConfigView`) to fully represent the `config.toml` schema. Update `GET/PUT /web/admin/config` and `GET /web/admin/options` handlers to map these new fields.
3. **Backend Config Management**: Ensure the trait-driven loader and validation logic comprehensively validate the expanded payloads before invoking `save()`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/web/apps/dashboard/src/App.vue` | Modified | Refactored into smaller components |
| `clients/web/apps/dashboard/src/components/` | New | New modular config components |
| `clients/agent-runtime/src/gateway/` | Modified | Updated Axum endpoints and structs |
| `clients/agent-runtime/src/config/` | Modified | Enhanced validation logic for `config.toml` |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Frontend state management complexity for nested configs | Medium | Implement robust state management (e.g., Pinia) or careful prop-drilling with clear types. |
| Validation mismatch between frontend and backend | Low | Use shared schema/types where possible, and enforce strict backend validation. |
| Credential exposure during transit or save | Low | Rely on existing encrypted `save()` mechanism and ensure HTTPS/secure transport for endpoints. |

## Rollback Plan

- Revert frontend changes to the original `App.vue` prototype state.
- Revert backend struct changes (`AdminConfigUpdateRequest`, `AdminConfigView`) to their previous, limited schema.
- Revert any specific validation logic added for the expanded fields.

## Dependencies

- None.

## Success Criteria

- [ ] Users can view all `config.toml` settings in the web dashboard.
- [ ] Users can modify and successfully save all `config.toml` settings via the web dashboard.
- [ ] The backend correctly validates all incoming configuration updates before saving.
- [ ] Credentials are saved securely using the encrypted `save()` mechanism.
- [ ] The frontend `App.vue` is modularized into maintainable components.