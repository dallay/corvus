# Delta for Gateway

## ADDED Requirements

### Requirement: Rook Acceptance and Regression Matrix Artifact

The `gateway` domain SHALL define a maintained Rook acceptance/regression matrix artifact for the
already-shipped operator slices covered by #592 through #599.

The matrix MUST be a consolidation artifact only. It MUST describe verification traceability for
existing shipped behavior and MUST NOT introduce new runtime features, APIs, routes, transport
semantics, or operator workflows.

The matrix MUST cover these lanes:

- dashboard
- TUI
- security
- audit

For surface-specific behavior, the matrix MUST treat `openspec/specs/dashboard/spec.md` as the
behavioral source of truth for the dashboard surface and `openspec/specs/rook-tui/spec.md` as the
behavioral source of truth for the terminal surface. The `gateway` domain SHALL own only the shared
acceptance/regression framing, command traceability, and cross-slice verification posture.

The matrix MUST trace the shipped slices as follows:

- dashboard lane: #592, #593, #594
- TUI lane: #595, #596, #597
- security lane: #598
- audit lane: #599

#### Scenario: reviewer can locate one bounded matrix for shipped Rook slices

- GIVEN the Rook operator slices #592 through #599 have already shipped with archived verification
  evidence
- WHEN a reviewer opens the maintained acceptance/regression matrix
- THEN the matrix MUST present dashboard, TUI, security, and audit lanes in one bounded artifact
- AND the matrix MUST identify which shipped slices belong to each lane
- AND the matrix MUST frame itself as a consolidation of existing evidence rather than a new
  behavioral contract for dashboard or TUI surfaces

#### Scenario: matrix preserves source-of-truth boundaries across specs

- GIVEN the dashboard and TUI surfaces already have their own behavioral specifications
- WHEN the matrix references acceptance coverage for those surfaces
- THEN the matrix MUST reference `dashboard` and `rook-tui` as the authoritative behavioral specs
- AND the matrix MUST NOT restate those surfaces as gateway-owned behavior

### Requirement: Matrix Lanes Map to Canonical Commands and Archived Evidence

Each matrix lane MUST map shipped acceptance coverage to existing commands and archived verification
evidence already present in the repository.

For every shipped slice included in the matrix, the artifact MUST identify:

- the lane it belongs to
- the archived `verify-report.md` evidence source for that slice
- the canonical repository command or existing package/crate command used to support regression
  confidence when such a command already exists
- any slice-specific focused command that remains relevant historical evidence

The matrix SHOULD prefer canonical repository entrypoints when available, including existing
`Makefile` targets and existing package/crate commands, and MAY retain focused historical commands
when they are needed to preserve traceability to archived slice evidence.

The dashboard lane MUST trace to existing dashboard commands and archived evidence for #592 through
#594, including the established dashboard build, check, unit/integration test, and end-to-end test
entrypoints already used by those slices.

The TUI lane MUST trace to existing `clients/rook` cargo verification commands and archived evidence
for #595 through #597, including the established `cargo test`, focused `tui::` test coverage,
`cargo clippy`, and `cargo fmt --check` entrypoints where those were already used by the archived
slices.

The security lane MUST trace to the targeted regression evidence from #598 for loopback-first bind
defaults, explicit bind override behavior, inbound versus outbound auth separation, admin secret
redaction, and structured-log secret safety. Each security lane entry MUST include a required
"Risk & Rollback Traceability" field that references the specific regression evidence IDs (e.g., #598),
links to threat/risk notes, and summarizes the rollback/mitigation steps and tests for boundary/failure
modes so reviewers can verify operational fallback posture.

The audit lane MUST trace to the targeted regression evidence from #599 for audit migration,
storage/service wiring, handler emission, bounded audit reads, and the preserved honesty of usage
and health coverage. Each audit lane entry MUST include a required "Risk & Rollback Traceability"
field that references the specific regression evidence IDs (e.g., #599), links to threat/risk notes,
and summarizes the rollback/mitigation steps and tests for boundary/failure modes so reviewers can
verify operational fallback posture.

#### Scenario: dashboard lane maps shipped slices to existing commands and evidence

- GIVEN archived verification reports exist for #592, #593, and #594
- WHEN the matrix presents the dashboard lane
- THEN the lane MUST identify those three slices as dashboard coverage
- AND the lane MUST reference their archived verify evidence
- AND the lane MUST map that coverage to existing dashboard verification entrypoints already used in
  the repository rather than inventing a new dashboard-only harness

#### Scenario: TUI and security lanes preserve canonical and focused evidence

- GIVEN archived verification reports exist for #595 through #598
- WHEN the matrix presents the TUI and security lanes
- THEN the TUI lane MUST map to existing `clients/rook` verification commands already used by the
  archived slices
- AND the security lane MUST preserve focused regression evidence for bind posture, auth separation,
  and secret safety
- AND the matrix MUST distinguish canonical repo entrypoints from slice-specific focused evidence
  when both are listed

### Requirement: Matrix Must Preserve Honest Coverage Boundaries

The acceptance/regression matrix MUST preserve honest boundaries around manual verification,
placeholder behavior, deferred workflow areas, and runtime-only semantics.

If a shipped slice recorded partial manual verification or an incomplete interactive check, the
matrix MUST label that coverage explicitly and MUST NOT flatten it into a fully automated or fully
verified claim.

If a behavior remains deferred in the governing source-of-truth spec, the matrix MUST identify it as
deferred or out of scope rather than implying acceptance coverage.

The matrix MUST preserve the existing `GET /api/usage` placeholder posture and MUST NOT claim real
usage, quota, billing, or cost-accounting coverage unless a separate specification changes that
contract.

The matrix MUST preserve the existing runtime-only health posture and MUST NOT claim durable health
history, persisted health snapshots, or historical health analytics coverage.

The matrix MUST preserve the audit slice's bounded posture by distinguishing persisted audit evidence
from runtime-only health state.

#### Scenario: partial manual verification remains explicitly partial

- GIVEN archived slice #596 records meaningful regression evidence but also preserves an incomplete
  manual interactive verification caveat
- WHEN the matrix summarizes route-inspection coverage in the TUI lane
- THEN the matrix MUST mark that caveat explicitly
- AND the matrix MUST NOT describe #596 as fully verified beyond the archived evidence actually
  recorded

#### Scenario: placeholder and runtime-only areas remain honest in the matrix

- GIVEN the current gateway contract keeps usage as a placeholder response and health as runtime-only
  state
- WHEN the matrix presents audit and observability coverage for #599
- THEN the matrix MUST describe usage coverage as placeholder-only
- AND the matrix MUST describe health coverage as runtime-only rather than durable historical state
- AND the matrix MUST NOT imply that persisted audit coverage upgrades those separate behaviors into
  real usage accounting or persisted health history

#### Scenario: deferred workflow areas are not overstated as covered

- GIVEN dashboard and TUI source-of-truth specs still defer some workflow areas beyond the shipped
  slices
- WHEN the matrix enumerates acceptance or regression coverage
- THEN the matrix MUST identify those areas as deferred, manual, or out of scope where applicable
- AND the matrix MUST NOT imply coverage for setup, mutation, advanced troubleshooting, logs,
  backups, or other unverified workflow areas beyond what the authoritative specs and archived
  evidence support
