# Design: `code_search` Benchmark Rollout and Evidence-Based Guidance

## Technical Approach

This change will add a dedicated rollout benchmark workflow for `code_search`, publish measured
results in runtime-tool documentation, and derive rollout guidance from those measurements without
changing search semantics or planner behavior.

The implementation will treat rollout as an evidence pipeline with four parts:

1. a deterministic benchmark runner that exercises the current shell baseline and native
   `code_search` path against the same scenario matrix,
2. explicit native execution states (`no index`, `cold build`, `warm index`) so reviewers can see
   the cost of first use versus reuse,
3. a correctness-parity pass that proves the benchmarked shell and native cases return the same
   canonical match set before any preference claim is made,
4. documentation updates that record methodology, results, current fallback behavior, rollout
   guidance, and deferred work.

This maps directly to the spec by keeping v1 focused on benchmark coverage, recorded results,
current-behavior documentation, measured rollout guidance, and strict separation of deferred
optimizations.

## Architecture Decisions

### Decision: Use a dedicated rollout benchmark runner instead of only extending Criterion microbenches

**Choice**: Add a dedicated Rust benchmark runner for rollout evidence, while leaving the existing
Criterion microbench file as a lower-level performance harness.

**Alternatives considered**:
- Extend `clients/agent-runtime/benches/agent_benchmarks.rs` only.
- Measure ad hoc shell and native commands manually and paste results into docs.

**Rationale**: rollout evidence needs stateful setup/teardown, shell-tool invocation, explicit
index-state control, correctness checks, and a docs-friendly summary. Criterion is good for hot
loops, but awkward for “delete index → build index → search → compare → label plan mode” flows.
Ad hoc measurements would not be reproducible enough for Issue #360.

### Decision: Benchmark the real baseline as the `shell` tool executing grep through the native runtime

**Choice**: Treat the shell comparator as the current generic `shell` tool path, using `ShellTool`
with `NativeRuntime`, which in turn executes commands through `sh -c`, and benchmark grep-based
commands through that path.

**Alternatives considered**:
- Benchmark raw `grep` directly without the shell tool.
- Compare only native `code_search` states and omit the shell baseline.

**Rationale**: the rollout decision is about replacing or preferring the current agent workflow,
not about comparing Rust code to a hypothetical external command. Using the actual shell-tool path
 preserves command-validation, shell startup, and runtime wrapping overhead that users experience
today.

### Decision: Model native execution as explicit evidence states

**Choice**: Report native results in three separate states:
- **No index**: `state/code-search/index.db` absent; native search executes with fallback behavior.
- **Cold build**: no reusable index exists, `refresh_or_rebuild()` is timed, then the first search
  is timed and reported as `build_ms`, `search_ms`, and `total_ms`.
- **Warm index**: a compatible index already exists; search is timed without rebuild work.

**Alternatives considered**:
- Report a single “native” number.
- Benchmark only warm-index performance.

**Rationale**: the rollout question is operational. A single number would hide the difference
between first-use cost and steady-state reuse. The current implementation already exposes the
necessary primitives through `WorkspaceTrigramIndex::refresh_or_rebuild()` and
`WorkspaceTrigramIndex::plan_candidates()`.

### Decision: Gate rollout claims on canonical correctness parity for the benchmarked overlap set

**Choice**: Add a parity comparator that canonicalizes shell and native outputs to the same
line-oriented match model and require those benchmark cases to agree before documentation makes a
preference claim.

**Alternatives considered**:
- Use timing only and assume correctness from existing tests.
- Compare raw text output strings from shell and native tools.

**Rationale**: rollout guidance must be evidence-based, and timing without parity would be
misleading. Raw text output is not stable enough because `code_search` returns structured matches
and grep returns line-oriented text. Canonical comparison keeps the measurement fair while leaving
broader regex semantics to the existing specs and tests.

### Decision: Keep deferred planner/search optimizations explicitly out of v1

