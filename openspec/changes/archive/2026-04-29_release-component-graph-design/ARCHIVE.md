# Archive Summary: Component-Aware Release Graph for Versioned Artifacts

**Change ID**: release-component-graph-design  
**Archived**: 2026-04-29  
**Status**: COMPLETED - PASS  
**Type**: Documentation-first design phase

## Overview

This change formalized the release-component graph model for externally versioned artifacts in the Corvus monorepo. It established the canonical contract for component-scoped release planning, validation, and publication.

## Completion Summary

- **Verification Result**: PASS
- **Requirements Documented**: 8/8 (100%)
- **Scenarios Verified**: 30/30 (100%)
- **Tasks Completed**: 10/10 (100%)
- **Critical Issues**: 0
- **Warnings**: 0
- **Observations**: 3 (terminology consistency, RFC 2119 compliance, rollback posture)

## What Was Delivered

### Phase 1: Main Spec Updates (Documentation-First)

Updated the canonical release-management specs with graph-backed component resolution:

1. **openspec/specs/release-management/spec.md**
   - Added graph-backed component scope resolution as the canonical authority
   - Defined path classification rules (release-owned, shared-release-infra, non-release, ignored)
   - Specified transitive dependency expansion semantics
   - Established fail-closed behavior for unmapped paths

2. **openspec/specs/release-management/component-versioning.md**
   - Added graph resolution algorithm section
   - Defined direct ownership, shared infrastructure fan-out, and transitive closure rules
   - Specified operator-facing inclusion reasons

3. **openspec/specs/release-management/component-inventory.md**
   - Added graph authority statement
   - Clarified that the graph is the executable source of truth

4. **openspec/specs/release-management/impact-map.md**
   - Updated with graph-driven path classification
   - Defined four path categories with resolution rules

5. **openspec/specs/release-management/pipeline-gating.md**
   - Added MUST-level gating requirements for graph-backed validation
   - Specified version surface alignment checks

6. **openspec/specs/release-management/migration-plan.md**
   - Added phased rollout clarity for graph adoption
   - Defined transition from workflow-local maps to canonical graph

### Phase 2: Delta Spec (This Change)

Created comprehensive specification in `openspec/changes/release-component-graph-design/spec.md`:

- 8 requirements with full RFC 2119 compliance
- 30 scenarios covering all resolution paths
- Complete component inventory (rook, cerebro, corvus-runtime, gradle-kmp)
- Publish policy enforcement rules
- Version surface alignment requirements
- Stable/beta parity guarantees
- Operator-facing traceability requirements

## Key Design Decisions

1. **Graph as Canonical Authority**: The release-component graph is the single source of truth for component resolution, replacing workflow-local maps.

2. **Four Path Categories**: Every changed path must be classified as release-owned, shared-release-infra, non-release, or ignored.

3. **Transitive Dependency Expansion**: Components can declare `depends_on_release_of` edges to join release scope when upstream components change.

4. **Publish Policy Distinction**: Components are either `publishable` (publish artifacts) or `validate-only` (participate in validation without publishing).

5. **Fail-Closed for Unmapped Paths**: Once fully adopted, the resolver must fail when a release-relevant path is not classified by the graph.

6. **Stable/Beta Parity**: Both channels use identical graph semantics; only version suffixes and manifest files differ.

## Implementation Readiness

This was a documentation-first phase. The next implementation slice should:

1. Identify the executable graph file location and format
2. Define the first implementation slice for graph-backed resolver
3. Define publish-validation follow-up work
4. Confirm multi-component handoff contract

## Artifacts

- `proposal.md` - Change intent and scope
- `spec.md` - Complete requirements specification
- `design.md` - Technical approach and architecture
- `tasks.md` - Implementation task breakdown
- `apply-summary.md` - Phase 1 completion summary
- `verify-report.md` - Verification results
- `verify-result.json` - Structured verification data
- `state.yaml` - Phase tracking state

## References

- Main spec: `openspec/specs/release-management/spec.md`
- Component versioning: `openspec/specs/release-management/component-versioning.md`
- Component inventory: `openspec/specs/release-management/component-inventory.md`
- Impact map: `openspec/specs/release-management/impact-map.md`
- Pipeline gating: `openspec/specs/release-management/pipeline-gating.md`
- Migration plan: `openspec/specs/release-management/migration-plan.md`

## Lessons Learned

1. **Documentation-first approach worked well**: Updating main specs first provided clear context for the delta spec.
2. **RFC 2119 compliance is essential**: Consistent use of MUST/SHOULD/MAY makes requirements unambiguous.
3. **Scenario coverage drives completeness**: 30 scenarios ensured all resolution paths were specified.
4. **Terminology consistency matters**: Using consistent terms (affected, directly affected, transitively affected) across all artifacts improved clarity.

## Next Steps

The implementation phase should:
1. Choose graph file format (JSON, YAML, or TOML)
2. Implement graph-backed resolver
3. Migrate stable and beta workflows to use the resolver
4. Add validation for unmapped paths
5. Update publish workflows to consume graph-derived scope
