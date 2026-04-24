# Tasks: Rook Acceptance and Regression Matrix

## Phase 1: Source Inventory and Matrix Skeleton

- [x] 1.1 Read archived verify reports for `#592`-`#599` and extract per-slice commands, verdicts, and caveats into a working outline for dashboard, TUI, security, and audit lanes.
- [x] 1.2 Create `openspec/specs/gateway/rook-acceptance-regression-matrix.md` with purpose, non-goals, command-selection rules, status legend, and empty lane tables matching the design row shape.
- [x] 1.3 Add initial lane rows for `#592`-`#599` with archived verify-report links and source-of-truth spec references, leaving status/caveat cells explicit where evidence still needs normalization.

## Phase 2: Populate Canonical Commands and Honest Coverage States

- [x] 2.1 Normalize dashboard lane rows (`#592`-`#594`) to canonical existing commands from `Makefile` and `clients/web/apps/rook-dashboard/package.json`, preserving any slice-specific focused evidence only where it adds traceability.
- [x] 2.2 Normalize TUI lane rows (`#595`-`#597`) to existing `clients/rook` cargo verification commands, and mark route-inspection/manual-shell coverage for `#596` as `Partial` with the archived warning carried forward.
- [x] 2.3 Populate security (`#598`) and audit (`#599`) rows with targeted regression commands and caveats that keep loopback/auth separation, usage placeholder, and runtime-only health claims honest.
- [x] 2.4 Mark any unimplemented or out-of-scope workflow areas as `Manual`, `Deferred`, or placeholder-only instead of implying broader acceptance coverage.

## Phase 3: Spec Pointer and Optional Thin Glue

- [x] 3.1 Update `openspec/specs/gateway/spec.md` with a minimal durable pointer/reference to `openspec/specs/gateway/rook-acceptance-regression-matrix.md` without expanding gateway runtime requirements.
- [x] 3.2 Decide whether a convenience command is justified; if not, document rerun guidance in the matrix only.
- [cancelled] 3.3 If justified by clear reuse value, add one thin compositional target in `Makefile` that only sequences existing dashboard and Rust commands already cited by the matrix, with no new harness semantics.

## Phase 4: Verification and Evidence Check

- [x] 4.1 Verify every canonical command named in the matrix exists in `Makefile`, `clients/web/apps/rook-dashboard/package.json`, or archived focused cargo commands, and correct any drift.
- [x] 4.2 Cross-check each matrix row against the archived `verify-report.md` evidence and the `gateway`, `dashboard`, and `rook-tui` specs so lane ownership, status labels, and caveats stay accurate.
- [x] 4.3 Review the finished artifact for honesty and bounded scope: no new runtime/API claims, `#596` remains partial, and `#599` keeps usage placeholder/runtime-only health posture explicit.
