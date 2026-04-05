# code_search Benchmark Rollout Specification

## Purpose

Defines the evidence, documentation, and rollout guidance required to decide when Corvus should
prefer the native `code_search` tool over the current shell/grep-based workflow for Issue #360.

This specification covers benchmark coverage, recorded benchmark results, documented current
behavior and limits, rollout recommendations grounded in measured data, and explicit separation of
future optimizations from v1 rollout requirements. It does not change `code_search` matching
semantics, safety guarantees, or index-planning behavior.

## Requirements

### Requirement: REQ-CSR-001 Benchmark Matrix Coverage

The change MUST define a benchmark matrix that compares native `code_search` with the current
shell/grep-based workflow across representative repository search shapes.

The benchmark matrix MUST include, at minimum:

- literal searches and regex searches,
- small-hit, large-hit, and no-hit queries,
- native search runs in cold/no-reusable-index and warm/reusable-index states,
- explicit coverage for regex requests that currently bypass indexed candidate narrowing and fall
  back from planning with `query_regex_not_supported`,
- enough query and repository context for a reviewer to understand what each benchmark is intended
  to represent.

#### Scenario: Benchmark matrix covers representative native and shell comparisons

- GIVEN the benchmark plan for `code_search` rollout
- WHEN a reviewer inspects the documented benchmark matrix
- THEN the matrix MUST include native-versus-shell comparisons for both literal and regex queries
- AND it MUST include small-hit, large-hit, and no-hit cases
- AND it MUST distinguish cold-index native runs from warm-index native runs

#### Scenario: Regex fallback cases are benchmarked explicitly

- GIVEN the verified v1 behavior that regex correctness is supported but indexed narrowing is not
- WHEN the benchmark matrix documents regex scenarios
- THEN it MUST identify those scenarios as fallback cases from index planning
- AND it MUST describe them as discovery plus live verification rather than indexed narrowing
- AND it MUST NOT describe regex benchmarks as evidence of native regex-aware candidate planning

### Requirement: REQ-CSR-002 Recorded Results and Methodology

The change MUST publish recorded benchmark results together with enough methodology and environment
context for reviewers to interpret or reproduce the rollout recommendation.

The recorded results MUST, at minimum:

- preserve the measured outcomes for each benchmark matrix entry,
- distinguish shell results from native cold-index and native warm-index results where applicable,
- identify which native cases executed through indexed candidate narrowing versus fallback
  discovery plus live verification,
- record the repository, environment assumptions, and measurement method used for the published
  results,
- avoid presenting anecdotal or partial observations as rollout guidance.

#### Scenario: Recorded results preserve comparison outcomes by execution mode

- GIVEN a completed benchmark run for the rollout change
- WHEN the benchmark results are published
- THEN each documented case MUST preserve the measured result for the shell workflow
- AND it MUST preserve the measured result for native search in each relevant index state
- AND it MUST indicate whether the native case used indexed candidate narrowing or fallback
  discovery plus live verification

#### Scenario: Results remain interpretable when behavior evolves later

- GIVEN benchmark results published for the current version of `code_search`
- WHEN a later reviewer compares those results to newer runtime behavior
- THEN the published methodology MUST identify the measured behavior and environment assumptions
- AND the reviewer MUST be able to tell that the recorded numbers apply to the documented v1
  behavior rather than an unspecified future implementation

### Requirement: REQ-CSR-003 Documentation of Current Behavior, Limits, and Fallbacks

The runtime-tool documentation MUST explain the current `code_search` behavior relevant to rollout
without overstating native search capabilities.

That documentation MUST, at minimum:

- describe the expected search behavior and current correctness guarantees,
- state that live verification against current file contents remains authoritative,
- state that regex correctness and safety are supported,
- state that indexed candidate narrowing currently does NOT support regex requests and that
  `query_regex_not_supported` causes fallback to discovery plus live verification,
- document any known v1 limitations that materially affect rollout interpretation,
- distinguish current verified behavior from deferred optimization ideas.

#### Scenario: Documentation describes regex support without overstating index support

- GIVEN the runtime-tool documentation for `code_search`
- WHEN a reader looks for regex behavior details
- THEN the documentation MUST state that regex matching is supported with the existing correctness
  and safety guarantees
- AND it MUST state that regex candidate narrowing is not part of v1 indexed planning
- AND it MUST describe `query_regex_not_supported` as a fallback reason rather than a search error

#### Scenario: Documentation preserves live verification as the source of truth

- GIVEN a reader evaluating whether indexed search results are authoritative on their own
- WHEN the reader consults the rollout documentation
- THEN the documentation MUST state that final matches come from live verification of current file
  contents
- AND it MUST NOT imply that indexed candidates alone are sufficient to report a match

### Requirement: REQ-CSR-004 Evidence-Based Rollout Guidance

The change MUST provide rollout guidance that explains when native `code_search` SHOULD be
preferred over shell/grep-based search and when shell search MAY still be appropriate.

That guidance MUST be grounded in the recorded benchmark results and documented current behavior.
It MUST, at minimum:

- identify the benchmark-supported cases where native search is recommended,
- identify cases where shell search remains reasonable or preferred,
- explain the tradeoffs using current evidence instead of assumptions,
- avoid presenting the recommendation as a blanket deprecation of shell search,
- align the recommendation with current v1 behavior, including regex fallback and live
  verification.

#### Scenario: Rollout guidance recommends native search only where evidence supports it

- GIVEN published benchmark results for native and shell workflows
- WHEN the rollout recommendation is written
- THEN the recommendation MUST identify the situations where native `code_search` SHOULD be
  preferred based on the measured outcomes and documented ergonomics
- AND it MUST explain why those situations justify preference today
- AND it MUST NOT claim preference for unmeasured or unsupported cases

#### Scenario: Rollout guidance leaves room for shell search where appropriate

- GIVEN the current shell/grep-based workflow remains available
- WHEN the rollout guidance discusses non-preferred native cases or operator tradeoffs
- THEN it MUST identify cases where shell search MAY remain appropriate
- AND it MUST frame that guidance as conditional tradeoffs rather than removal of shell-based
  workflows

### Requirement: REQ-CSR-005 Deferred Optimization Separation

The change MUST separate deferred optimization opportunities from v1 rollout requirements.

Deferred items MAY include future search-performance or index-planning improvements, but they MUST
be labeled as non-v1 work and MUST NOT be required for the current rollout recommendation.

#### Scenario: Deferred optimization ideas are documented separately from v1 requirements

- GIVEN the rollout artifacts for this change
- WHEN a reviewer reads about future opportunities
- THEN those items MUST appear in a clearly separate deferred or future-optimization section
- AND they MUST be labeled as outside the v1 rollout requirements
- AND their absence MUST NOT invalidate the current benchmark-backed recommendation

#### Scenario: v1 rollout decision does not imply search-engine behavior changes

- GIVEN this change is limited to benchmarks, documentation, and rollout guidance
- WHEN a reviewer inspects the specification and artifacts
- THEN the v1 requirements MUST NOT require regex-aware indexed candidate narrowing or other new
  search-engine optimizations
- AND the rollout decision MUST be expressed using the currently verified behavior