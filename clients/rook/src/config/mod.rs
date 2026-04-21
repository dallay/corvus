//! Config — Rook-specific configuration loading and validation.
//!
//! Owns the `RookConfig` struct and its TOML/env loading logic. Intentionally
//! separate from the `corvus` binary's `Config` type — Rook has its own
//! independent configuration schema and file path (`~/.config/rook/config.toml`
//! by default).
//!
//! FIXME: implement `RookConfig` struct with gateway, registry, and TUI sections.
//! FIXME: add env-var overrides (`ROOK_*` prefix).
//! FIXME: add `rook config validate` to the CLI once struct is stable.
