//! Routing engine — request-time selection of provider accounts.
//!
//! Implements the selection strategies defined in [`crate::domain::SelectionStrategy`]:
//! Priority, RoundRobin, Weighted, and Failover. Also handles health-aware
//! resolution and automatic fallback through pool chains.
//!
//! FIXME: implement `RoutingEngine` struct with strategy dispatch.
//! FIXME: wire cooldown state from health monitor into account eligibility.
//! FIXME: integrate with [`crate::registry`] for live pool/account lookups.
