# Tasks: Web Agent Config

## Phase 1: Contract Baseline and Safety Guards

- [x] 1.1 Add a config coverage matrix in `openspec/changes/web-agent-config/design.md` mapping
  every editable `config.toml` section from `clients/agent-runtime/src/config/schema.rs` to
  `AdminConfigView` and `AdminConfigUpdateRequest` fields in
  `clients/agent-runtime/src/gateway/admin.rs` (including explicit non-editable/hidden fields).
- [x] 1.2 Create shared dashboard admin-config types in
  `clients/web/apps/dashboard/src/types/admin-config.ts` (view, update, nested section patches, and
  `SecretMode = "unchanged" | "replace" | "clear"`) aligned to backend JSON contracts.
- [x] 1.3 Add a focused Rust contract test module (RED) under
  `clients/agent-runtime/src/gateway/admin.rs` that fails when newly defined config sections are
  missing in `admin_config_view()` serialization or `AdminConfigUpdateRequest` deserialization.
- [x] 1.4 Implement the minimum gateway contract updates (GREEN) in
  `clients/agent-runtime/src/gateway/admin.rs` to satisfy the new coverage tests without exposing
  raw secrets, then refactor repeated mapping helpers (REFACTOR).

## Phase 2: Frontend Modularization (Dashboard)

- [x] 2.1 Extract API/state orchestration from `clients/web/apps/dashboard/src/App.vue` into
  `clients/web/apps/dashboard/src/composables/useConfig.ts` with fetch/connect/save actions,
  per-section saving flags, and diff-based payload builders.
- [x] 2.2 Create modular config components under `clients/web/apps/dashboard/src/components/config/`
  for `GeneralSettings.vue`, `SecuritySettings.vue`, `ObservabilitySettings.vue`,
  `RuntimeSettings.vue`, `SchedulerSettings.vue`, `GatewaySettings.vue`, and `WebhookSettings.vue`
  using typed props/events from `src/types/admin-config.ts`.
- [x] 2.3 Refactor `clients/web/apps/dashboard/src/App.vue` into a layout/container shell that wires
  authentication/pairing controls plus modular config sections via `useConfig.ts`, preserving
  existing i18n keys and current UX copy.
- [x] 2.4 Add a secret intent UI contract in
  `clients/web/apps/dashboard/src/components/config/WebhookSettings.vue` and
  `clients/web/apps/dashboard/src/composables/useConfig.ts` so unchanged/replace/clear flows cannot
  emit ambiguous payloads (replace requires non-empty value, clear sends no value).
- [x] 2.5 Move pure payload/diff logic into
  `clients/web/apps/dashboard/src/composables/configPayload.ts` and keep components presentational
  to reduce App-level coupling and support isolated unit tests.

## Phase 3: Backend Expansion, Validation, and Persistence

- [x] 3.1 Expand `AdminConfigView` in `clients/agent-runtime/src/gateway/admin.rs` to represent the
  full intended admin-editable `config.toml` surface (defaults, runtime, autonomy, scheduler,
  gateway, channels/webhook, observability, identity/provider-related fields) while masking/omitting
  all sensitive values.
- [x] 3.2 Expand `AdminConfigUpdateRequest` and nested patch structs in
  `clients/agent-runtime/src/gateway/admin.rs` to accept the same full editable surface, keeping
  optional partial updates and strict serde behavior for unknown/invalid shapes.
- [x] 3.3 Generalize secret update handling in `clients/agent-runtime/src/gateway/admin.rs` by
  introducing reusable secret patch application (`Unchanged`, `Replace { value }`, `Clear`) for
  every secret-bearing field currently persisted through `Config::save()` in
  `clients/agent-runtime/src/config/schema.rs`.
- [x] 3.4 Add centralized patch validation helpers in `clients/agent-runtime/src/gateway/admin.rs`
  for bounds/ranges/enums/empties (ports, temperatures, rate limits, backend enums, host
  constraints) and return deterministic 400 errors with field-specific messages.
- [x] 3.5 Update restart-required detection in `clients/agent-runtime/src/gateway/admin.rs` so
  conflict reporting remains correct for all newly supported fields, including secret-change intent
  and normalized values.
- [x] 3.6 Ensure persistence flow in `clients/agent-runtime/src/gateway/admin.rs` performs: parse ->
  validate -> apply -> `validate_for_runtime()` in `clients/agent-runtime/src/config/schema.rs` ->
  `save()` -> in-memory swap, with rollback on failed save and no partial in-memory mutation.

## Phase 4: Automated Testing (Unit, Integration, E2E)

- [x] 4.1 Add frontend composable unit tests (RED/GREEN) in
  `clients/web/apps/dashboard/src/composables/useConfig.spec.ts` covering initial fetch mapping,
  section-scoped save state, diff payload generation, and secret mode payload rules.
- [x] 4.2 Split and update UI tests from `clients/web/apps/dashboard/src/App.spec.ts` into
  component-focused specs under `clients/web/apps/dashboard/src/components/config/*.spec.ts`
  verifying modular rendering (Server/Identity/Provider-focused sections) and module-local
  validation feedback.
- [x] 4.3 Add Rust unit tests in `clients/agent-runtime/src/gateway/admin.rs` for expanded
  request/view serde, secret update transitions (unchanged/replace/clear), and field-level
  validation errors for invalid ranges or malformed payloads.
- [x] 4.4 Add gateway integration tests in
  `clients/agent-runtime/tests/admin_config_api_integration.rs` for `GET /web/admin/config` and
  `PUT /web/admin/config`, asserting full-field round-trip updates, secret redaction in GET, secure
  secret mutation behavior, and persistence rollback on save failure.
- [x] 4.5 Add dashboard E2E coverage using Playwright in
  `clients/web/apps/dashboard/e2e/admin-config.spec.ts` plus
  `clients/web/apps/dashboard/playwright.config.ts` to validate end-to-end edit/save flows and
  secret mode transitions against a test gateway fixture.
- [x] 4.6 Wire test scripts in `clients/web/apps/dashboard/package.json` and
  `clients/web/package.json` (for example `test:e2e`) and run verification stack (
  `pnpm --filter @corvus/dashboard test`, E2E suite, and targeted Rust tests for admin config
  endpoints).

## Phase 5: Final Verification and Handoff

- [x] 5.1 Execute full regression for touched surfaces (`cargo test -p agent-runtime admin`,
  `pnpm --filter @corvus/dashboard test`, dashboard E2E) and then repository baseline `make test`;
  capture failures/fixes in the same files before completion.
- [x] 5.2 Update `openspec/changes/web-agent-config/design.md` and this
  `openspec/changes/web-agent-config/tasks.md` with any final field mapping deltas discovered during
  implementation, ensuring all spec scenarios are explicitly marked test-covered.
