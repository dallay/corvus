# Tooling Parity Specification

## Purpose

Define the first implementation slice for Claude-style search and fetch parity in Corvus.
This specification covers the dedicated `Glob`, `Grep`, and read-only `WebFetch` tool contracts,
their validation and security boundaries, stable result contracts, and the parity mapping that MUST
be surfaced consistently in documentation and tool inventory outputs.

This slice explicitly excludes `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`,
and it does not require broad renaming or removal of existing Corvus-native tool names.

## Requirements

### Requirement: Dedicated `Glob` Tool Contract

The system MUST expose a dedicated read-only tool named `Glob` for workspace-safe file pattern
discovery.

The `Glob` tool MUST accept:
- `pattern` as a required string glob pattern;
- `path` as an optional workspace-relative directory scope.

The `Glob` tool MUST reject requests when:
- `pattern` is empty;
- `path` is absolute, escapes the workspace, or resolves outside the active workspace boundary.

The `Glob` tool MUST return a stable structured result with:
- `filenames` as an array of workspace-relative paths;
- `durationMs` as a non-negative integer;
- `numFiles` as the total number of returned paths;
- `truncated` as a boolean indicating whether an internal result cap was applied.

Returned paths MUST be workspace-relative and MUST be ordered deterministically. The contract MAY
choose a deterministic ordering strategy such as modification-time ordering, but the chosen
ordering MUST remain stable for repeated runs against an unchanged workspace.

#### Scenario: `Glob` returns workspace-relative matches for a valid pattern

- GIVEN a workspace containing `src/main.ts` and `src/lib/util.ts`
- WHEN `Glob` is invoked with `{ "pattern": "src/**/*.ts" }`
- THEN the call MUST succeed
- AND `structured.filenames` MUST contain only workspace-relative paths
- AND `structured.numFiles` MUST equal the number of returned paths
- AND `structured.truncated` MUST be `false` when no cap was reached

#### Scenario: `Glob` rejects a path that escapes the workspace

- GIVEN an active workspace rooted at `/workspace`
- WHEN `Glob` is invoked with `{ "pattern": "**/*.rs", "path": "../.." }`
- THEN the call MUST fail validation
- AND the tool MUST NOT traverse outside `/workspace`

#### Scenario: `Glob` ordering is stable for an unchanged workspace

- GIVEN the same workspace contents and file metadata across repeated runs
- WHEN `Glob` is invoked repeatedly with the same input
- THEN the returned `structured.filenames` order MUST be identical on every run

### Requirement: Dedicated `Grep` Tool Contract

The system MUST expose a dedicated read-only tool named `Grep` for content search.

The `Grep` contract MUST remain parity-aligned with Claude-style search expectations while staying
behaviorally aligned with Corvus search semantics. The implementation MAY wrap the existing search
engine, but the exposed `Grep` contract MUST be the stable public surface for this slice.

The `Grep` tool MUST accept:
- `pattern` as a required search pattern string;
- `path` as an optional workspace-relative file or directory scope;
- `glob` as an optional include filter;
- `output_mode` as an optional enum with values `content`, `files_with_matches`, or `count`.

The `Grep` tool MAY accept optional context and search modifiers, but any accepted modifier MUST be
validated deterministically and MUST NOT widen filesystem access beyond the workspace boundary.

The `Grep` tool MUST reject requests when:
- `pattern` is empty;
- `path` is absolute, escapes the workspace, or resolves outside the workspace;
- context-only fields are supplied in a combination that the declared `output_mode` does not allow;
- numeric pagination or context fields are negative.

The `Grep` tool MUST return a stable structured result that includes:
- `mode` as the effective output mode;
- `numFiles` as a non-negative integer;
- `filenames` as workspace-relative paths in deterministic order;
- `content` when `mode = "content"`;
- `numLines` when `mode = "content"`;
- `numMatches` when `mode = "count"`;
- `appliedLimit` and `appliedOffset` when relevant.

When `mode = "content"`, returned content MUST preserve deterministic ordering for an unchanged
workspace and MUST remain bounded by the tool's documented output limits.

#### Scenario: `Grep` returns file matches in a deterministic public contract

- GIVEN a workspace where `src/app.ts` and `src/lib.ts` both contain the text `SearchClient`
- WHEN `Grep` is invoked with `{ "pattern": "SearchClient", "output_mode": "files_with_matches" }`
- THEN the call MUST succeed
- AND `structured.mode` MUST equal `files_with_matches`
- AND `structured.filenames` MUST contain only workspace-relative file paths
- AND the filenames MUST appear in deterministic order across repeated runs on the same workspace

#### Scenario: `Grep` rejects invalid output mode combinations

- GIVEN a valid workspace
- WHEN `Grep` is invoked with `{ "pattern": "needle", "output_mode": "count", "-A": 2 }`
- THEN the call MUST fail validation
- AND the failure MUST explain that content context fields are only valid with content output

#### Scenario: `Grep` cannot search outside the workspace

- GIVEN an active workspace rooted at `/workspace`
- WHEN `Grep` is invoked with `{ "pattern": "token", "path": "/etc" }`
- THEN the call MUST fail validation or permission checks
- AND the tool MUST NOT read files outside `/workspace`

