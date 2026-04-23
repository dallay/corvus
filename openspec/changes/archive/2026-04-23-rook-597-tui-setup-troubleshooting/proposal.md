# Proposal: TUI Setup and Troubleshooting Finalization

## Change

`rook-597-tui-setup-troubleshooting`

## Why

Issue #597 calls for "TUI setup and troubleshooting workflows". However, the current Rook TUI established in #595 and #596 is a strictly read-only, flat observability surface. It lacks text inputs, focus management, forms, and mutation infrastructure. 

At the same time, the embedded web dashboard (built in #592 and #593) already provides a rich, verified surface for account setup, credential management, pool administration, and route configuration. Building an equivalent text-based UI (TUI) form system for setup duplicates effort, increases maintenance burden, and deviates from the primary goal of the terminal output: fast, read-only situational awareness.

Therefore, the best way to handle "setup and troubleshooting" in the terminal is to bridge the operator directly to the web dashboard, providing clear instructions and deep links rather than rebuilding complex forms in `ratatui`.

## What Changes

This change formally bounds the TUI as a read-only observability surface and handles #597 by providing explicit navigational bridges and troubleshooting guidance back to the web dashboard.

The slice will:
- Update the TUI footer/shell to display the web dashboard local URL (e.g., `http://localhost:3000` or equivalent bound address), clarifying that setup and mutations happen there.
- Replace the current "deferred to #597" footer messages with a clear bridge message: `Setup and mutations are managed in the web dashboard: http://localhost:<port>`.
- Keep the TUI flat, read-only, and fast.

## In Scope
- Removing `#597` deferral messages from `clients/rook/src/tui/app.rs`.
- Adding the dashboard URL to the TUI header or footer to guide operators.
- Ensuring the TUI remains strictly read-only.
- Updating tests to reflect these messaging changes.

## Out of Scope
- TUI forms, modals, or text inputs.
- TUI-based setup flows (e.g., typing API keys in the terminal).
- Complex interactive troubleshooting trees.

## Verified Dependencies
- The web dashboard is already fully capable of handling setup and mutations (#592, #593).
- The HTTP server port is known at runtime or can be derived from the config/environment.

## Expected Outcome
After this change, the TUI will be considered "complete" for the M2 milestone. Operators looking to set up providers, pools, or troubleshoot complex states will be explicitly guided by the TUI to open the web dashboard, keeping the terminal experience fast, safe, and read-only.
