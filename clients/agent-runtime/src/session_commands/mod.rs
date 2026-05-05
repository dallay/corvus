//! Slash command platform for runtime ingress.
//!
//! This module owns the runtime slash-command extension point. New slash
//! commands define a [`SlashCommandDescriptor`] plus a [`SlashCommandHandler`]
//! and are registered through [`SlashCommandRegistry`]. Ingress surfaces parse
//! raw prompts, then delegate lookup, alias resolution, requirement checks, and
//! handler dispatch to the registry instead of matching command names locally.

pub mod parser;
pub mod registry;
pub mod service;
pub mod types;

#[allow(unused_imports)]
pub use parser::SessionCommandParser;
#[allow(unused_imports)]
pub use registry::{default_registry, SlashCommandRegistry};
pub use service::SessionCommandService;
#[allow(unused_imports)]
pub use types::{
    CommandBackend, CommandCaller, CommandCapability, CommandContext, CommandContextFacts,
    CommandIngressContext, CommandIngressSource, CommandPermission, CommandSessionContext,
    CommandSessionSource, RawSlashInvocation, SessionCommandFailure, SessionCommandFailureKind,
    SessionCommandHelpEntry, SessionCommandInspectGap, SessionCommandInspectGapCode,
    SessionCommandInspectSessionRecord, SessionCommandInspectSnapshot,
    SessionCommandInspectSnapshotSlot, SessionCommandInspectSnapshots,
    SessionCommandInspectStateRecord, SessionCommandOutcome, SessionCommandSessionInspect,
    SessionCommandSessionStatus, SessionCommandSuccess, SessionCommandSuccessData,
    SessionCommandToolEntry, SessionCommandToolSourceKind, SlashCommandArgumentShape,
    SlashCommandDescriptor, SlashCommandHandler, SlashCommandRegistration,
    SlashCommandRequirements, SlashInvocation, SlashRegistryError,
};
