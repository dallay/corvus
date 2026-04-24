# Rook Acceptance and Regression Matrix

## Purpose

This artifact consolidates the acceptance and regression evidence for shipped Rook slices `#592`
 through `#599`.

It is a traceability document, not a behavioral source-of-truth. Behavioral contracts remain owned by:

- `openspec/specs/dashboard/spec.md` for dashboard behavior
- `openspec/specs/rook-tui/spec.md` for TUI behavior
- `openspec/specs/gateway/spec.md` for gateway/admin/security/audit behavior

## Non-Goals

- Define new runtime behavior or APIs
- Replace per-domain specs as the source-of-truth
- Claim broader coverage than archived evidence proves
- Invent real usage analytics or durable health history where the product still exposes placeholder or
  runtime-only semantics

## Canonical Command Selection Rules

1. Prefer stable repo entrypoints or package scripts that still exist today.
2. Use focused historical commands only when they provide slice-specific evidence not preserved by a
   broader canonical command.
3. Carry forward archived caveats verbatim when evidence was partial or manual.
4. Mark status honestly as one of:
   - `Auto`
   - `Manual`
   - `Partial`
   - `Deferred`

## Status Legend

| Status | Meaning |
|---|---|
| Auto | Covered by repeatable commands that passed in archived verification or current repo checks |
| Manual | Requires a human/interactive verification step |
| Partial | Some automated evidence exists, but coverage or a manual follow-up caveat remains |
| Deferred | The workflow area is intentionally not implemented or remains placeholder-only |

## Dashboard Lane

| Slice | Covered surface | Canonical commands | Focused evidence | Source of truth | Archived verification | Status | Caveats |
|---|---|---|---|---|---|---|---|
| #592 | overview, providers, accounts shell | `pnpm --dir clients/web --filter @corvus/rook-dashboard run build`; `pnpm --dir clients/web --filter @corvus/rook-dashboard test`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run test:e2e` | `cargo test --manifest-path "clients/rook/Cargo.toml" admin_router_update_` | `dashboard/spec.md` | `archive/2026-04-22-rook-592-dashboard-overview-providers-accounts/verify-report.md` | Auto | Dashboard slice evidence is fully automated in archived report. |
| #593 | pools, routes, read-only health, embedded assets | `pnpm --dir clients/web --filter @corvus/rook-dashboard run check`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run test`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run test:e2e` | `pnpm --dir clients/web --filter @corvus/rook-dashboard test -- src/features/pools/usePools.spec.ts src/features/pools/PoolsPage.spec.ts src/features/routes/useRoutes.spec.ts src/features/routes/RoutesPage.spec.ts` | `dashboard/spec.md` | `archive/2026-04-22-rook-593-dashboard-pools-routes-health-ops/verify-report.md` | Auto | Embedded asset packaging validation is part of this slice and should stay linked to archived evidence. |
| #594 | usage placeholder, settings | `pnpm --dir clients/web --filter @corvus/rook-dashboard run build`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run test`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run test:e2e`; `pnpm --dir clients/web --filter @corvus/rook-dashboard run check` | none | `dashboard/spec.md` | `archive/2026-04-22-rook-594-dashboard-usage-settings/verify-report.md` | Auto | Usage remains placeholder-only by contract; logs/backups remain deferred outside this slice. |

## TUI Lane

