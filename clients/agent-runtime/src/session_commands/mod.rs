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
    CommandContext, RawSlashInvocation, SessionCommandError, SessionCommandResult,
    SlashCommandArgumentShape, SlashCommandDescriptor, SlashCommandHandler,
    SlashCommandRegistration, SlashCommandRequirements, SlashInvocation, SlashRegistryError,
};
