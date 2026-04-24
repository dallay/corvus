# Delta for rook-tui

## MODIFIED Requirements

### Requirement: TUI Navigation Is Bounded to the #595 Read-Only Slice

The TUI MUST provide a clear first-level navigation, view selection, or equivalent terminal affordance
that allows the operator to reach the implemented read-only views for the bounded Rook TUI surface:

- status
- providers
- pools
- health
- routes

For #597, the TUI MUST formalize its read-only boundary by guiding operators to the web dashboard for setup, mutations, and troubleshooting workflows. It MUST explicitly remove placeholders that promise future TUI-based setup features.

(Previously: The TUI navigation requirement kept troubleshooting/setup flows and mutation workflows explicitly deferred to follow-up changes.)

#### Scenario: logs and mutations are explicitly bridged to the web dashboard

- GIVEN the Rook TUI is running
- WHEN the operator inspects available views and actions
- THEN the TUI MUST NOT present recent logs, troubleshooting/setup, or repair workflows as implemented terminal views
- AND the TUI MUST explicitly display the Web Dashboard URL as the required destination for setup, mutation, and advanced troubleshooting
- AND all "deferred to #597" messaging MUST be removed.

### Requirement: View States Stay Scoped to the Active TUI View

The status, providers, pools, health, and routes views MUST each handle loading, empty, and error states natively without replacing the overall shell or blanking out sibling views.

(Previously: This requirement explicitly deferred setup workflows.)

#### Scenario: setup explicitly directed to web dashboard

- GIVEN one TUI view depends on a verified read request that returns empty (e.g., no accounts configured)
- WHEN the operator reads the empty state or the main shell
- THEN the TUI MUST inform the operator to perform setup via the web dashboard.

## REMOVED Requirements

### Requirement: Deferred Workflows and Mutations Remain Explicitly Out of Scope

*(This entire requirement is replaced by the modified bridging scenario above. The TUI is no longer "deferring" these features to a future TUI slice; it is permanently delegating them to the web dashboard).*
