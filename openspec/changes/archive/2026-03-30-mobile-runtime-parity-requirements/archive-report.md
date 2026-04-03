# Archive Report

**Change**: 2026-03-29-mobile-runtime-parity-requirements
**Archived**: 2026-03-30
**Status**: Complete with Warnings

---

## Summary

This change corrected the mobile runtime parity milestone from a host-first model to a client-first
model. Desktop, Android, and iOS composeApp surfaces are now required to start in
onboarding/readiness/configuration UX instead of probing or launching a local runtime by default.

---

## Verification Results

| Metric         | Value                  |
|----------------|------------------------|
| Tasks total    | 9                      |
| Tasks complete | 9                      |
| Build          | ✅ Passed               |
| Tests          | ✅ 75 passed / 0 failed |

**Known Limitation**: Real Android/iOS smoke validation remains incomplete due to missing mobile
transport infrastructure (documented scope limitation).

---

## Specs Synced

| Domain          | Action  | Details                                       |
|-----------------|---------|-----------------------------------------------|
| onboarding      | Updated | 3 requirements modified, 3 requirements added |
| client-surfaces | Updated | 2 requirements modified, 3 requirements added |

### Onboarding Spec Changes

- **Modified**: Surface-Specific Trust Establishment - now defines client-first connection paths for
  desktop, Android, iOS
- **Modified**: Surface-Specific Completion Criteria - now defines readiness-based completion
  instead of session-backed completion
- **Modified**: Recovery And Retry Taxonomy - added endpoint/URL and trusted companion recovery
  states
- **Added**: Client Startup Entry Point - requires onboarding-first startup
- **Added**: Minimal Client Configuration Surface - defines required config controls per platform
- **Added**: Corrected Milestone Exclusions - defines what's NOT required for this milestone

### Client-Surfaces Spec Changes

- **Modified**: Transport Invariant - now treats desktop, Android, iOS as client-first surfaces
- **Modified**: Capability Tier Enforcement - defines reduced capability set for this milestone
- **Added**: Client-First Startup Routing - requires onboarding-first startup behavior
- **Added**: Platform-Specific Connection Path Disclosure - requires disclosure of supported paths
  only
- **Added**: Milestone Scope Exclusions - defines what's NOT required for this milestone

---

## Archive Contents

- ✅ proposal.md
- ✅ specs/onboarding/spec.md
- ✅ specs/client-surfaces/spec.md
- ✅ design.md
- ✅ tasks.md
- ✅ verify-report.md
- ✅ smoke-validation-report.md
- ✅ exploration.md
- ✅ state.yaml
- ✅ android-launch.png
- ✅ ios-launch.png

---

## Source of Truth Updated

The following specs now reflect the client-first model:

- `openspec/specs/onboarding/spec.md`
- `openspec/specs/client-surfaces/spec.md`

---

## Risks

| Risk                                                              | Status                                          |
|-------------------------------------------------------------------|-------------------------------------------------|
| Real Android/iOS smoke validation incomplete                      | Known limitation - documented in scope          |
| composeApp still duplicates runtime contracts from agent-core-kmp | Warning - should be addressed in follow-on work |

---

## Next Steps

The client-first model is now captured in the main specs. Follow-on work should:

1. Complete Android/iOS transport infrastructure
2. Collapse duplicated composeApp runtime contracts onto shared agent-core-kmp types
3. Implement the onboarding-first startup routing in composeApp

---

**SDD Cycle Complete** - Ready for next change.
