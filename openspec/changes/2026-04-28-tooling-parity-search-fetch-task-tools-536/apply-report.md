# Apply Report: Tooling parity for search, fetch, and task tools #536

## Change

- **Change ID:** `2026-04-28-tooling-parity-search-fetch-task-tools-536`
- **Scope:** Additive snake_case compatibility aliases for canonical search, fetch, and persistent task parity tools, plus deterministic publication of canonical-to-alias mapping without widening effective capabilities.

## Outcome

Apply work for this change has been completed and the missing apply artifacts have now been persisted.

The implemented slice delivers:

- alias-aware runtime execution for `glob`, `grep`, `web_fetch`, `task_create`, `task_get`, `task_list`, `task_update`, and `task_stop`;
- centralized parity alias metadata shared by runtime resolution and publication surfaces;
- policy/risk evaluation parity so aliases cannot bypass canonical validation or approval boundaries;
- deterministic canonical-first alias publication in agent-facing inventory/instruction surfaces;
- regression coverage across execution, policy, mapping, registry publication, and provider publication.

## Implementation Summary

The completed apply work covered these primary runtime surfaces:

- `clients/agent-runtime/src/agent/agent.rs`
  - resolves tool calls by canonical tool name or declared alias metadata.
- `clients/agent-runtime/src/tools/mod.rs`
  - centralizes the parity alias source of truth with shared canonical/alias mapping helpers.
- `clients/agent-runtime/src/tools/{glob,grep,web_fetch,task_create,task_get,task_list,task_update,task_stop}.rs`
  - derive additive alias metadata from the centralized parity map instead of duplicating local name tables.
- `clients/agent-runtime/src/agent/dispatcher.rs`
  - canonicalizes alias names for policy/risk decisions through the shared mapping;
  - publishes canonical names plus deterministic alias suffixes in XML/agent-facing prompt instructions.
- `clients/agent-runtime/src/capabilities/tool_registration.rs`
  - preserves canonical registry entries with alias metadata and avoids duplicate registration.
- `clients/agent-runtime/src/providers/compatible.rs`
  - avoids publishing aliases as separate provider tool definitions.
- `clients/agent-runtime/src/agent/tests.rs`
  - verifies real parity alias execution reaches the canonical implementation.

## Verification Evidence

The following focused verification commands were run successfully for this slice:

```bash
cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all
cargo test --manifest-path clients/agent-runtime/Cargo.toml xml_prompt_instructions_publish_aliases_deterministically -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml parity_alias_mapping_round_trips -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml turn_executes_tool_via_alias_then_returns -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml test_risk_classification -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml registry_from_tools_preserves_canonical_and_alias_metadata_without_duplicate_entries -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml tool_specs_convert_to_openai_format_do_not_publish_aliases_as_duplicate_tools -- --nocapture
cargo test --manifest-path clients/agent-runtime/Cargo.toml tool_spec_generation -- --nocapture
git diff --check -- clients/agent-runtime/src/agent/agent.rs clients/agent-runtime/src/agent/dispatcher.rs clients/agent-runtime/src/agent/tests.rs clients/agent-runtime/src/tools/mod.rs clients/agent-runtime/src/tools/glob.rs clients/agent-runtime/src/tools/grep.rs clients/agent-runtime/src/tools/web_fetch.rs clients/agent-runtime/src/tools/task_create.rs clients/agent-runtime/src/tools/task_get.rs clients/agent-runtime/src/tools/task_list.rs clients/agent-runtime/src/tools/task_update.rs clients/agent-runtime/src/tools/task_stop.rs clients/agent-runtime/src/capabilities/tool_registration.rs clients/agent-runtime/src/providers/compatible.rs openspec/changes/2026-04-28-tooling-parity-search-fetch-task-tools-536/tasks.md
```

Observed result:

- focused parity alias execution passed;
- policy/risk parity checks passed;
- inventory/publication regression checks passed;
- no whitespace or diff formatting issues remained after formatting.

## Task State

`tasks.md` for this change is checked complete across all listed phases.

## Audit Completion

This report, together with `apply-result.json` and `state.yaml`, closes the missing apply-artifact gap for this change so the OpenSpec audit chain is complete.
