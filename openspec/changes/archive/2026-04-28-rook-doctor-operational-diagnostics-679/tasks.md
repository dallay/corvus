# Tasks: Rook Doctor Operational Diagnostics

## Phase 1: Infrastructure

- [x] 1.1 In `clients/rook/src/server/mod.rs`, extract a shared startup-readiness seam that accepts effective `RookConfig` and returns structured local readiness data without binding a socket.
- [x] 1.2 In `clients/rook/src/registry/mod.rs`, add a startup-readiness entrypoint that reuses runtime registry open/create behavior instead of `open_readonly()`.
- [x] 1.3 In `clients/rook/src/db/mod.rs`, route readiness checks through the same open + migration path as startup and map path, permission, lock, and migration failures to operator-actionable errors.
- [x] 1.4 In `clients/rook/src/config/mod.rs`, add any small helper/view needed to report effective bind target and inbound-auth config state without exposing secret values.

## Phase 2: Implementation

- [x] 2.1 In `clients/rook/src/doctor.rs`, write failing unit tests for the richer `DoctorCheckResult`/`DoctorReport` contract, stable check ordering, warn/fail guidance, and `ensure_success` exit semantics.
- [x] 2.2 In `clients/rook/src/doctor.rs`, implement structured doctor result types and rendering with stable check names, `pass`/`warn`/`fail`, summaries, details, and guidance.
- [x] 2.3 In `clients/rook/src/doctor.rs`, switch doctor orchestration to call the shared startup-readiness seam and report required checks for config, database, assets, and inbound auth.
- [x] 2.4 In `clients/rook/src/doctor.rs` and `clients/rook/src/config/mod.rs`, implement secret-safe auth/config reporting that states enabled/configured/missing status but never echoes bearer tokens or provider secrets.
- [x] 2.5 In `clients/rook/src/main.rs`, keep the existing `rook doctor` entrypoint while wiring the enhanced report and preserving non-zero exit behavior only for required local failures.
- [ ] 2.6 If the design is carried through now, add opt-in advisory upstream probe plumbing in `clients/rook/src/main.rs` and `clients/rook/src/doctor.rs`, keeping probe results separate from required local readiness and exit code decisions.

## Phase 3: Testing

- [x] 3.1 In `clients/rook/src/doctor.rs`, add unit tests proving inbound-auth diagnostics never leak raw token values in summaries, details, guidance, or aggregated failure text.
- [x] 3.2 In `clients/rook/src/server/mod.rs`, `clients/rook/src/registry/mod.rs`, or `clients/rook/src/db/mod.rs`, add focused tests for startup-equivalent DB readiness success plus invalid path, open/create denial, lock/contention, and migration-failure mapping.
- [x] 3.3 Add integration tests for a happy-path doctor run using a temp DB path, asserting effective bind target reporting and passing required checks for config, database, assets, and inbound auth.
- [x] 3.4 Add integration/CLI tests for failure paths: invalid effective config, enabled auth with missing token, unusable DB path, and missing dashboard assets, asserting `fail` statuses and non-zero exit behavior.
- [ ] 3.5 If upstream probing is implemented, add tests proving default doctor omits probes and opt-in probe failures stay advisory-only with zero exit when local readiness passes.
