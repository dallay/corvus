//! Dashboard — embedded admin UI and management API.
//!
//! Serves static dashboard assets and an admin REST API for managing
//! provider accounts, pools, and routes at runtime without restarting
//! the gateway process.
//!
//! FIXME: embed dashboard build artefacts via `include_dir!` or `rust-embed`.
//! FIXME: implement admin API handlers (CRUD for registry entities).
//! FIXME: add basic auth/token guard on all `/admin/*` routes.