**Choice**: Document deferred items in a separate “Future optimizations / non-v1” section and do
not let them affect rollout success criteria.

**Alternatives considered**:
- Fold regex-aware narrowing and related planner improvements into the rollout work.
- Mention future ideas inline with the recommendation.

**Rationale**: the spec is about benchmarking and documentation for today’s verified behavior.
Mixing future planner work into v1 would blur the decision boundary and weaken the evidence trail.

## Data Flow

### Benchmark execution flow

```text
Scenario matrix
    │
    ├── Shell baseline case ──→ ShellTool ──→ NativeRuntime ──→ sh -c "grep ..."
    │
    └── Native case ──→ WorkspaceTrigramIndex::plan_candidates()
                         │
                         ├── no index / unsupported query ──→ discovery + live verification
                         └── compatible literal query ──────→ indexed candidate narrowing + live verification
```

### Evidence publication flow

```text
Benchmark runner
    │
    ├── measurements by case/state
    ├── plan-mode labels (indexed vs fallback)
    ├── parity result per case
    ▼
Recorded results table
    │
    ├── docs/clients/agent-runtime/tools/code-search.md
    └── docs/es/clients/agent-runtime/tools/code-search.md
            │
            ▼
Rollout guidance derived from measured evidence
```

### Sequence diagram for one benchmark case

```text
Runner -> Workspace setup: prepare fixture or repo snapshot metadata
Runner -> Native state prep: delete index / build index / reuse index
Runner -> Planner: plan_candidates(request)
Planner --> Runner: coverage + reason
Runner -> ShellTool: execute(grep command)   [shell baseline]
ShellTool --> Runner: shell output + timing
Runner -> CodeSearchTool: execute(args)      [native path]
CodeSearchTool --> Runner: structured matches + timing
Runner -> Comparator: canonicalize(shell, native)
Comparator --> Runner: parity pass/fail
Runner -> Reporter: write measurement row with state, plan mode, parity, timings
```

## Benchmark Environments and States

The benchmark matrix will cover two workspace environments:

1. **Deterministic fixture workspace**
   - generated during the run,
   - tuned so benchmark cases have stable hit/no-hit characteristics,
   - constrained so parity cases have at most one logical match per line.

2. **Repository snapshot workspace**
   - the current checkout used for rollout measurement,
   - recorded with commit SHA, file count, and benchmark date,
   - used to make the recommendation relevant to real Corvus usage.

Each benchmark case will run in these native states when applicable:

| State | Preparation | What it represents |
|------|-------------|--------------------|
| `shell_baseline` | No native state prep; execute shell grep through `ShellTool` | Current generic search workflow |
| `native_no_index` | Remove `state/code-search/index.db` before each measured run | Native search with no reusable index |
| `native_cold_build` | Remove index, time `refresh_or_rebuild()`, then time first `code_search` | First-use cost when native indexing must be built |
| `native_warm_index` | Build or refresh index once before measurement loop | Steady-state native reuse |

Regex scenarios will still be run in `native_cold_build` and `native_warm_index`, but the runner
will label them as fallback cases because `plan_candidates()` returns
`query_regex_not_supported` and execution continues through discovery plus live verification.

## Comparison Method

Each benchmark row will be defined by:

- workspace environment,
- query kind (`literal` or `regex`),
- result shape (`small-hit`, `large-hit`, `no-hit`),
- optional scope/filter settings (`path`, `include`, `exclude`),
- execution mode (`shell_baseline`, `native_no_index`, `native_cold_build`, `native_warm_index`).

The runner will measure repeated samples per row and report:

- `samples`,
- `median_ms`,
- `p95_ms`,
- for cold builds: `build_median_ms`, `search_median_ms`, `total_median_ms`,
- `plan_mode` (`indexed_narrowing`, `fallback_discovery_live_verification`, or
  `index_unavailable`),
- `plan_reason` (for example `query_regex_not_supported` or `index_unavailable`).

Measurement rules:

