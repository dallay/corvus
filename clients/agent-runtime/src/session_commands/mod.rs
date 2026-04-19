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
    SessionCommandHelpEntry, SessionCommandOutcome, SessionCommandSessionStatus,
    SessionCommandSuccess, SessionCommandSuccessData,
    SessionCommandToolEntry, SessionCommandToolSourceKind, SlashCommandArgumentShape,
    SlashCommandDescriptor, SlashCommandHandler, SlashCommandRegistration,
    SlashCommandRequirements, SlashInvocation, SlashRegistryError,
};
