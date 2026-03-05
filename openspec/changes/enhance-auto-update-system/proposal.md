# Proposal: Enhance Auto-Update System

## Problem

The current update flow is fragmented and only partially safe: visibility is inconsistent across CLI, daemon, and in-conversation channels; auto-install support is limited to a few package managers; update state persistence is not atomic across processes; and runtime auto-install lacks explicit artifact verification and auditability. This creates user confusion, security risk, and operational instability when mixed runtime versions are active.

## Goals

- Provide proactive update visibility across CLI startup, in-conversation prompts, and client UI/admin surfaces.
- Add explicit, safe-by-default auto-update policy with opt-in auto-install and environment overrides.
- Detect installation method and execute method-specific update routines (npm/pnpm/yarn/bun, binary/script, homebrew, cargo), with deterministic fallback.
- Make update operations process-safe and atomic across concurrent CLI/daemon processes.
- Add first-class `corvus update` command group: `status`, `check`, `install`, `auto-enable`, `auto-disable`, `history`.
- Enforce artifact integrity/security checks (checksum first, signature-ready contract) and produce auditable update events.

## Non-Goals

- Redesigning release publishing pipelines beyond consuming existing checksum/signature metadata.
- Shipping full UX redesigns for unrelated clients; this change focuses on shared indicators and admin/dashboard integration points.
- Implementing zero-downtime binary hot-swap or full rollback orchestration for all runtime modes.

## High-Level Approach

1. Build an `UpdateManager` flow in `clients/agent-runtime/src/update/mod.rs` that unifies check, policy evaluation, install planning, verification, and event recording.
2. Introduce install-method detection and persistence (detected + user override), then route installs through method executors with explicit unsupported-method handling.
3. Add a dedicated `update` command tree in `clients/agent-runtime/src/main.rs` for interactive and scriptable operations.
4. Replace non-atomic update state writes (`workspace/state/version_check.json`) with temp-file + fsync + atomic rename semantics and inter-process file locking.
5. Extend config schema in `clients/agent-runtime/src/config/schema.rs` with auto-update policy knobs and env overrides (keeping safe defaults).
6. Normalize notification payloads for CLI banners, channel messages (`clients/agent-runtime/src/channels/mod.rs`), daemon push notifications (`clients/agent-runtime/src/daemon/mod.rs`), and gateway/admin API exposure (`clients/agent-runtime/src/gateway/admin.rs`, `clients/web/apps/dashboard/src/types/admin-config.ts`).
7. Add security verification gates before install and append structured audit log events to update history.

## Phased Scope

### Phase 1: Safety and Command Foundation
- Add `corvus update status|check|install` command surface.
- Implement atomic state persistence, inter-process locking, and single-install transaction guards.
- Add install-method detection for currently supported methods and robust manual fallback.
- Standardize update status model used by CLI and daemon.

### Phase 2: Auto-Update Policy and Visibility Expansion
- Add `auto-enable`, `auto-disable`, and policy/env override support.
- Unify proactive notifications across CLI/in-conversation/daemon channels.
- Expose update status + policy fields through admin gateway and dashboard types.

### Phase 3: Verification Hardening and Auditability
- Enforce checksum verification for downloaded artifacts and define signature-verification extension points.
- Add `corvus update history` backed by structured audit events.
- Add daemon-safe restart/staging behavior to avoid mixed-version runtime state.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/update/mod.rs` | Modified | Core update manager, method detection/execution, verification gates, history events |
| `clients/agent-runtime/src/main.rs` | Modified | New `corvus update` subcommands and CLI wiring |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | In-conversation visibility and nonce-confirmed install handoff |
| `clients/agent-runtime/src/daemon/mod.rs` | Modified | Polling, notification, and safe install coordination |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Auto-update policy schema/defaults/env overrides |
| `clients/agent-runtime/src/service/mod.rs` | Modified | Controlled restart/staging integration after install |
| `clients/agent-runtime/src/gateway/admin.rs` | Modified | Update status/policy fields for client UI visibility |
| `clients/web/apps/dashboard/src/types/admin-config.ts` | Modified | Typed update indicator and config fields |
| `workspace/state/version_check.json` (+ lock/history peers) | Modified/New | Atomic state, lock coordination, and audit history storage |

## Risks and Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Concurrent CLI + daemon update races | High | Inter-process file locks + transaction state machine + idempotent install steps |
| Partial/corrupt update state writes | Medium | Temp-file write, fsync, atomic rename, and read-after-write validation |
| Wrong install strategy selected | Medium | Detection priority matrix, persisted method override, explicit dry-run/status output |
| Integrity bypass in non-package-manager paths | High | Mandatory checksum verification; signature verification hook and fail-closed policy |
| Runtime disruption from mixed versions | Medium | Staged install markers and coordinated service restart gating |
| Source/repo drift for version checks | Medium | Canonical source configuration and strict endpoint validation |

## Rollback Plan

- Keep existing startup banner + manual update pathway behind compatibility path while new command group is introduced.
- Guard new auto-update/install-method logic behind feature flags or config toggles so behavior can revert to notify-only mode.
- If regressions appear, disable auto-install policy defaults, retain check-only flow, and revert command handlers to existing behavior without deleting stored history.
- Revert affected modules in a single patch set (`update`, `main`, `daemon`, `channels`, `config`, `gateway`, dashboard types) and preserve state files for postmortem.

## Dependencies

- Existing GitHub release metadata and checksum artifacts from `.github/workflows/_publish.yml`.
- Existing installer paths (`clients/agent-runtime/npm/corvus-cli/lib/install.js`, `clients/web/apps/marketing/public/install`) for method heuristics and verification alignment.

## Acceptance Criteria

- [ ] `corvus update status|check|install|auto-enable|auto-disable|history` are available and return deterministic exit codes.
- [ ] Default policy is safe (`check+notify` enabled, auto-install disabled) and env overrides are documented and effective.
- [ ] Installation method is detected (or explicitly overridden), surfaced in `status`, and used for method-specific execution/fallback.
- [ ] Concurrent update attempts do not corrupt state or run parallel installs.
- [ ] Update state writes are atomic and recoverable after interruption.
- [ ] Runtime install path performs artifact integrity verification before activation.
- [ ] CLI, in-conversation, and admin/dashboard surfaces expose consistent update availability and policy status.
- [ ] Update attempts and outcomes are persisted in audit history and viewable via `corvus update history`.
