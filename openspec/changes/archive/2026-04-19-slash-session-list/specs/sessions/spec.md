# Delta for sessions

## MODIFIED Requirements

### Requirement: Session Discoverability Root Help

The system MUST treat `/session` with empty raw arguments as a read-only discoverability entry point for session commands in this slice.

The response MUST describe `/session` root usage and the supported `/session` family forms for this slice. It MUST identify `/session status` as the compact current-session summary view, it MUST identify `/session inspect` as the richer current-session inspection view, and it MUST identify `/session list` as the caller-scoped accessible-session listing view. The response MAY mention adjacent slash-session lifecycle commands such as `/resume`, `/suspend`, `/compact`, and `/tldr`, but it MUST distinguish them from `/session` subcommands. `/session` root help MUST remain the family help or usage hub and MUST NOT create, update, suspend, resume, compact, summarize, inspect, list beyond the current caller scope, or otherwise mutate session records or slash-session state.

(Previously: Root help identified `/session status` and `/session inspect` as the supported `/session` family forms in this slice, but it did not include `/session list`.)

#### Scenario: Root help includes `/session list` without mutation

- GIVEN a current session context exists
- WHEN the user runs `/session`
- THEN the system MUST return read-only help or usage guidance for the `/session` family
- AND the guidance MUST include `/session status` as the compact summary view
- AND the guidance MUST include `/session inspect` as the richer inspection view
- AND the guidance MUST include `/session list` as the accessible-session listing view
- AND no session lifecycle or snapshot state MUST be modified.

## ADDED Requirements

### Requirement: Caller-Scoped Session List Discoverability

The system MUST make `/session list` a read-only discoverability view over sessions accessible to the current caller scope represented by the typed slash command execution context.

The `/session` handler and service boundary MUST preserve sufficient caller-scope context for `/session list` so the visibility contract can be enforced explicitly rather than inferred from only the current session identifier. `/session list` MUST list only sessions accessible to that current caller scope, and it MUST NOT broaden visibility to admin, global, or cross-scope session inventory. If the runtime cannot establish or preserve sufficient caller-scope facts for this authorization-sensitive listing operation, it MUST return an explicit denial or unsupported outcome instead of broadening visibility.

The `/session list` result MUST be ordered by `last_activity DESC`. When two or more visible sessions share the same `last_activity`, the system MUST apply a stable secondary ordering rule so repeated executions over unchanged authoritative data return the same row order.

The structured row contract for `/session list` MUST contain only these fields: `id`, `last_activity`, `lifecycle`, and `resumable`. In that contract, `id` MUST identify the listed session, `last_activity` MUST reflect the authoritative last-activity timestamp used for ordering, `lifecycle` MUST reflect the authoritative slash-session lifecycle classification for that session, and `resumable` MUST indicate whether that session currently has authoritative resume-capable state available for resume. The command MUST return balanced output consisting of concise human-readable summary text plus structured row data derived from the same authoritative listing model so both views remain consistent. `/session list` MUST remain read-only and MUST NOT require or accept target-session arguments, filters, search, pagination, attach, switch, delete, resume, suspend, or any other mutation behavior. It MUST NOT expose rich row metadata beyond the minimal row contract.

#### Scenario: Session list returns only caller-visible rows in deterministic order

- GIVEN authoritative session records include `sess-a`, `sess-b`, and `sess-c`
- AND the typed execution context represents a caller scope authorized to view only `sess-a` and `sess-c`
- AND `sess-c` has more recent `last_activity` than `sess-a`
- WHEN the user runs `/session list`
- THEN the system MUST return only `sess-c` and `sess-a`
- AND the rows MUST be ordered by `last_activity DESC`
- AND each structured row MUST include only `id`, `last_activity`, `lifecycle`, and `resumable`.

#### Scenario: Stable tiebreaker preserves repeated ordering for equal activity timestamps

- GIVEN the current caller scope is authorized to view sessions `sess-a` and `sess-b`
- AND both sessions have the same authoritative `last_activity` value
- WHEN the user runs `/session list` multiple times without any underlying session changes
- THEN the system MUST return `sess-b` and `sess-a` in the same relative order on each execution
- AND that ordering MUST be produced by the stable secondary ordering rule `id DESC`.

#### Scenario: Missing caller-scope context does not broaden visibility

- GIVEN `/session list` is invoked on a surface where sufficient caller-scope facts are unavailable at the `/session` handler or service boundary
- WHEN the runtime evaluates the authorization-sensitive listing request
- THEN the system MUST return an explicit denial or unsupported outcome
- AND the system MUST NOT fall back to listing all sessions or an implementation-defined wider scope.

#### Scenario: Empty caller-visible set still returns balanced read-only output

- GIVEN authoritative session records exist
- AND the typed execution context represents a caller scope that is authorized to view none of them
- WHEN the user runs `/session list`
- THEN the system MUST return a read-only success result with a human-readable empty-state summary
- AND the structured result MUST contain zero rows
- AND no session lifecycle or snapshot state MUST be modified.