1. Use the same immutable workspace contents for shell and native comparisons within a case.
2. Perform one untimed warm-up per row before collecting timed samples.
3. Report medians and p95s instead of means to reduce skew from filesystem jitter.
4. Keep shell and native command shapes equivalent at the benchmark-case level.
5. Record environment metadata alongside the results: OS, CPU, Rust profile, workspace type,
   commit SHA (for repo snapshot), and file count.

## Correctness-Parity Method

Parity is required for benchmarked comparisons but intentionally scoped to the overlap between the
current shell grep workflow and the current `code_search` behavior.

### Canonical comparison model

Both engines will be normalized to:

```rust
struct CanonicalLineMatch {
    file: String,
    line: usize,
    content: String,
}
```

- Shell output will be parsed from `grep -nH` style lines.
- Native output will be derived from `structured.matches`, collapsed to unique line entries.

### Parity scope rules

- Benchmark parity cases MUST use queries supported by both Rust regex and the chosen grep mode.
- Regex parity claims are limited to the shared syntax subset (no backreferences, lookaround, or
  other features neither engine consistently shares in this workflow).
- Fixture cases MUST avoid multiple relevant matches on the same line so line-level comparison
  remains exact.
- Existing unit/spec coverage remains the authority for native-only semantics and fallback
  behavior beyond the benchmark overlap set.

### Parity outcome handling

- A benchmark row is **eligible for rollout guidance** only if parity passes.
- A parity failure is recorded in the results table and blocks any “native SHOULD be preferred”
  statement for that row’s query class until explained or corrected.

## Rollout Guidance Derivation

Documentation will derive guidance from the recorded results using a fixed rubric rather than
narrative judgment alone.

### Gate 1: correctness

No recommendation is made unless the benchmarked class has parity pass for the shell/native
comparison rows that support the claim.

### Gate 2: execution-mode interpretation

- If `plan_mode = indexed_narrowing`, the recommendation may cite indexed candidate narrowing.
- If `plan_mode = fallback_discovery_live_verification`, the recommendation MUST describe the row
  as fallback behavior and MUST NOT imply regex-aware narrowing.

### Gate 3: performance bucket

For each benchmark class, compare native median time to shell median time:

- **Native win**: native median is at least 20% faster (`<= 0.8x` shell median).
- **Near parity**: native median is within ±20% of shell (`> 0.8x` and `<= 1.2x`).
- **Shell win**: native median is more than 20% slower (`> 1.2x` shell median).

### Guidance rules

- `SHOULD prefer native` when parity passes and native is a **Native win**, or when native is
  **Near parity** and the case benefits from structured output plus workspace-safe verification.
- `MAY prefer native` when parity passes but the result depends on warm-index reuse and cold/no-index
  costs are materially worse.
- `MAY keep shell` when parity passes but shell is a **Shell win**, especially for ephemeral,
  one-off, or regex-heavy fallback scenarios.
- `DO NOT claim native preference` for unmeasured or failed-parity cases.

This rubric lets the docs explain recommendations with measured evidence while keeping operator
tradeoffs explicit.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/examples/code_search_rollout_benchmark.rs` | Create | Dedicated rollout benchmark runner that prepares scenarios, executes shell/native states, records timings, labels plan mode, and checks parity. |
| `clients/agent-runtime/benches/agent_benchmarks.rs` | Modify | Add a short note or companion entrypoint reference if needed so low-level microbenches and rollout benchmarks are clearly separated. |
| `clients/agent-runtime/docs/design/code-search-tool.md` | Modify | Align the internal design doc with the implemented indexed-planning reality and point readers to the canonical rollout documentation for current behavior. |
| `docs/clients/agent-runtime/tools/code-search.md` | Create | Canonical English documentation for current behavior, benchmark methodology, recorded results, rollout guidance, fallback reasons, and deferred optimizations. |
| `docs/clients/agent-runtime/tools/core.md` | Modify | Add `code_search` to the core tools reference and link to the dedicated page. |
| `docs/clients/agent-runtime/tools/index.mdx` | Modify | Link the dedicated `code_search` page from the tools index. |
| `docs/es/clients/agent-runtime/tools/code-search.md` | Create | Spanish companion page with the same behavior limits, benchmark summary, rollout guidance, and deferred-work separation. |
| `docs/es/clients/agent-runtime/tools/core.md` | Modify | Add the Spanish `code_search` reference and link to the dedicated page. |
| `docs/es/clients/agent-runtime/tools/index.mdx` | Modify | Add the Spanish navigation link for the dedicated `code_search` page. |

## Interfaces / Contracts

The rollout benchmark runner will use explicit scenario and result structs so measurements can be
reported consistently.

```rust
enum QueryKind {
    Literal,
    Regex,
}

