# Proposal: Refine `rook doctor` for Production-Focused Operational Diagnostics

## Intent

`rook doctor` already exists, but today it only covers a narrow subset of the operational questions an operator needs to answer before putting Rook into service. It validates that effective configuration can load, checks embedded dashboard assets, verifies inbound auth basics, and opens the database read-only, but it does not yet provide a sufficiently production-focused readiness picture or clearly separate critical failures from advisory findings.

This change enhances the existing `rook doctor` command into a deterministic local diagnostics workflow for operators. The goal is to let an operator run one command and quickly learn whether the configured Rook instance is safe to start or debug further, with actionable failures, bounded checks, and non-zero exit behavior for conditions that would prevent correct local service startup.

## Scope

### In Scope

1. **Enhance the existing `rook doctor` command rather than introducing a new command**
   - Treat this change as an operational refinement of the current doctor implementation.
   - Preserve the current CLI entrypoint while expanding the checks and result semantics.

2. **Database open and migration readiness diagnostics**
   - Verify that the effective database path is usable.
   - Verify that startup-time database initialization and migrations can complete successfully enough for service startup, not just that a read-only open succeeds.
   - Report actionable failure messages when the database path, permissions, locking posture, or migration state prevent readiness.

3. **Effective server configuration validation**
   - Validate the effective host, port, database path, and other server-affecting settings that `serve` would actually use.
   - Make operator-visible reporting explicit about the effective bind target and retain alignment with the gateway domain’s existing loopback-first bind posture.
   - Distinguish configuration errors from advisory findings where startup is still possible.

4. **Inbound auth validation when enabled**
   - Verify that protected-route inbound bearer auth configuration is internally consistent when enabled.
   - Confirm that required credentials/config are present without leaking secret values.
   - Keep the check local and deterministic by default.

5. **Dashboard asset availability diagnostics**
   - Verify that required embedded dashboard assets are available for a production binary.
   - Surface asset failures as actionable diagnostics when the admin/dashboard surface would be broken.

6. **Actionable status model and non-zero exit behavior**
   - Continue producing machine-readable or structured pass/warn/fail output suitable for operators and automation.
   - Return a non-zero exit code when one or more required readiness checks fail.
   - Keep warnings non-fatal unless they correspond to a startup-blocking condition.

7. **Optional upstream probe evaluation, only if architecture already supports it cleanly**
   - Consider optional, explicitly bounded probes of configured upstream accounts only if they can be implemented as a clearly opt-in diagnostic mode that does not redefine local readiness.
   - If retained, remote probes MUST be advisory and MUST NOT be part of the default deterministic readiness result.

### Out of Scope

- Creating `rook doctor` from scratch or changing its fundamental CLI identity.
- Expanding gateway readiness to depend on upstream provider reachability by default.
- Broad new remote health orchestration, account verification workflows, or provider-specific certification logic.
- Changes to the dashboard UX itself beyond validating that required embedded assets are present.
- New auth models, pairing flows, or exposure-hardening changes outside current inbound auth validation.
- Full observability redesign, long-running monitoring, or historical diagnostics storage.

## Approach

This proposal keeps the `gateway` domain as the main specification source of truth because the existing gateway spec already defines Rook’s HTTP operational posture, including safe default binding and related admin/API behavior. The work should extend the current `rook doctor` implementation so it evaluates the same effective configuration and startup prerequisites used by `serve`, rather than inventing a parallel diagnostic model.

The enhanced command should remain local-first and deterministic by default:

1. Load the same effective configuration path used by runtime startup.
2. Validate configuration semantics relevant to server startup and bind posture.
3. Attempt the same class of database initialization/migration readiness check that startup depends on.
4. Validate enabled inbound auth configuration without exposing bearer secrets.
5. Verify embedded dashboard asset availability.
6. Produce a summarized pass/warn/fail report with actionable messages and a non-zero exit on required failures.

