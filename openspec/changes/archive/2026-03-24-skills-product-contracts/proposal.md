# Proposal: Skills Product Contracts

## Intent

Close the 4 child workstreams of the skills trust initiative by formalizing product-level contracts.
The Corvus skills system is 85-95% implemented (commits `682fbeed`, `ef20eb39`); what remains is
documenting the product contracts that govern how skills are authored, distributed, installed, and
curated. These contracts serve as the authoritative reference for contributors, agents, and
downstream tooling.

## Scope

### In Scope

- **Local Skills UX contract** — directory layout, SKILL.md format, install/list/remove behavior
- **Third-Party Source Policy contract** — consent model, trust gating, scanner enforcement,
  sandboxing
- **Skill Install Lifecycle contract** — source resolution, lockfile semantics, update/repair flows,
  agent-driven install policy
- **Official Catalog Model contract** — repository governance, index format, review standard,
  embedded/cache lifecycle
- Optional: `--json` CLI flag for structured output (deferred to follow-up)

### Out of Scope

- Major code changes — contracts document existing behavior
- Source allowlisting implementation (deferred; `--trust` per-install is sufficient)
- Catalog CI pipeline for `dallay/corvus-skills` (infrastructure follow-up)
- Semantic versioning enforcement for catalog skills

## Approach

Write 4 product contract documents under `openspec/changes/skills-product-contracts/specs/`. Each
contract uses clear product language (not RFC 2119), covers the user experience, directory layout,
commands, trust model, and future work. Contracts reference the existing spec R1-R20 as
implementation authority.

## Affected Areas

| Area                                               | Impact    | Description                                     |
|----------------------------------------------------|-----------|-------------------------------------------------|
| `openspec/changes/skills-product-contracts/specs/` | New       | 4 product contract documents                    |
| `openspec/specs/skills-trust/spec.md`              | Reference | Existing spec unchanged; contracts reference it |
| `clients/agent-runtime/src/skills/`                | Reference | Existing code unchanged; contracts document it  |

## Risks

| Risk                                | Likelihood | Mitigation                                                 |
|-------------------------------------|------------|------------------------------------------------------------|
| Contracts drift from implementation | Low        | Contracts written directly from code review in exploration |
| Missing edge cases in contracts     | Low        | Verification task cross-checks against code                |

## Rollback Plan

Delete the `openspec/changes/skills-product-contracts/specs/` directory. These are
documentation-only changes with zero code impact.

## Dependencies

- Exploration analysis completed (see `exploration.md`)
- Existing implementation in `clients/agent-runtime/src/skills/`

## Success Criteria

- [ ] All 4 product contracts written and internally consistent
- [ ] Contracts accurately reflect implemented behavior
- [ ] Key decisions documented: agent auto-install policy, source allowlisting deferral, review
  standard
- [ ] Future work clearly scoped in each contract
