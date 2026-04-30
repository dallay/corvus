# Tasks: Tooling parity search, fetch, and task tools #536

## Phase 1: Alias contract and regression red

- [x] 1.1 RED: Add failing tests for alias-aware resolution of `glob`, `grep`, `web_fetch`, `task_create`, `task_get`, `task_list`, `task_update`, and `task_stop` to the same runtime implementations as their canonical PascalCase parity tools.
- [x] 1.2 RED: Add failing tests proving alias invocation preserves the same validation, security, and backend-availability behavior as the canonical tool for representative search, fetch, and task paths.
- [x] 1.3 RED: Add failing tests for published inventory or mapping surfaces so canonical names and aliases are rendered deterministically without implying separate implementations.

## Phase 2: Runtime alias wiring green

- [x] 2.1 GREEN: Update runtime tool registration in `clients/agent-runtime` so the snake_case compatibility aliases resolve to the same implementations as `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`.
- [x] 2.2 GREEN: Centralize parity mapping metadata so alias resolution and published inventory use one source of truth instead of duplicated name tables.
- [x] 2.3 GREEN: Ensure alias resolution cannot bypass existing policy, validation, or backend support checks and does not broaden effective capabilities.
- [x] 2.4 REFACTOR: Clean up any ad hoc naming logic so canonical-versus-alias presentation remains concise and consistent with the existing hybrid naming design.

## Phase 3: Inventory and documentation surfaces

- [x] 3.1 GREEN: Update the relevant inventory, slash-command, or agent-facing documentation surfaces to publish canonical names and compatibility aliases in a stable, deterministic format.
- [x] 3.2 GREEN: Ensure published messaging explains that PascalCase names remain canonical for this slice and snake_case names are additive compatibility aliases.
- [x] 3.3 VERIFY: Add or update regression coverage confirming inventory publication stays aligned with the centralized mapping metadata.

## Phase 4: Verification and bounded cleanup

- [x] 4.1 VERIFY: Run targeted runtime tests covering alias resolution, inventory publication, and representative search/fetch/task behavior.
- [x] 4.2 VERIFY: Run the most relevant repository verification commands for touched runtime surfaces without widening scope beyond this parity slice.
- [x] 4.3 REFACTOR: Apply only narrow cleanup directly related to alias wiring, published mapping, or parity test readability if needed.
