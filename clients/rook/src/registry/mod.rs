//! Registry — persistence layer for Rook domain objects.
//!
//! Owns all SQLite read/write operations for [`ProviderAccount`],
//! [`ProviderPool`], [`ModelRoute`], and [`RoutingPolicy`].
//!
//! Consumers (gateway, TUI, dashboard) interact with higher-level service
//! types; they must never call SQLite directly.
//!
//! FIXME: implement CRUD operations backed by rusqlite.
//! FIXME: add migration runner (embed SQL migrations via `include_str!`).
