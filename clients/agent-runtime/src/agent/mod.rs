#[allow(clippy::module_inception)]
pub mod agent;
pub mod classifier;
pub mod code_session;
pub mod coordinator;
pub mod dispatcher;
pub mod mailbox;
pub mod memory_loader;
pub mod mission;
pub mod prompt;
pub mod unified_entrypoint;
pub mod unified_loop;
pub(crate) mod validation;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use agent::{
    run, Agent, AgentBuilder, AgentExecutionError, AgentTurnEvent, AgentTurnOutcome,
    AgentTurnResult, TurnContext,
};
