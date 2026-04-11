//! Compatibility shim for extracted provider contracts.

pub use corvus_traits::providers::{
    build_tool_instructions_text, ChatMessage, ChatRequest, ChatResponse, ConversationMessage,
    Provider, ProviderCapabilities, StreamChunk, StreamError, StreamOptions, StreamResult,
    ToolCall, ToolResultMessage, ToolsPayload,
};