#### Scenario: `Grep` preserves zero-match success semantics

- GIVEN a workspace with no files containing `pattern_that_does_not_exist_536`
- WHEN `Grep` is invoked with `{ "pattern": "pattern_that_does_not_exist_536", "output_mode": "count" }`
- THEN the call MUST succeed
- AND the structured result MUST report zero matches
- AND the result shape MUST still match the documented contract for `count` mode

### Requirement: Dedicated Read-Only `WebFetch` Tool Contract

The system MUST expose a dedicated tool named `WebFetch` for read-only fetch-and-extract flows.

The `WebFetch` tool MUST accept:
- `url` as a required absolute URL string;
- `prompt` as a required instruction string describing how fetched content should be summarized or
  transformed for the caller.

`WebFetch` MUST be read-only. It MUST NOT expose mutation semantics for remote resources, and it
MUST NOT be used as a generic arbitrary HTTP write surface.

`WebFetch` MUST preserve the same effective outbound network security boundary required for current
allowlisted HTTP access, including:
- host allowlist enforcement when configured;
- private-host and local-network protections;
- rejection of unsupported or unsafe URL schemes.

The `WebFetch` tool MUST return a stable structured result with:
- `bytes` as the fetched response size in bytes when available;
- `code` as the HTTP status code;
- `codeText` as the status text when available;
- `result` as the extracted or summarized response content;
- `durationMs` as a non-negative integer;
- `url` as the final fetched URL represented to the caller.

If the fetch is denied by policy, the tool MUST fail without making the prohibited network request.

#### Scenario: `WebFetch` returns extracted content for an allowlisted URL

- GIVEN outbound policy permits `https://docs.example.com/page`
- WHEN `WebFetch` is invoked with `{ "url": "https://docs.example.com/page", "prompt": "Summarize the key API limits" }`
- THEN the call MUST succeed
- AND the structured result MUST include `code`, `result`, `durationMs`, and `url`
- AND the tool MUST remain read-only for that request

#### Scenario: `WebFetch` rejects a private-network target

- GIVEN the runtime blocks private-network destinations
- WHEN `WebFetch` is invoked with `{ "url": "http://127.0.0.1:8080/admin", "prompt": "Summarize" }`
- THEN the call MUST be denied
- AND the runtime MUST NOT fetch the target resource

#### Scenario: `WebFetch` rejects an unsupported URL scheme

- GIVEN a valid runtime session
- WHEN `WebFetch` is invoked with `{ "url": "file:///etc/passwd", "prompt": "Summarize" }`
- THEN the call MUST fail validation
- AND the tool MUST NOT treat local file reads as web fetches

### Requirement: Tool Inventory and Surfaced Listing Compatibility

The system MUST keep surfaced tool listings consistent with the first-slice parity contracts.

Any surfaced runtime tool inventory relevant to operators or agents, including `/tools` or other
runtime-exposed tool listings, MUST represent `Glob`, `Grep`, and `WebFetch` as available tools
when they are enabled for the current profile.

Such listings MUST remain backward compatible for this slice. They MUST NOT require removal of
existing Corvus-native tool names from the runtime, and they MUST distinguish parity-facing names
from legacy or native names clearly enough that operators can understand what is canonical in this
transition slice.

If a profile or permission context disables one of the parity tools, surfaced listings MUST reflect
that effective availability rather than advertising unavailable tools.

#### Scenario: `/tools` inventory shows enabled parity tools

- GIVEN the active runtime profile enables `Glob`, `Grep`, and `WebFetch`
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST include `Glob`, `Grep`, and `WebFetch`
- AND each listed tool MUST use the same names defined by this specification

#### Scenario: surfaced inventory does not advertise disabled parity tools

- GIVEN the active runtime profile disables `WebFetch`
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST NOT claim that `WebFetch` is available
- AND the listing MUST remain internally consistent with the runtime's actual tool allowlist

### Requirement: Published Parity Mapping and Scope Boundary Documentation

The change MUST publish a parity mapping between Corvus-native tool names and Claude-style tool
names for the first slice.

The published mapping MUST, at minimum, document the relationship between:
- `code_search` and `Grep`;
- `http_request` and `WebFetch`;
- Corvus file-discovery capability and `Glob`.

The mapping MUST identify whether each parity name is additive, canonical for parity-facing
surfaces, legacy/native, or deferred for future consolidation.

The same documentation set MUST also state that:
- `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` are explicitly deferred from
  this slice; and
- this slice does not require broad rename or removal of existing internal tool names.

#### Scenario: parity mapping documentation distinguishes parity and native names

- GIVEN a maintainer reads the parity documentation for this change
- WHEN they inspect the mapping table
- THEN they MUST be able to identify which Corvus-native tool or capability backs `Glob`, `Grep`, and `WebFetch`
- AND they MUST be able to tell whether each mapping is additive or a future consolidation candidate

#### Scenario: documentation explicitly defers task tools

- GIVEN a maintainer or operator reviews the first-slice parity documentation
- WHEN they look for `TaskCreate` and related task lifecycle tools
- THEN the documentation MUST state that `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` are out of scope for this slice
- AND it MUST NOT imply that `schedule` or `cron_*` already satisfy that task-tool contract
