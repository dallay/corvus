# Proposal: `code_search` Benchmark Rollout and Documentation

**Issue**: #360

## Intent

Corvus now has a native `code_search` path with verified regex correctness and safety, plus an
indexed candidate-planning path for supported literal queries. However, regex requests do not yet
participate in indexed narrowing: `plan_candidates()` returns `query_regex_not_supported`, and regex
searches safely fall back to workspace discovery plus live verification.

This change exists to measure whether the native path is worth preferring over the current
shell/grep-based workflow, document exactly when and why fallback happens, and provide rollout
guidance grounded in representative data rather than assumptions.

## Scope

### In Scope

- Define representative benchmark coverage for `code_search` versus the current shell/grep-based
  search workflow across realistic repository query shapes.
- Capture benchmark results for both cold-index and warm-index native search behavior, including
  regex requests that currently fall back from index planning to discovery plus live verification.
- Document native `code_search` usage, current limitations, safety/correctness guarantees, and
  fallback behavior for unsupported index-planning query shapes.
- Document rollout guidance that explains when the native tool should be preferred over shell search
  and what evidence supports that recommendation.
- Separate non-v1 optimizations and future search-performance ideas from the rollout decision so the
  initial recommendation is based on today's verified behavior.

### Out of Scope

- Changing regex semantics, safety rules, or the current fallback behavior itself.
- Adding regex-aware index planning, case-insensitive index narrowing, whole-word index narrowing,
  or other search-engine optimizations as part of this change.
- Replacing live verification with index-only matches.
- Broad redesign of the `code_search` API or shell tool behavior beyond the benchmark/documentation
  work needed for rollout.

## Approach

Treat this as an evidence-and-guidance change, not a search-engine rewrite.

1. Establish a benchmark matrix that compares native `code_search` against the shell/grep-based
   workflow for representative literal and regex searches, including small-hit, large-hit, and
   no-hit cases.
2. Measure native behavior in at least two index states: cold/no reusable index and warm/compatible
   reusable index. Regex cases must be called out explicitly because they currently bypass indexed
   narrowing and execute through discovery plus live verification.
3. Publish benchmark results and methodology in repository documentation so reviewers can reproduce
   or interpret the rollout recommendation.
4. Update runtime tool documentation to explain tool usage, known limits, index-planning support
   boundaries, authoritative live verification, and current fallback reasons such as
   `query_regex_not_supported`.
5. Add a separate future-optimizations section so follow-up opportunities are visible without being
   mistaken for v1 rollout requirements.

The decision standard is practical: the rollout guidance should recommend the native path only where
the measured behavior, correctness guarantees, and operational ergonomics justify it today.

## Affected Areas

| Area                                             | Impact                               | Description                                                                                                                                        |
|--------------------------------------------------|--------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/tools/code_search.rs` | Referenced / possibly modified later | Source of truth for current native tool behavior, structured output, and fallback orchestration that benchmarks and docs must describe accurately. |
| `clients/agent-runtime/src/search/index.rs`      | Referenced / possibly modified later | Documents the current candidate-planning boundary, including `query_regex_not_supported` for regex queries and other unsupported narrowing modes.  |
| `clients/agent-runtime/src/search/tests.rs`      | Possibly modified later              | Likely home for regression or benchmark-adjacent validation proving documented fallback and index-state behavior remain accurate.                  |
| `docs/clients/agent-runtime/tools/`              | Modified                             | Add or update English documentation for `code_search` usage, benchmark findings, fallback behavior, rollout guidance, and deferred optimizations.  |
| `docs/es/clients/agent-runtime/tools/`           | Modified                             | Keep Spanish runtime-tool documentation aligned if rollout guidance is surfaced in localized docs.                                                 |
| `openspec/specs/regex-semantics/spec.md`         | Referenced in later phases           | Existing spec establishes regex correctness and live-verification authority that rollout docs must reflect.                                        |
| `openspec/specs/workspace-index/spec.md`         | Referenced in later phases           | Existing spec defines the indexed candidate model, advisory status, and cases where narrowing cannot safely apply.                                 |

## Risks

| Risk                                                                       | Likelihood | Mitigation                                                                                                                                                                                 |
|----------------------------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Benchmarks are not representative and lead to a bad rollout recommendation | Medium     | Define a benchmark matrix with realistic literal/regex, hit/no-hit, and cold/warm scenarios tied to actual repository usage patterns.                                                      |
| Documentation overstates native index support for regex queries            | Medium     | Explicitly document that regex correctness is supported, but regex candidate narrowing is not part of v1 and currently falls back from index planning to discovery plus live verification. |
| Results become stale as search internals evolve                            | Medium     | Document methodology, environment assumptions, and the exact behavior/version being measured so follow-up phases can refresh the numbers cleanly.                                          |
| Rollout guidance is interpreted as a hard deprecation of shell search      | Low        | Frame the recommendation as conditional guidance with clear tradeoffs, not a blanket removal of shell-based workflows.                                                                     |
| Future optimization ideas blur into current commitments                    | Medium     | Keep a dedicated non-v1 section that clearly labels deferred work and excludes it from rollout success criteria.                                                                           |

## Rollback Plan

If the benchmark methodology, results, or rollout recommendation prove misleading, revert the
benchmark and documentation updates and keep the existing shell-search guidance unchanged. Because
this change is intended to produce evidence and documentation rather than alter the core search
semantics, rollback is limited to removing or correcting the published benchmark and rollout
artifacts.

## Dependencies

- Existing native search behavior in `clients/agent-runtime/src/tools/code_search.rs`
- Existing candidate-planning constraints in `clients/agent-runtime/src/search/index.rs`
- Existing correctness guarantees in `openspec/specs/regex-semantics/spec.md`
- Existing indexed-candidate and freshness guarantees in `openspec/specs/workspace-index/spec.md`
- Issue #360 acceptance criteria and verified finding that regex queries currently fall back from
  index planning via `query_regex_not_supported`

## Success Criteria

- [ ] The change defines a representative benchmark matrix comparing native `code_search` and the
  current shell/grep-based workflow.
- [ ] Benchmark results capture cold-index and warm-index native behavior and explicitly account for
  regex queries that currently bypass indexed narrowing.
- [ ] Documentation explains native tool usage, correctness guarantees, known limitations, and
  fallback behavior, including the current regex-planning limitation.
- [ ] Rollout guidance states when the native path should be preferred, when shell search may still
  be appropriate, and why.
- [ ] Deferred optimizations are listed separately from v1 rollout expectations so future work does
  not distort the current recommendation.
