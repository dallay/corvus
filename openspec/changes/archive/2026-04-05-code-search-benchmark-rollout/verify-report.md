# Verification Report

**Change**: code-search-benchmark-rollout  
**Date**: 2026-04-05

## Completeness

| Metric           | Value |
|------------------|------:|
| Tasks total      |    10 |
| Tasks complete   |    10 |
| Tasks incomplete |     0 |

All tasks in `openspec/changes/code-search-benchmark-rollout/tasks.md` are marked complete.

## Build & Test Execution

### Executed during verification

1. `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`
    - Result: ✅ passed
    - Evidence: formatting re-check completed successfully for `clients/agent-runtime/**/*.rs`
      during re-verification.

2. `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`
    - Result: ✅ passed
    - Evidence: Clippy completed cleanly for `clients/agent-runtime/**/*.rs`, including the rollout
      benchmark example target after removing the leaked absolute-path output and fixing the
      percentile / regex benchmark findings.

3. `cargo test --manifest-path clients/agent-runtime/Cargo.toml`
    - Result: ✅ passed
    - Evidence: full `clients/agent-runtime` test coverage passed, including the main lib test
      binaries (`3402` and `3429` passing tests) plus the rollout benchmark example tests.

4.

`cargo run --manifest-path clients/agent-runtime/Cargo.toml --example code_search_rollout_benchmark -- --workspace fixture --samples 1 --cold-build-samples 1`
- Result: ✅ passed
- Evidence: fixture benchmark executed end-to-end, emitted shell/native rows for literal and
regex cases, and labeled regex rows as `fallback_discovery_live_verification` with
`query_regex_not_supported`.

5. `node ../../../../scripts/validate-docs-metadata.mjs`
    - Result: ✅ passed
    - Evidence: documentation metadata validation passed for the English and Spanish `code_search`
      rollout pages after the required frontmatter fixes.

### Not rerun during verification

- Full repo-snapshot benchmark (`--workspace repo` / `both` with published sample counts) was **not
  ** rerun successfully in this verify pass.
- Orchestration context states that the repo benchmark rerun still times out; this verify pass
  relies on the recorded repo-snapshot results already published in the docs.

## Spec Compliance Matrix

| Requirement | Scenario                                                                   | Evidence                                                                                                                                                          | Result      |
|-------------|----------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------|
| REQ-CSR-001 | Benchmark matrix covers representative native and shell comparisons        | `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` fixture/repo case matrix + successful fixture benchmark run + published matrix tables in docs   | ✅ COMPLIANT |
| REQ-CSR-001 | Regex fallback cases are benchmarked explicitly                            | example runner labels regex fallback; planner regression test; docs label regex as fallback with `query_regex_not_supported` and not regex-aware narrowing        | ✅ COMPLIANT |
| REQ-CSR-002 | Recorded results preserve comparison outcomes by execution mode            | published English/Spanish result tables include shell, no-index, cold-build, warm-index, plan mode, reason, parity                                                | ✅ COMPLIANT |
| REQ-CSR-002 | Results remain interpretable when behavior evolves later                   | docs record workspace kind, SHA, host, file count, timestamps, debug profile, methodology                                                                         | ✅ COMPLIANT |
| REQ-CSR-003 | Documentation describes regex support without overstating index support    | English/Spanish `code-search.md`, tool references, and internal design doc explicitly preserve regex correctness + `query_regex_not_supported` fallback semantics | ✅ COMPLIANT |
| REQ-CSR-003 | Documentation preserves live verification as the source of truth           | canonical docs explicitly state final matches come from live verification                                                                                         | ✅ COMPLIANT |
| REQ-CSR-004 | Rollout guidance recommends native only where evidence supports it         | guidance distinguishes regex measured wins, literal warm/cold tradeoffs, and unmeasured-case limits                                                               | ✅ COMPLIANT |
| REQ-CSR-004 | Rollout guidance leaves room for shell where appropriate                   | docs retain conditional `MAY keep shell / grep` guidance and avoid blanket deprecation                                                                            | ✅ COMPLIANT |
| REQ-CSR-005 | Deferred optimization ideas are documented separately from v1 requirements | dedicated non-v1 future optimization sections in English/Spanish docs                                                                                             | ✅ COMPLIANT |
| REQ-CSR-005 | v1 rollout decision does not imply search-engine behavior changes          | docs and code preserve current planner reason/limits; no regex-aware narrowing added                                                                              | ✅ COMPLIANT |

Compliance summary: 10/10 scenarios compliant based on executed tests plus artifact inspection.

## Correctness (Static)

| Requirement | Status        | Notes                                                                                                                             |
|-------------|---------------|-----------------------------------------------------------------------------------------------------------------------------------|
| REQ-CSR-001 | ✅ Implemented | Dedicated rollout runner defines literal/regex, hit-shape, and execution-state coverage for fixture and repo snapshot workspaces. |
| REQ-CSR-002 | ✅ Implemented | Docs publish measured tables, environment metadata, parity, plan mode, and plan reason.                                           |
| REQ-CSR-003 | ✅ Implemented | Docs and tests correctly preserve regex support without implying regex-aware narrowing.                                           |
| REQ-CSR-004 | ✅ Implemented | Rollout guidance is conditional and evidence-based, not blanket shell deprecation.                                                |
| REQ-CSR-005 | ✅ Implemented | Future optimizations are clearly separated from v1 rollout guidance.                                                              |

## Coherence (Design)

| Decision                                             | Followed? | Notes                                                                                             |
|------------------------------------------------------|-----------|---------------------------------------------------------------------------------------------------|
| Dedicated rollout benchmark runner                   | ✅ Yes     | Implemented at `clients/agent-runtime/examples/code_search_rollout_benchmark.rs`.                 |
| Shell baseline through `ShellTool` + `NativeRuntime` | ✅ Yes     | Runner instantiates `ShellTool::new(... NativeRuntime ...)` and benchmarks `grep` via shell.      |
| Explicit native evidence states                      | ✅ Yes     | `native_no_index`, `native_cold_build`, and `native_warm_index` are implemented and documented.   |
| Canonical correctness parity gate                    | ✅ Yes     | Example tests and fixture benchmark validate canonical shell/native parity.                       |
| Deferred optimizations kept out of v1                | ✅ Yes     | Docs isolate non-v1 items in dedicated future-optimization sections.                              |
| File changes table alignment                         | ✅ Yes     | Runner, bench note, planner regression, internal doc, English docs, and Spanish docs are present. |

## Issues Found

### CRITICAL

None.

### WARNING

1. Repo-snapshot benchmark evidence was not rerun successfully in this verify pass; published repo
   numbers were reviewed from docs, but fresh execution evidence is still missing because the rerun
   timed out.
2. The recorded benchmark results live in docs only; there is still no separate machine-readable
   artifact for independent diffing or replay.

### SUGGESTION

1. Add a narrower repo benchmark mode (subset or case filter) so verify can rerun repo evidence
   without timing out.
2. Add a docs-consistency check or exported artifact to reduce risk of table drift between measured
   output and published docs.

## Verdict

**PASS WITH WARNINGS**

The implementation matches the proposal/spec/design/tasks and preserves the required regex fallback
semantics (`query_regex_not_supported` with no regex-aware narrowing). Formatting is now clean,
targeted behavioral evidence still passes, and the remaining gap is limited to fresh repo-snapshot
benchmark reproduction.