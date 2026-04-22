//! Shared service layer consumed by the HTTP gateway, dashboard, and TUI.
//!
//! All business logic lives here so surfaces stay thin; they only translate
//! between their own representations and the domain types.

pub mod account;
pub mod health;
pub mod idempotency;
pub mod pool;
pub mod route;
pub mod settings;