| Slice | Covered surface | Canonical commands | Focused evidence | Source of truth | Archived verification | Status | Caveats |
|---|---|---|---|---|---|---|---|
| #595 | status, providers, pools, health shell | `cargo test --manifest-path "clients/rook/Cargo.toml"`; `cargo test --manifest-path "clients/rook/Cargo.toml" tui::`; `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings`; `cargo fmt --manifest-path "clients/rook/Cargo.toml" --check` | `cargo test --manifest-path "clients/rook/Cargo.toml" tui_command_launches_real_runner_with_effective_db_path`; `cargo test --manifest-path "clients/rook/Cargo.toml" enable_tui_runs_embedded_tui_with_shared_shutdown` | `rook-tui/spec.md` | `archive/2026-04-23-rook-595-tui-status-providers-pools-health/verify-report.md` | Auto | Archived report warned about concurrent unrelated working-tree noise, but the TUI slice evidence itself passed. |
| #596 | routes view and focused route inspection | `cargo test --manifest-path "clients/rook/Cargo.toml" tui::`; `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings` | specific `tui::app`, `tui::query`, `tui::view_models`, `tui::render` tests recorded in archived report | `rook-tui/spec.md` | `archive/2026-04-23-rook-596-tui-route-inspection-recent-logs/verify-report.md` | Partial | Archived verdict was `PASS WITH WARNINGS` because manual interactive verification of the TUI shell remained incomplete. Recent logs also remain deferred. |
| #597 | dashboard bridge for setup/mutations | `cargo test --manifest-path "clients/rook/Cargo.toml" tui::`; `cargo test --manifest-path "clients/rook/Cargo.toml" tui_command_launches_real_runner_with_effective_db_path`; `cargo test --manifest-path "clients/rook/Cargo.toml" enable_tui_runs_embedded_tui_with_shared_shutdown`; `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings` | none | `rook-tui/spec.md` | `archive/2026-04-23-rook-597-tui-setup-troubleshooting/verify-report.md` | Auto | Setup/mutations are intentionally delegated to the dashboard rather than implemented in the terminal. |

## Security Lane

| Slice | Covered surface | Canonical commands | Focused evidence | Source of truth | Archived verification | Status | Caveats |
|---|---|---|---|---|---|---|---|
| #598 | loopback defaults, auth separation, secret protection | `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings` | `cargo test --manifest-path "clients/rook/Cargo.toml" serve_cli_defaults_to_loopback_first_bind_posture`; `cargo test --manifest-path "clients/rook/Cargo.toml" server_config_defaults_to_loopback_first_bind_target`; `cargo test --manifest-path "clients/rook/Cargo.toml" explicit_non_loopback_override_remains_honored`; `cargo test --manifest-path "clients/rook/Cargo.toml" proxy_chat_completion_never_reuses_inbound_bearer_token_as_provider_auth`; `cargo test --manifest-path "clients/rook/Cargo.toml" inbound_auth_operator_state_reports_enabled_and_configured_without_exposing_token`; `cargo test --manifest-path "clients/rook/Cargo.toml" account_view_redacts_api_key_and_sets_has_api_key`; `cargo test --manifest-path "clients/rook/Cargo.toml" middleware_completion_log_fields_remain_structured_and_secret_free` | `gateway/spec.md` | `archive/2026-04-23-rook-598-security-defaults-and-secret-protection/verify-report.md` | Auto | Pairing integration is intentionally not claimed; the slice hardens the existing bearer-token boundary only. |

## Audit / Observability Lane

| Slice | Covered surface | Canonical commands | Focused evidence | Source of truth | Archived verification | Status | Caveats |
|---|---|---|---|---|---|---|---|
| #599 | persisted append-only admin audit trail | `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings` | `cargo test --manifest-path "clients/rook/Cargo.toml" open_in_memory_applies_admin_audit_events_migration`; `cargo test --manifest-path "clients/rook/Cargo.toml" open_in_memory_records_admin_audit_events_migration_version`; `cargo test --manifest-path "clients/rook/Cargo.toml" append_and_list_admin_audit_events_newest_first`; `cargo test --manifest-path "clients/rook/Cargo.toml" sqlite_audit_service_appends_and_lists_recent_events`; `cargo test --manifest-path "clients/rook/Cargo.toml" registry_audit_append_and_list_round_trip`; `cargo test --manifest-path "clients/rook/Cargo.toml" handle_create_account_appends_audit_event_on_success` | `gateway/spec.md` | `archive/2026-04-23-rook-599-observability-usage-health-audit/verify-report.md` | Auto | Request attribution remains intentionally minimal; `usage` stays placeholder-only and health stays runtime-only. |

## Honest Deferred / Placeholder Areas

| Area | Current state | Reason |
|---|---|---|
| Usage analytics/accounting | Deferred / placeholder-only | `GET /api/usage` still returns `available: false`; no usage ledger exists. |
| Durable health history | Deferred | Health remains in-memory current runtime state; no persisted health snapshots/history exist. |
| TUI recent logs workflow | Deferred | No verified backend/admin log-read contract exists. |
| TUI route-inspection manual shell confirmation | Partial | #596 archived verification kept a manual interactive gap. |

## Thin Repository-Wide Confidence Check

The repository-wide confidence command remains:

- `make build`

This is a useful broad regression signal, but it does **not** replace the slice-specific commands and
archived evidence above.
