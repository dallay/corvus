# Tasks: `code_search` Benchmark Rollout and Documentation

## Phase 1: Benchmark Runner Foundation

- [x] 1.1 Create `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` with the benchmark case/result structs, environment metadata capture, and CLI entry flow for fixture and repo-snapshot runs.
- [x] 1.2 In `clients/agent-runtime/examples/code_search_rollout_benchmark.rs`, implement deterministic shell command generation plus native execution-state prep for `shell_baseline`, `native_no_index`, `native_cold_build`, and `native_warm_index`.
- [x] 1.3 Add example-local tests in `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` for shell command building, plan-mode labeling, and canonical line-match normalization.

## Phase 2: Parity and Runtime Verification

- [x] 2.1 Extend `clients/agent-runtime/src/search/tests.rs` with coverage proving regex requests still label as `fallback_discovery_live_verification` with reason `query_regex_not_supported` after index build.
- [x] 2.2 Add an end-to-end smoke case in `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` that runs one fixture scenario through shell and native paths and asserts canonical parity plus non-empty measurements.
- [x] 2.3 Update `clients/agent-runtime/benches/agent_benchmarks.rs` to separate Criterion microbenches from the rollout benchmark entrypoint so contributors run the right harness.

## Phase 3: Recorded Results and Rollout Docs

- [x] 3.1 Run `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` on the deterministic fixture workspace and the current repo snapshot, then record the measured rows and environment metadata in `docs/clients/agent-runtime/tools/code-search.md`.
- [x] 3.2 In `docs/clients/agent-runtime/tools/code-search.md`, document current behavior, live verification authority, regex fallback semantics, rollout recommendations, and a clearly separate non-v1 optimizations section.
- [x] 3.3 Update `clients/agent-runtime/docs/design/code-search-tool.md`, `docs/clients/agent-runtime/tools/core.md`, and `docs/clients/agent-runtime/tools/index.mdx` so internal and public references match the verified planner behavior and link the new page.
- [x] 3.4 Mirror the published benchmark summary, fallback wording, rollout guidance, and navigation updates in `docs/es/clients/agent-runtime/tools/code-search.md`, `docs/es/clients/agent-runtime/tools/core.md`, and `docs/es/clients/agent-runtime/tools/index.mdx`.

## Phase 4: Final Verification

- [x] 4.1 Run targeted Rust validation for the new runner and search coverage in `clients/agent-runtime` and fix any parity, planner-label, or example-test regressions before marking the change complete.
- [x] 4.2 Review the English and Spanish docs against `openspec/changes/code-search-benchmark-rollout/specs/code-search-rollout/spec.md` and confirm they never imply indexed regex narrowing or blanket shell deprecation.
