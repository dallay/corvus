# Proposal: Indexed Candidate Selection for `code_search`

**Issue**: #356

## Intent

`code_search` currently preserves correctness by scanning discovered workspace files directly, but
it does not yet use the local trigram index to narrow the work before verifying real matches. That
leaves the runtime paying full scan costs even when a safe local index can cheaply eliminate obvious
non-candidates.

This change introduces an indexed candidate-selection path that uses the local workspace trigram
index to derive deterministic candidate files, then verifies every reported match against real file
contents before returning results. Correctness stays ahead of speed: the index only reduces the
candidate set, it does not become the source of truth for final matches.

## Scope

### In Scope

- Add a trigram-based candidate selection step under `clients/agent-runtime/src/search/*` that
  derives workspace-relative candidate files for a query.
- Keep `clients/agent-runtime/src/tools/code_search.rs` as the orchestration layer that compiles the
  pattern, enforces security/resource limits, chooses the candidate strategy, and verifies matches
  against live file contents.
- Preserve deterministic ordering for candidate processing and returned verified matches.
- Apply `max_results` to verified matches, not just raw candidates, so truncation semantics remain
  correct.
- Extend machine-readable match data so verified results include file path plus line/offset context
  and preview data suitable for downstream tooling.
- Add a safe fallback to the existing discovery-based scan path when indexed candidate extraction is
  unavailable, too weak, or would risk missing correct matches.
- Add regression coverage for indexed filtering, fallback behavior, ordering, limits, and
  result-shape guarantees.

### Out of Scope

- Replacing final content verification with index-only hits.
- Ranking, fuzzy scoring, semantic retrieval, or AST-aware search.
- Background indexing daemons, watch mode, or cross-workspace caches.
- Changing the existing workspace safety model, ignore rules, or regex semantics.
- Broad redesign of the `code_search` API beyond the structured fields needed for verified match
  metadata.

## Approach

Implement the change in two layers with correctness-preserving boundaries:

1. **Indexed candidate derivation** — extend the search/index layer to derive a deterministic list
   of workspace-relative candidate files from the local trigram index for queries where trigrams are
   meaningful.
2. **Verification against real contents** — keep `code_search` responsible for reading candidate
   files, applying the real regex/literal matcher to file contents, collecting context/preview data,
   and enforcing per-file and total-result limits on verified matches.
3. **Safe fallback** — when the query cannot produce useful trigrams, when candidate extraction
   yields no trustworthy reduction, or when the index is unavailable/incompatible, fall back to the
   current discovery walk so observable correctness is preserved.
4. **Stable result contract** — return verified matches in deterministic file/path order with
   structured fields that expose file identity, exact verified location, surrounding context, and
   preview text without making the index itself externally observable as a source of truth.

The index should be treated as an optimization gate only. Every returned match must still come from
live file-content verification, and fallback must prefer full correctness over partial indexed
speedups.

## Affected Areas

| Area                                                        | Impact   | Description                                                                                                                                                                  |
|-------------------------------------------------------------|----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/tools/code_search.rs`            | Modified | Keep tool orchestration here: choose indexed vs fallback path, verify real matches, enforce deterministic ordering and verified-result limits, and format structured output. |
| `clients/agent-runtime/src/search/index.rs`                 | Modified | Add or expose query-time candidate selection over the local trigram index while preserving workspace-relative identities and index compatibility rules.                      |
| `clients/agent-runtime/src/search/sqlite.rs`                | Modified | Add the SQLite reads needed to derive deterministic candidate file sets from persisted trigram postings.                                                                     |
| `clients/agent-runtime/src/search/mod.rs`                   | Modified | Export the search/index query surface used by `code_search` without moving orchestration out of the tool layer.                                                              |
| `clients/agent-runtime/src/search/tests.rs`                 | Modified | Add regression tests for indexed candidate derivation, deterministic ordering, and safe fallback behavior.                                                                   |
| `clients/agent-runtime/src/tools/code_search.rs` test block | Modified | Add end-to-end tool tests covering verified limits, structured match metadata, and correctness parity when fallback is used.                                                 |

## Risks

| Risk                                                                        | Likelihood | Mitigation                                                                                                                                                                            |
|-----------------------------------------------------------------------------|------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Indexed candidate extraction drops valid files and causes false negatives   | Medium     | Treat the index as a candidate reducer only, verify all returned matches from live contents, and fall back to full discovery when candidate derivation is not clearly safe or useful. |
| Query-time ordering becomes nondeterministic across runs                    | Medium     | Sort candidate file identities and preserve stable verified-match emission order based on workspace-relative path and in-file position.                                               |
| `max_results` is applied before verification and changes current semantics  | Medium     | Keep truncation and stats based on verified matches only, with regression tests proving parity.                                                                                       |
| Structured output changes break downstream consumers                        | Low        | Extend the match schema in a backward-compatible way where possible and add explicit result-format tests for the new fields.                                                          |
| Indexed path adds complexity without helping short or non-selective queries | Medium     | Define a conservative fallback threshold so the runtime can safely reuse the current scan path when the index does not provide clear value.                                           |

## Rollback Plan

If indexed candidate selection causes missed matches, unstable ordering, or operational regressions,
revert the query-time index integration in `clients/agent-runtime/src/search/*` and restore
`clients/agent-runtime/src/tools/code_search.rs` to the current discovery-first scan path. Because
the index remains derived workspace state and verification still reads live files, rollback does not
require data migration; the runtime can simply ignore indexed candidate selection and continue using
the existing scan-only behavior.

## Dependencies

- Existing workspace trigram index lifecycle in `clients/agent-runtime/src/search/index.rs`
- Existing persisted trigram postings schema in `clients/agent-runtime/src/search/sqlite.rs`
- Existing safe discovery and verification behavior in
  `clients/agent-runtime/src/search/discovery.rs` and
  `clients/agent-runtime/src/tools/code_search.rs`
- Existing OpenSpec contracts in `openspec/specs/workspace-index/spec.md`,
  `openspec/specs/result-format/spec.md`, `openspec/specs/regex-semantics/spec.md`, and
  `openspec/specs/safety-model/spec.md`

## Success Criteria

- [ ] `code_search` can use the local workspace trigram index to derive candidate files without
  making the index the source of truth for final matches.
- [ ] Every returned result is verified against real file contents before it is emitted.
- [ ] Verified results are returned in deterministic order across repeated runs on unchanged inputs.
- [ ] `max_results` and truncation behavior apply to verified matches, not merely candidate files.
- [ ] Structured match payloads include machine-readable file path, verified line/offset location,
  relevant context, and preview content.
- [ ] When indexed candidate extraction is unavailable or not useful, `code_search` safely falls
  back to the existing scan path without losing correctness.
- [ ] Automated tests prove indexed selection, fallback parity, ordering, and result-shape
  guarantees.