If the repository’s current architecture justifies probing configured upstream accounts, that behavior should be introduced only as an explicitly opt-in probe mode with bounded timeouts and clearly advisory results. Default `rook doctor` behavior should stay deterministic and avoid remote dependencies so operators can trust it in offline, CI, or incident environments.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/main.rs` | Modified | Preserve the existing `rook doctor` CLI entrypoint while expanding its operational semantics and exit behavior. |
| `clients/rook/src/doctor.rs` | Modified | Extend doctor checks, result classification, and rendered output for production-focused diagnostics. |
| `clients/rook/src/config/mod.rs` | Modified | Reuse and potentially extend effective configuration loading/validation so doctor and serve evaluate the same startup inputs. |
| `clients/rook/src/server/mod.rs` | Modified | Provide or reuse startup-readiness logic so doctor validates the same local prerequisites the server depends on. |
| `clients/rook/src/registry/mod.rs` | Modified | Support database open/migration readiness checks aligned with real startup behavior. |
| `clients/rook/src/db/` | Modified | Ensure migration/open readiness can be exercised in a doctor-safe path with actionable errors. |
| `clients/rook/src/dashboard/mod.rs` | Modified/Verified | Continue to serve as the source for embedded dashboard asset availability checks. |
| `clients/rook/src/auth/` | Modified/Verified | Reuse inbound bearer-auth configuration/state validation for operator diagnostics. |
| `openspec/specs/gateway/spec.md` | Modified | Record the refined `rook doctor` operational expectations under the gateway domain. |
| `openspec/specs/dashboard/spec.md` | Referenced | Remains relevant only for dashboard asset expectations; the main behavior change belongs to gateway operations. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Doctor drifts from real startup behavior and produces false confidence | Medium | Reuse the same effective config, DB initialization, and startup validation paths used by `serve` wherever possible. |
| Database checks become destructive or mutate operator state unexpectedly | Medium | Keep diagnostics bounded, prefer startup-equivalent safe checks, and avoid speculative writes outside what startup already requires. |
| Optional remote probes make doctor flaky in offline or incident conditions | Medium | Keep all remote probes opt-in, bounded, and advisory; exclude them from default readiness and exit code decisions. |
| Auth diagnostics leak sensitive bearer configuration in logs or output | Low | Report presence/validity only, never raw token material, and preserve existing redaction behavior. |
| Operators misinterpret warnings as fatal or failures as advisory | Medium | Define explicit pass/warn/fail semantics and document which checks drive non-zero exit status. |

## Rollback Plan

If the refined diagnostics prove too brittle or produce misleading results, rollback should revert the enhancement in layers while preserving the existing command surface:

1. Revert new doctor checks that are not clearly aligned with startup behavior, keeping only the previously stable local checks.
2. Remove any new startup-equivalent DB migration/readiness probe if it introduces unsafe side effects or false negatives, falling back to the prior database connectivity check.
3. Remove any optional upstream probe mode entirely if it adds confusion or operational flakiness.
4. Preserve the existing `rook doctor` command name and basic reporting flow so operator scripts do not need to change during rollback.

Because this change is an enhancement to an existing command, rollback should not require API contract changes, data migration reversal, or modifications to the primary `/v1` or `/api` serving surfaces.

## Dependencies

- Existing `rook doctor` implementation and CLI wiring in `clients/rook/src/main.rs` and `clients/rook/src/doctor.rs`.
- Shared effective configuration loading and validation in `clients/rook/src/config/mod.rs`.
- Registry/database startup behavior and migrations used by Rook runtime initialization.
- Existing gateway-spec source of truth for bind posture, admin surface behavior, and local operational defaults.
- Existing dashboard asset embedding and inbound auth configuration logic.

## Success Criteria

- [ ] `rook doctor` is specified and implemented as an enhancement of the existing command, not a net-new CLI surface.
- [ ] Default doctor execution performs deterministic local checks for effective config, database startup/migration readiness, inbound auth validity when enabled, and dashboard asset availability.
- [ ] Startup-blocking failures produce actionable messages and a non-zero exit code.
- [ ] Secret-bearing auth diagnostics do not expose raw token values in output.
- [ ] The effective bind target and related server configuration are reported consistently with the gateway domain’s existing loopback-first posture.
- [ ] Any remote upstream probing, if implemented at all, is clearly opt-in, bounded, advisory-only, and excluded from default readiness decisions.