enum ResultShape {
    SmallHit,
    LargeHit,
    NoHit,
}

enum ExecutionMode {
    ShellBaseline,
    NativeNoIndex,
    NativeColdBuild,
    NativeWarmIndex,
}

enum PlanMode {
    IndexedNarrowing,
    FallbackDiscoveryLiveVerification,
    IndexUnavailable,
}

struct BenchmarkCase {
    id: &'static str,
    query_kind: QueryKind,
    result_shape: ResultShape,
    pattern: String,
    is_regex: bool,
    path: String,
    include: Vec<String>,
    exclude: Vec<String>,
    case_sensitive: bool,
    whole_word: bool,
}

struct BenchmarkMeasurement {
    case_id: String,
    execution_mode: ExecutionMode,
    plan_mode: PlanMode,
    plan_reason: String,
    samples: usize,
    median_ms: u64,
    p95_ms: u64,
    build_median_ms: Option<u64>,
    search_median_ms: Option<u64>,
    parity_passed: bool,
}
```

Shell command construction contract:

- literal cases use grep fixed-string mode,
- regex cases use grep extended-regex mode,
- path/include filters are mapped only for the supported benchmark overlap set,
- command generation is deterministic and test-covered.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Shell command builder for literal/regex/path/include cases | Example-local tests that verify generated grep commands match the intended benchmark case shape. |
| Unit | Plan-mode labeling | Tests that map `CandidateCoverage` + reason (`query_regex_not_supported`, `index_unavailable`, etc.) to the reported benchmark label. |
| Unit | Canonical comparator | Feed shell lines and native structured matches into the comparator and verify pass/fail behavior, including duplicate-line rejection. |
| Integration | Native state preparation | Run a temp-workspace case that proves `native_no_index`, `native_cold_build`, and `native_warm_index` produce the expected planner state transitions. |
| Integration | Regex fallback labeling | Use a built index plus a regex case and verify the benchmark row is labeled `fallback_discovery_live_verification` with reason `query_regex_not_supported`. |
| Integration | End-to-end benchmark row | Execute one small fixture case through both shell and native modes and assert parity plus non-empty measurement output. |
| Docs | Recorded results consistency | Manual review during this change: benchmark tables in English and Spanish docs must describe the same measured recommendation and explicitly mark deferred items as non-v1. |

## Migration / Rollout

No migration is required.

Rollout is documentation-led:

1. implement the benchmark runner,
2. execute the benchmark matrix on the chosen environments,
3. record the methodology and results in the dedicated docs page,
4. derive the recommendation using the rubric above,
5. keep deferred optimizations in a separate section clearly labeled out of scope for v1.

The docs will explicitly separate:

- **Current measured recommendation**,
- **Current limitations and fallback reasons**,
- **Future optimizations (non-v1)** such as regex-aware index narrowing, case-insensitive index
  narrowing, whole-word index narrowing, or other planner/search-engine changes.

## Open Questions

- [ ] Confirm whether the rollout benchmark should check in a machine-readable artifact in addition
      to the recorded markdown tables, or whether the docs page alone is the repository source of
      truth for measured results.
- [ ] Confirm the exact benchmark sample counts for cold-build versus warm-index rows so runtime
      cost stays practical in CI/local execution without weakening the evidence.
