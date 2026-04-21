//! TUI — operator terminal interface.
//!
//! Provides a real-time view of the gateway state: active accounts,
//! pool health, routing decisions, and live request metrics. Operators
//! can enable/disable accounts and trigger manual failovers without
//! leaving the terminal.
//!
//! FIXME: implement TUI with `ratatui` (add dep when this module is built out).
//! FIXME: wire live state from routing engine and registry.
//! FIXME: add keyboard shortcut layer for account management actions.
