//! Corvus provider registry surfaces for manifest composition.

pub mod factory;
pub mod registry;

pub use corvus_traits::providers::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider, StreamChunk,
    StreamOptions, ToolCall, ToolResultMessage,
};
pub use factory::{select_provider, ProviderFactorySelection};
pub use registry::{
    list_providers, provider_availability, resolve_provider_key, CapabilityAvailability,
    ProviderDescriptor,
};
