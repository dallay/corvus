//! Corvus Providers Registry
//!
//! Re-exports provider types and provides registry functions.

pub use corvus_traits::providers::Provider;

/// Information about a provider.
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub local: bool,
}

// Re-export types
pub use corvus_traits::providers::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, StreamChunk, StreamOptions,
    ToolCall, ToolResultMessage,
};
